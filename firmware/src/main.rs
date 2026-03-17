#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::{Stack, StackResources, DhcpConfig};
use embassy_net_wiznet::chip::W5500;
use embassy_net_wiznet::*;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::peripherals::SPI0;
use embassy_rp::peripherals;
use embassy_rp::spi::{Async, Config as SpiConfig, Spi};
use embassy_rp::watchdog::*;
use embassy_time::{Delay, Duration, Ticker, Instant};
use embedded_hal_bus::spi::ExclusiveDevice;
//use embedded_io_async::Write;
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
use assign_resources::assign_resources;

mod mdns;
mod config;
mod crc8_ccitt;
mod http;

use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::blocking_mutex::Mutex;

use nom::IResult;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::space1;
use nom::sequence::separated_pair;
use nom::sequence::pair;
use nom::sequence::preceded;
use nom::number::complete::float;
use nom::combinator::{rest, opt};
use nom::Parser;

const FLASH_SIZE:usize = 2 * 1024 * 1024;
static FLASH_UUID: Mutex<ThreadModeRawMutex, RefCell<[u8;8]>> = Mutex::new(RefCell::new([0u8; 8]));

use git_version::git_version;
const PRODUCT_NAME:&str = "G-5500 HamLib Adaptor - Phil Crump M0DNY";
const GIT_VERSION:&str = git_version!(args = ["--dirty", "--always"], fallback = "nogit");

// Watchdog period needs to include max dhcp config duration
const WATCHDOG_PERIOD_MS:u64 = 8300; // Max is 8388ms
static WATCHDOG_RESET_SYSTEM: Mutex<ThreadModeRawMutex, RefCell<bool>> = Mutex::new(RefCell::new(false));

const DHCP_HOSTNAME:&str = "g5500-hamlib-adaptor";
const DHCP_TIMEOUT_MS:u64 = 5000;

const NUMBER_HAMLIB_SOCKETS:u16 = 4;
const SOCKET_TIMEOUT_S:u64 = 60;

static SOCKETS_CONNECTED: Mutex<ThreadModeRawMutex, RefCell<u16>> = Mutex::new(RefCell::new(0));
static NETWORK_CONNECTED: Mutex<ThreadModeRawMutex, RefCell<bool>> = Mutex::new(RefCell::new(false));

const PARK_AZ_DEGREES:f32 = 180.0;
const PARK_EL_DEGREES:f32 = 0.0;

const CONTROL_DEGREES_THRESHOLD:f32 = 3.0;
const CONTROL_AZ_DEGREES_MAXIMUM:f32 = 450.0;
const CONTROL_EL_DEGREES_MAXIMUM:f32 = 180.0;

static CURRENT_AZ_EL_RAW: Mutex<ThreadModeRawMutex, RefCell<(f32, f32)>> = Mutex::new(RefCell::new((0.0, 0.0)));
static CURRENT_AZ_EL_DEGREES: Mutex<ThreadModeRawMutex, RefCell<(f32, f32)>> = Mutex::new(RefCell::new((0.0, 0.0)));
static DEMAND_RUN_AZ_EL_DEGREES: Mutex<ThreadModeRawMutex, RefCell<(bool, f32, f32)>> = Mutex::new(RefCell::new((false, 0.0, 0.0)));

static STORED_CONFIG: Mutex<ThreadModeRawMutex, RefCell<Option<config::Config>>> = Mutex::new(RefCell::new(None));

static CONFIG_SAVE_PENDING: Mutex<ThreadModeRawMutex, RefCell<bool>> = Mutex::new(RefCell::new(false));

assign_resources! {
    azel_adc: AzElAdc {
        adc: ADC,
        dma: DMA_CH2,
        pin_az: PIN_26,
        pin_el: PIN_27
    },
    azel_control: AzElControl {
        pin_az_cw: PIN_2,
        pin_az_ccw: PIN_3,
        pin_el_up: PIN_4,
        pin_el_dn: PIN_5
    }
}

// W5500 Driver
#[embassy_executor::task]
async fn ethernet_task(
    runner: Runner<
        'static,
        W5500,
        ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, Delay>,
        Input<'static>,
        Output<'static>,
    >,
) -> ! {
    runner.run().await
}

// Network Driver
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device<'static>>) -> ! {
    runner.run().await
}

// mDNS Responder Task
#[embassy_executor::task]
async fn mdns_task(stack: Stack<'static>) -> ! {
    mdns::mdns_responder(stack, DHCP_HOSTNAME, 4533).await
}

// HTTP Configuration Server Task
#[embassy_executor::task]
async fn http_task(stack: Stack<'static>) -> ! {
    http::http_server(stack).await
}

// System LED Blink Task
// Flashes every 1 second while waiting for network, every 0.5 seconds when connected
#[embassy_executor::task]
async fn led_blink_task(mut led: Output<'static>) {
    loop {
        let is_connected = NETWORK_CONNECTED.lock(|f| *f.borrow());
        let interval = if is_connected {
            Duration::from_millis(500)  // Fast blink when connected
        } else {
            Duration::from_millis(1000) // Slow blink when waiting
        };
        
        led.toggle();
        embassy_time::Timer::after(interval).await;
    }
}

// AzEL Control Driver
#[embassy_executor::task]
async fn control_task(_spawner: Spawner, r: AzElControl) {
    let mut az_cw = Output::new(r.pin_az_cw, Level::Low);
    let mut az_ccw = Output::new(r.pin_az_ccw, Level::Low);
    let mut el_up = Output::new(r.pin_el_up, Level::Low);
    let mut el_dn = Output::new(r.pin_el_dn, Level::Low);

    let mut local_current_az_degrees:f32;
    let mut local_current_el_degrees:f32;

    let mut local_demand_run:bool;
    let mut local_demand_az_degrees:f32;
    let mut local_demand_el_degrees:f32;

    let mut flag_ontarget:bool;

    let mut ticker = Ticker::every(Duration::from_millis(250));
    loop
    {
        (local_demand_run, local_demand_az_degrees, local_demand_el_degrees) = DEMAND_RUN_AZ_EL_DEGREES.lock(|f| *f.borrow());

        if local_demand_run
        {
            (local_current_az_degrees, local_current_el_degrees) = CURRENT_AZ_EL_DEGREES.lock(|f| *f.borrow());
            flag_ontarget = true;

            // Azimuth
            if local_current_az_degrees < (local_demand_az_degrees - CONTROL_DEGREES_THRESHOLD)
            {
                az_ccw.set_low();
                az_cw.set_high();
                flag_ontarget = false;
            }
            else if local_current_az_degrees > (local_demand_az_degrees + CONTROL_DEGREES_THRESHOLD)
            {
                az_cw.set_low();
                az_ccw.set_high();
                flag_ontarget = false;
            }
            else
            {
                az_cw.set_low();
                az_ccw.set_low();
            }

            // Elevation
            if local_current_el_degrees < (local_demand_el_degrees - CONTROL_DEGREES_THRESHOLD)
            {
                el_dn.set_low();
                el_up.set_high();
                flag_ontarget = false;
            }
            else if local_current_el_degrees > (local_demand_el_degrees + CONTROL_DEGREES_THRESHOLD)
            {
                el_up.set_low();
                el_dn.set_high();
                flag_ontarget = false;
            }
            else
            {
                el_up.set_low();
                el_dn.set_low();
            }

            if flag_ontarget
            {
                // Reset RUN to stopped to prevent more driving
                DEMAND_RUN_AZ_EL_DEGREES.lock(|f| {
                    let (_, az, el) = *f.borrow();
                    f.replace((false, az, el));
                });
            }
        }
        else
        {
            // Stopped (by command, or on target)
            az_cw.set_low();
            az_ccw.set_low();
            el_up.set_low();
            el_dn.set_low();
        }

        ticker.next().await;
    }

}

use embassy_rp::adc::{Adc, Channel, Config, InterruptHandler};
use embassy_rp::bind_interrupts;

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

struct CalParams {
    az_slope: f32,
    az_intercept: f32,
    el_slope: f32,
    el_intercept: f32,
    az_offset: f32,
    el_offset: f32,
    use_cal_az: bool,
    use_cal_el: bool,
}

impl CalParams {
    fn from_config(cfg: &config::Config) -> Self {
        let use_cal_az = cfg.calibration_valid
            && (cfg.az_raw_high - cfg.az_raw_low).abs() >= 100.0
            && cfg.az_deg_high != cfg.az_deg_low;
        let use_cal_el = cfg.calibration_valid
            && (cfg.el_raw_high - cfg.el_raw_low).abs() >= 100.0
            && cfg.el_deg_high != cfg.el_deg_low;

        let (az_slope, az_intercept) = if use_cal_az {
            let slope = (cfg.az_deg_high - cfg.az_deg_low) / (cfg.az_raw_high - cfg.az_raw_low);
            (slope, cfg.az_deg_low - slope * cfg.az_raw_low)
        } else {
            (0.0, 0.0)
        };

        let (el_slope, el_intercept) = if use_cal_el {
            let slope = (cfg.el_deg_high - cfg.el_deg_low) / (cfg.el_raw_high - cfg.el_raw_low);
            (slope, cfg.el_deg_low - slope * cfg.el_raw_low)
        } else {
            (0.0, 0.0)
        };

        CalParams {
            az_slope, az_intercept,
            el_slope, el_intercept,
            az_offset: cfg.az_cal_offset,
            el_offset: cfg.el_cal_offset,
            use_cal_az, use_cal_el,
        }
    }
}

// AzEl ADC Driver
#[embassy_executor::task]
async fn adc_task(_spawner: Spawner, r: AzElAdc) {

    // Either

    const UPPER_RESISTOR_K:f32 = 10.0;
    const LOWER_RESISTOR_K:f32 = 10.0;

    const LADDER_RATIO:f32 = LOWER_RESISTOR_K/(UPPER_RESISTOR_K+LOWER_RESISTOR_K);

    const VREF_V:f32 = 3.3;

    // Manuals say 2.0-4.5V
    //const G5500_VOLTAGE_LOW:f32 = 2.0;
    //const G5500_VOLTAGE_HIGH:f32 = 4.5;
    // Rotators are actually 0-5v
    const G5500_VOLTAGE_LOW:f32 = 0.0;
    const G5500_VOLTAGE_HIGH:f32 = 5.0;

    const ADC_RAW_AZ_LOW:f32 = G5500_VOLTAGE_LOW * LADDER_RATIO * (4096.0/VREF_V);
    const ADC_RAW_EL_LOW:f32 = G5500_VOLTAGE_LOW * LADDER_RATIO * (4096.0/VREF_V);

    const ADC_RAW_AZ_HIGH:f32 = G5500_VOLTAGE_HIGH * LADDER_RATIO * (4096.0/VREF_V);
    const ADC_RAW_EL_HIGH:f32 = G5500_VOLTAGE_HIGH * LADDER_RATIO * (4096.0/VREF_V);

    // or

    /*const ADC_RAW_AZ_0:u16 = ??;
    const ADC_RAW_AZ_360:u16 = ??;

    const ADC_RAW_EL_0:u16 = ??;
    const ADC_RAW_EL_180:u16 = ??;

    const ADC_RAW_AZ_LOW:f32 = ADC_RAW_AZ_0 as f32;
    const ADC_RAW_AZ_HIGH:f32 = ADC_RAW_AZ_360 as f32 * (450.0/360.0);

    const ADC_RAW_EL_LOW:f32 = ADC_RAW_EL_0 as f32;
    const ADC_RAW_EL_HIGH:f32 = ADC_RAW_EL_180 as f32;*/

    let mut adc = Adc::new(r.adc, Irqs, Config::default());
    let mut dma = r.dma;
    let mut pins = [
        Channel::new_pin(r.pin_az, Pull::None),
        Channel::new_pin(r.pin_el, Pull::None)
    ];

    const NUM_SAMPLES:usize = 512;
    // 512 samples * 2 to take approx 100ms, so 0.1ms/sample = 10kHz sample rate
    const DIV:u16 = 2399;
    let mut buf = [0_u16; { NUM_SAMPLES * 2 }];

    let mut az_sum:u32 ;
    let mut az_count:u32 ;
    let mut el_sum:u32 ;
    let mut el_count:u32;

    let mut candidate_az_raw:f32;
    let mut candidate_el_raw:f32;

    let mut candidate_az_degrees:f32;
    let mut candidate_el_degrees:f32;

    // Compute initial calibration parameters from stored config
    let mut cal = STORED_CONFIG.lock(|f| {
        match *f.borrow() {
            Some(ref cfg) => CalParams::from_config(cfg),
            None => CalParams::from_config(&config::Config::default()),
        }
    });

    let mut ticker = Ticker::every(Duration::from_millis(100));
    loop {
        adc.read_many_multichannel(&mut pins, &mut buf, DIV, &mut dma)
            .await
            .unwrap();

        // Count is used to allow us to skip samples if we hit the documented DNL spikes.
        az_sum = 0;
        az_count = 0;
        el_sum = 0;
        el_count = 0;

        // DNL spike values to skip
        const DNL_SPIKES: [u16; 4] = [512, 1536, 2560, 3584];
        
        for i in 0..NUM_SAMPLES
        {
            let az_val = buf[2*i];
            let el_val = buf[2*i + 1];
            
            // Azimuth - skip DNL spikes
            if !DNL_SPIKES.contains(&az_val) {
                az_sum += az_val as u32;
                az_count += 1;
            }

            // Elevation - skip DNL spikes
            if !DNL_SPIKES.contains(&el_val) {
                el_sum += el_val as u32;
                el_count += 1;
            }
        }

        // Azimuth
        candidate_az_raw = az_sum as f32 / az_count as f32;
        if cal.use_cal_az {
            candidate_az_degrees = (cal.az_slope * candidate_az_raw + cal.az_intercept + cal.az_offset).clamp(0.0, 450.0);
        } else {
            candidate_az_degrees = ((candidate_az_raw - ADC_RAW_AZ_LOW) / ((ADC_RAW_AZ_HIGH - ADC_RAW_AZ_LOW) / 450.0) + cal.az_offset).clamp(0.0, 450.0);
        }

        // Elevation
        candidate_el_raw = el_sum as f32 / el_count as f32;
        if cal.use_cal_el {
            candidate_el_degrees = (cal.el_slope * candidate_el_raw + cal.el_intercept + cal.el_offset).clamp(0.0, 180.0);
        } else {
            candidate_el_degrees = ((candidate_el_raw - ADC_RAW_EL_LOW) / ((ADC_RAW_EL_HIGH - ADC_RAW_EL_LOW) / 180.0) + cal.el_offset).clamp(0.0, 180.0);
        }

        // Update global current AzEl
        CURRENT_AZ_EL_DEGREES.lock(|f| {
            f.replace((candidate_az_degrees, candidate_el_degrees));
        });

        // Update global raw values
        CURRENT_AZ_EL_RAW.lock(|f| {
            f.replace((candidate_az_raw, candidate_el_raw));
        });

        // Re-read config each tick to pick up calibration changes
        cal = STORED_CONFIG.lock(|f| {
            match *f.borrow() {
                Some(ref cfg) => CalParams::from_config(cfg),
                None => CalParams::from_config(&config::Config::default()),
            }
        });

        ticker.next().await;
    }
}



#[derive(PartialEq, Eq)]
enum HamlibCommand {
    // Existing commands
    GetInfo,
    GetPos,
    Stop,
    Park,
    SetPos,
    Move,
    Quit,
    DumpState,
    DumpCaps,
    Reset,
    // New functional commands
    GetStatus,
    SetConf,
    GetConf,
    DumpConf,
    // Stub commands (RPRT -4)
    SetLevel,
    GetLevel,
    SetFunc,
    GetFunc,
    SetParm,
    GetParm,
    SendCmd,
    // Locator stubs (RPRT -4)
    Lonlat2Loc,
    Loc2Lonlat,
    Dms2Dec,
    Dec2Dms,
    Dmmm2Dec,
    Dec2Dmmm,
    Qrb,
    AzSp2AzLp,
    DistSp2DistLp,
    // Sentinel
    _None,
}

// Rotator move direction codes (from Hamlib rotator.h)
const ROT_MOVE_UP: u16 = 2;
const ROT_MOVE_DOWN: u16 = 4;
const ROT_MOVE_LEFT: u16 = 8;    // CCW
const ROT_MOVE_RIGHT: u16 = 16;  // CW
const ROT_MOVE_UP_LEFT: u16 = 32;    // UP + CCW
const ROT_MOVE_UP_RIGHT: u16 = 64;   // UP + CW
const ROT_MOVE_DOWN_LEFT: u16 = 128; // DOWN + CCW
const ROT_MOVE_DOWN_RIGHT: u16 = 256; // DOWN + CW

// Rotator status flags (from Hamlib rotator.h)
const ROT_STATUS_NONE: u32 = 0;
const ROT_STATUS_MOVING: u32 = 2; // (1 << 1)

// Config token enum for set_conf/get_conf
#[derive(PartialEq, Eq, Clone, Copy)]
enum ConfigToken {
    MinAz,
    MaxAz,
    MinEl,
    MaxEl,
    ParkAz,
    ParkEl,
}

impl ConfigToken {
    fn from_bytes(input: &[u8]) -> Option<Self> {
        match input {
            b"min_az" => Some(Self::MinAz),
            b"max_az" => Some(Self::MaxAz),
            b"min_el" => Some(Self::MinEl),
            b"max_el" => Some(Self::MaxEl),
            b"park_az" => Some(Self::ParkAz),
            b"park_el" => Some(Self::ParkEl),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq)]
struct Command {
    command_type: HamlibCommand
}
impl Command {
    // _, \get_info
    #[inline]
    fn parse_get_info(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("_"), tag("\\get_info")
        )).parse(input)
    }
    // p, \get_pos
    #[inline]
    fn parse_get_pos(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("p"), tag("\\get_pos")
        )).parse(input)
    }
    // S, \stop
    #[inline]
    fn parse_stop(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("S"), tag("\\stop")
        )).parse(input)
    }
    // K, \park
    #[inline]
    fn parse_park(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("K"), tag("\\park")
        )).parse(input)
    }
    // P 180.00 45.00, \set_pos 180.00 45.00
    #[inline]
    fn parse_set_pos(input: &[u8]) -> IResult<&[u8], (f32, f32)> {
        preceded(
            pair(
                alt((tag("P"), tag("\\set_pos"))),
                space1
            ),
            separated_pair(float, space1, float)
        ).parse(input)
    }
    // q, \quit
    #[inline]
    fn parse_quit(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("q"), tag("\\quit")
        )).parse(input)
    }
    // dump_state
    #[inline]
    fn parse_dump_state(input: &[u8]) -> IResult<&[u8], &[u8]> {
        tag("\\dump_state")(input)
    }
    // R [reset_type], \reset [reset_type] — optional integer argument
    #[inline]
    fn parse_reset(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, matched) = alt((
            tag("R"), tag("\\reset")
        )).parse(input)?;
        // Consume optional space + integer argument (ignored — always full reset)
        let _ = opt(pair(space1::<&[u8], nom::error::Error<&[u8]>>, rest)).parse(remaining);
        Ok((b"", matched))
    }
    // M DIR SPEED, \move DIR SPEED
    #[inline]
    fn parse_move(input: &[u8]) -> IResult<&[u8], (f32, f32)> {
        preceded(
            pair(
                alt((tag("M"), tag("\\move"))),
                space1
            ),
            separated_pair(float, space1, float)
        ).parse(input)
    }
    // 1, \dump_caps
    #[inline]
    fn parse_dump_caps(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("1"), tag("\\dump_caps")
        )).parse(input)
    }
    // s, \get_status
    #[inline]
    fn parse_get_status(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("s"), tag("\\get_status")
        )).parse(input)
    }
    // C TOKEN VALUE, \set_conf TOKEN VALUE
    #[inline]
    fn parse_set_conf(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
        let (remaining, _) = alt((tag("C"), tag("\\set_conf"))).parse(input)?;
        let (remaining, _) = space1(remaining)?;
        // Split remaining on first space: token value
        if let Some(pos) = remaining.iter().position(|&b| b == b' ') {
            let token = &remaining[..pos];
            let value = &remaining[pos + 1..];
            // Trim trailing whitespace/newlines from value
            let value = &value[..value.iter().rposition(|b| !b.is_ascii_whitespace()).map(|p| p + 1).unwrap_or(value.len())];
            Ok((b"", (token, value)))
        } else {
            // Token only, no value — trim trailing whitespace
            let token = &remaining[..remaining.iter().rposition(|b| !b.is_ascii_whitespace()).map(|p| p + 1).unwrap_or(remaining.len())];
            Ok((b"", (token, b"" as &[u8])))
        }
    }
    // \get_conf TOKEN (note: no single-char short form in Hamlib dispatch table)
    #[inline]
    fn parse_get_conf(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = tag("\\get_conf")(input)?;
        let (remaining, _) = space1(remaining)?;
        let token = &remaining[..remaining.iter().rposition(|b| !b.is_ascii_whitespace()).map(|p| p + 1).unwrap_or(remaining.len())];
        Ok((b"", token))
    }
    // 3, \dump_conf
    #[inline]
    fn parse_dump_conf(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((
            tag("3"), tag("\\dump_conf")
        )).parse(input)
    }
    // V, \set_level (stub — consume trailing args)
    #[inline]
    fn parse_set_level(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("V"), tag("\\set_level"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // v, \get_level (stub — consume trailing args)
    #[inline]
    fn parse_get_level(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("v"), tag("\\get_level"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // U, \set_func (stub — consume trailing args)
    #[inline]
    fn parse_set_func(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("U"), tag("\\set_func"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // u, \get_func (stub — consume trailing args)
    #[inline]
    fn parse_get_func(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("u"), tag("\\get_func"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // X, \set_parm (stub — consume trailing args)
    #[inline]
    fn parse_set_parm(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("X"), tag("\\set_parm"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // x, \get_parm (stub — consume trailing args)
    #[inline]
    fn parse_get_parm(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("x"), tag("\\get_parm"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // w, \send_cmd (stub — consume trailing args)
    #[inline]
    fn parse_send_cmd(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("w"), tag("\\send_cmd"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    // Locator stubs — each matches short + long form, consumes trailing args
    #[inline]
    fn parse_lonlat2loc(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("L"), tag("\\lonlat2loc"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_loc2lonlat(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("l"), tag("\\loc2lonlat"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_dms2dec(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("D"), tag("\\dms2dec"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_dec2dms(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("d"), tag("\\dec2dms"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_dmmm2dec(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("E"), tag("\\dmmm2dec"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_dec2dmmm(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("e"), tag("\\dec2dmmm"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_qrb(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("B"), tag("\\qrb"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_az_sp2az_lp(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("A"), tag("\\a_sp2a_lp"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse_dist_sp2dist_lp(input: &[u8]) -> IResult<&[u8], &[u8]> {
        let (remaining, _) = alt((tag("a"), tag("\\d_sp2d_lp"))).parse(input)?;
        let (remaining, _) = rest(remaining)?;
        Ok((remaining, b""))
    }
    #[inline]
    fn parse(input: &[u8]) -> (HamlibCommand, f32, f32, Option<(&[u8], &[u8])>) {
        // Try long-form commands first to avoid single-char prefix conflicts
        // (e.g., \get_pos before p, \set_pos before P, \stop before S, etc.)

        // Existing commands
        if Self::parse_get_info(input).is_ok() {
            return (HamlibCommand::GetInfo, 0.0, 0.0, None);
        }
        if let Ok((_, (az, el))) = Self::parse_set_pos(input) {
            return (HamlibCommand::SetPos, az, el, None);
        }
        if Self::parse_get_pos(input).is_ok() {
            return (HamlibCommand::GetPos, 0.0, 0.0, None);
        }
        if Self::parse_stop(input).is_ok() {
            return (HamlibCommand::Stop, 0.0, 0.0, None);
        }
        if Self::parse_park(input).is_ok() {
            return (HamlibCommand::Park, 0.0, 0.0, None);
        }
        if Self::parse_quit(input).is_ok() {
            return (HamlibCommand::Quit, 0.0, 0.0, None);
        }
        if Self::parse_dump_state(input).is_ok() {
            return (HamlibCommand::DumpState, 0.0, 0.0, None);
        }
        if Self::parse_reset(input).is_ok() {
            return (HamlibCommand::Reset, 0.0, 0.0, None);
        }
        if let Ok((_, (dir, speed))) = Self::parse_move(input) {
            return (HamlibCommand::Move, dir, speed, None);
        }
        if Self::parse_dump_caps(input).is_ok() {
            return (HamlibCommand::DumpCaps, 0.0, 0.0, None);
        }

        // New functional commands
        if Self::parse_get_status(input).is_ok() {
            return (HamlibCommand::GetStatus, 0.0, 0.0, None);
        }
        if let Ok((_, (token, value))) = Self::parse_set_conf(input) {
            return (HamlibCommand::SetConf, 0.0, 0.0, Some((token, value)));
        }
        if let Ok((_, token)) = Self::parse_get_conf(input) {
            return (HamlibCommand::GetConf, 0.0, 0.0, Some((token, b"" as &[u8])));
        }
        if Self::parse_dump_conf(input).is_ok() {
            return (HamlibCommand::DumpConf, 0.0, 0.0, None);
        }

        // Stub commands (RPRT -4)
        if Self::parse_set_level(input).is_ok() {
            return (HamlibCommand::SetLevel, 0.0, 0.0, None);
        }
        if Self::parse_get_level(input).is_ok() {
            return (HamlibCommand::GetLevel, 0.0, 0.0, None);
        }
        if Self::parse_set_func(input).is_ok() {
            return (HamlibCommand::SetFunc, 0.0, 0.0, None);
        }
        if Self::parse_get_func(input).is_ok() {
            return (HamlibCommand::GetFunc, 0.0, 0.0, None);
        }
        if Self::parse_set_parm(input).is_ok() {
            return (HamlibCommand::SetParm, 0.0, 0.0, None);
        }
        if Self::parse_get_parm(input).is_ok() {
            return (HamlibCommand::GetParm, 0.0, 0.0, None);
        }
        if Self::parse_send_cmd(input).is_ok() {
            return (HamlibCommand::SendCmd, 0.0, 0.0, None);
        }

        // Locator stubs (RPRT -4)
        if Self::parse_lonlat2loc(input).is_ok() {
            return (HamlibCommand::Lonlat2Loc, 0.0, 0.0, None);
        }
        if Self::parse_loc2lonlat(input).is_ok() {
            return (HamlibCommand::Loc2Lonlat, 0.0, 0.0, None);
        }
        if Self::parse_dms2dec(input).is_ok() {
            return (HamlibCommand::Dms2Dec, 0.0, 0.0, None);
        }
        if Self::parse_dec2dms(input).is_ok() {
            return (HamlibCommand::Dec2Dms, 0.0, 0.0, None);
        }
        if Self::parse_dmmm2dec(input).is_ok() {
            return (HamlibCommand::Dmmm2Dec, 0.0, 0.0, None);
        }
        if Self::parse_dec2dmmm(input).is_ok() {
            return (HamlibCommand::Dec2Dmmm, 0.0, 0.0, None);
        }
        if Self::parse_qrb(input).is_ok() {
            return (HamlibCommand::Qrb, 0.0, 0.0, None);
        }
        if Self::parse_az_sp2az_lp(input).is_ok() {
            return (HamlibCommand::AzSp2AzLp, 0.0, 0.0, None);
        }
        if Self::parse_dist_sp2dist_lp(input).is_ok() {
            return (HamlibCommand::DistSp2DistLp, 0.0, 0.0, None);
        }

        (HamlibCommand::_None, 0.0, 0.0, None)
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // System LED - Onboard LED on Raspberry Pi Pico (GPIO 25)
    let sys_led = Output::new(p.PIN_25, Level::Low);

    // Watchdog
    let mut watchdog = Watchdog::new(p.WATCHDOG);
    //watchdog.pause_on_debug(true); // Doesn't appear to work?
    watchdog.start(Duration::from_millis(WATCHDOG_PERIOD_MS));

    // Sockets LED - External LED (can be connected to any available GPIO)
    // Using PIN_15 as an example - adjust based on your hardware
    let mut sockets_led = Output::new(p.PIN_15, Level::Low);

    // Flash Driver
    let mut flash = embassy_rp::flash::Flash::<_, embassy_rp::flash::Async, FLASH_SIZE>::new(p.FLASH, p.DMA_CH3);
    let mut uid = [0; 8];
    flash.blocking_unique_id(&mut uid).unwrap();
    FLASH_UUID.lock(|f| {
        f.replace(uid);
    });

    // Load persistent config from flash
    let cfg = config::load_config(&mut flash);
    STORED_CONFIG.lock(|f| {
        f.replace(Some(cfg));
    });

    // Wiznet W5500 SPI Interface
    let mut spi_cfg = SpiConfig::default();
    spi_cfg.frequency = 50_000_000;
    let (miso, mosi, clk) = (p.PIN_16, p.PIN_19, p.PIN_18);
    let spi = Spi::new(p.SPI0, clk, mosi, miso, p.DMA_CH0, p.DMA_CH1, spi_cfg);
    let cs = Output::new(p.PIN_17, Level::High);
    let w5500_int = Input::new(p.PIN_21, Pull::Up);
    let w5500_reset = Output::new(p.PIN_20, Level::High);

    // Wiznet W5500 Driver
    let mac_addr = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
    static STATE: StaticCell<State<8, 8>> = StaticCell::new();
    let state = STATE.init(State::<8, 8>::new());
    let (device, runner) = embassy_net_wiznet::new(
        mac_addr,
        state,
        ExclusiveDevice::new(spi, cs, Delay),
        w5500_int,
        w5500_reset,
    )
    .await
    .unwrap();
    unwrap!(spawner.spawn(ethernet_task(runner)));

    // Network Random Seed
    let mut rng = RoscRng;
    let seed = rng.next_u64();

    // DHCP Config
    let mut dhcp_config = DhcpConfig::default();
    dhcp_config.hostname = Some(heapless::String::try_from(DHCP_HOSTNAME).unwrap());
    let net_config = embassy_net::Config::dhcpv4(dhcp_config);

    // SmolTCP network driver
    static RESOURCES: StaticCell<StackResources<7>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        device,
        net_config,
        RESOURCES.init(StackResources::new()),
        seed,
    );
    unwrap!(spawner.spawn(net_task(runner)));

    // Az & El ADC Driver
    let r = split_resources!(p);
    spawner.spawn(adc_task(spawner, r.azel_adc)).unwrap();

    // Az & El Control Driver
    spawner.spawn(control_task(spawner, r.azel_control)).unwrap();

    // System LED Blink Task
    spawner.spawn(led_blink_task(sys_led)).unwrap();

    // Feed watchdog either side of DHCP task
    watchdog.feed();

    info!("Waiting for DHCP...");
    let dhcp_result = wait_for_dhcp_config(stack).await;
    
    // Feed watchdog either side of DHCP task
    watchdog.feed();

    // Network setup: try DHCP first, fall back to static IP if configured
    let mut network_up = false;

    if let Some(cfg) = dhcp_result {
        let local_addr = cfg.address.address();
        info!("DHCP successful - IP address: {:?}", local_addr);
        network_up = true;
    } else {
        warn!("DHCP failed after {}ms timeout", DHCP_TIMEOUT_MS);

        // Check if static IP fallback is configured
        let static_cfg = STORED_CONFIG.lock(|f| f.borrow().clone());
        if let Some(ref cfg) = static_cfg {
            if cfg.static_ip_enabled {
                let ip = cfg.static_ip;
                info!("Falling back to static IP: {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);

                let static_config = embassy_net::ConfigV4::Static(embassy_net::StaticConfigV4 {
                    address: embassy_net::Ipv4Cidr::new(
                        embassy_net::Ipv4Address::new(ip[0], ip[1], ip[2], ip[3]),
                        24,
                    ),
                    gateway: Some(embassy_net::Ipv4Address::new(ip[0], ip[1], ip[2], 1)),
                    dns_servers: heapless::Vec::new(),
                });
                stack.set_config_v4(static_config);
                network_up = true;
            }
        }

        if !network_up {
            error!("No static IP configured - network disabled");
            error!("Device will continue operating without network access");
        }
    }

    if network_up {
        // Update network status - LED will blink faster now
        NETWORK_CONNECTED.lock(|f| f.replace(true));

        // Spawn TCP Sockets
        for _ in 0..NUMBER_HAMLIB_SOCKETS {
            unwrap!(spawner.spawn(listen_task(stack, 4533)));
        }

        // Spawn mDNS responder for device discovery
        unwrap!(spawner.spawn(mdns_task(stack)));

        // Spawn HTTP configuration server
        unwrap!(spawner.spawn(http_task(stack)));
    }

    let mut local_sockets_connected;
    let mut ticker = Ticker::every(Duration::from_millis(250));
    loop {
        // Set Socket-Connected LED
        local_sockets_connected = SOCKETS_CONNECTED.lock(|f| *f.borrow());
        sockets_led.set_level(if local_sockets_connected > 0 { Level::High } else { Level::Low });

        // Save config to flash if requested by HTTP handler
        if CONFIG_SAVE_PENDING.lock(|f| {
            let pending = *f.borrow();
            if pending { f.replace(false); }
            pending
        }) {
            let cfg_to_save = STORED_CONFIG.lock(|f| f.borrow().clone());
            if let Some(ref cfg) = cfg_to_save {
                config::save_config(&mut flash, cfg);
            }
        }

        // Reset system if requested, else feed watchdog
        if WATCHDOG_RESET_SYSTEM.lock(|f| *f.borrow())
        {
            watchdog.trigger_reset();
        }
        watchdog.feed();

        ticker.next().await;
    }
}
    

#[embassy_executor::task(pool_size = 4)]
async fn listen_task(stack: Stack<'static>, port: u16) {
    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];
    let mut buf = [0; 256];

    loop {
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(SOCKET_TIMEOUT_S)));

        if let Err(e) = socket.accept(port).await
        {
            warn!("TCP accept error: {:?}", e);
            continue;
        }

        // Connected.
        // Remote IP: socket.remote_endpoint()

        SOCKETS_CONNECTED.lock(|f| {
            let socket_count = *f.borrow();
            f.replace(socket_count + 1);
        });

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("TCP listen error: read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("TCP listen error: {:?}", e);
                    break;
                }
            };

            // Extended Response Protocol: detect '+' or other punctuation prefix
            let cmd_input = &buf[..n];
            let (erp_active, cmd_bytes) = if n > 0 && is_erp_prefix(cmd_input[0]) {
                (true, &cmd_input[1..])
            } else {
                (false, cmd_input)
            };

            let (response, mut demand_az, mut demand_el, _str_args) = Command::parse(cmd_bytes);

            match response {
                HamlibCommand::GetInfo => {
                    info!("Parsed get_info!");

                    let mut buf = [0u8; 192];

                    if erp_active {
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("get_info:\nInfo: {}, firmware: {}\nRPRT 0\n",
                                PRODUCT_NAME, GIT_VERSION
                            ),
                        ).unwrap();
                    } else {
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("Info \"{}, firmware: {}\"\n",
                                PRODUCT_NAME, GIT_VERSION
                            ),
                        ).unwrap();
                    }

                    let _ = socket.write(&buf).await;
                },
                HamlibCommand::GetPos => {
                    info!("Parsed get_pos!");

                    let (local_az_degrees, local_el_degrees) = CURRENT_AZ_EL_DEGREES.lock(|f| *f.borrow());

                    if erp_active {
                        let mut buf = [0u8; 80];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("get_pos:\nAzimuth: {:.2}\nElevation: {:.2}\nRPRT 0\n",
                                local_az_degrees, local_el_degrees),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    } else {
                        let mut buf = [0u8; 15];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("{:.2}\n{:.2}\n", local_az_degrees, local_el_degrees),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    }
                },
                HamlibCommand::Stop => {
                    info!("Parsed stop!");

                    DEMAND_RUN_AZ_EL_DEGREES.lock(|f| {
                        let (_, az, el) = *f.borrow();
                        f.replace((false, az, el));
                    });

                    if erp_active {
                        let _ = socket.write(b"stop:\nRPRT 0\n").await;
                    } else {
                        let _ = socket.write(b"RPRT 0\n").await;
                    }
                },
                HamlibCommand::Park => {
                    info!("Parsed park!");

                    let (park_az, park_el) = STORED_CONFIG.lock(|f| {
                        match f.borrow().as_ref() {
                            Some(cfg) => (cfg.park_az, cfg.park_el),
                            None => (PARK_AZ_DEGREES, PARK_EL_DEGREES),
                        }
                    });

                    DEMAND_RUN_AZ_EL_DEGREES.lock(|f| {
                        f.replace((true, park_az, park_el));
                    });

                    if erp_active {
                        let _ = socket.write(b"park:\nRPRT 0\n").await;
                    } else {
                        let _ = socket.write(b"RPRT 0\n").await;
                    }
                },
                HamlibCommand::SetPos => {
                    info!("Parsed Set Pos! ({}, {})", demand_az, demand_el);

                    demand_az = demand_az.clamp(0.0, CONTROL_AZ_DEGREES_MAXIMUM);
                    demand_el = demand_el.clamp(0.0, CONTROL_EL_DEGREES_MAXIMUM);

                    DEMAND_RUN_AZ_EL_DEGREES.lock(|f| {
                        f.replace((true, demand_az, demand_el));
                    });

                    if erp_active {
                        let mut buf = [0u8; 64];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("set_pos: {:.2} {:.2}\nRPRT 0\n", demand_az, demand_el),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    } else {
                        let _ = socket.write(b"RPRT 0\n").await;
                    }
                },
                HamlibCommand::Quit => {
                    info!("Parsed quit!");

                    // Close socket
                    break;
                },
                HamlibCommand::DumpState => {
                    info!("Parsed dump state!");

                    // Protocol v1 wire format matching netrotctl_open() in netrotctl.c
                    // Format: version, model, key=value pairs, "done"
                    let mut buf = [0u8; 256];
                    let _ = format_no_std::show(
                        &mut buf,
                        format_args!("\
                            1\n\
                            601\n\
                            min_az={:.6}\n\
                            max_az={:.6}\n\
                            min_el={:.6}\n\
                            max_el={:.6}\n\
                            south_zero=0\n\
                            rot_type=AzEl\n\
                            done\n",
                            0.0f32,
                            CONTROL_AZ_DEGREES_MAXIMUM as f64,
                            0.0f32,
                            CONTROL_EL_DEGREES_MAXIMUM as f64,
                        ),
                    ).unwrap();

                    let _ = socket.write(&buf).await;
                },
                HamlibCommand::Reset => {
                    info!("Parsed reset!");

                    // Raise flag for main thread to starve the watchdog
                    WATCHDOG_RESET_SYSTEM.lock(|f| {
                        f.replace(true);
                    });

                    break;
                },
                HamlibCommand::Move => {
                    let direction = demand_az as u16;
                    info!("Parsed move! dir={}, speed={}", direction, demand_el);

                    // Map direction to demand: move in the indicated direction
                    // by setting demand far in that direction so control_task drives the relays
                    let (current_az, current_el) = CURRENT_AZ_EL_DEGREES.lock(|f| *f.borrow());

                    let demand = match direction {
                        ROT_MOVE_UP =>         Some((current_az, CONTROL_EL_DEGREES_MAXIMUM)),
                        ROT_MOVE_DOWN =>       Some((current_az, 0.0)),
                        ROT_MOVE_LEFT =>       Some((0.0, current_el)),
                        ROT_MOVE_RIGHT =>      Some((CONTROL_AZ_DEGREES_MAXIMUM, current_el)),
                        ROT_MOVE_UP_LEFT =>    Some((0.0, CONTROL_EL_DEGREES_MAXIMUM)),
                        ROT_MOVE_UP_RIGHT =>   Some((CONTROL_AZ_DEGREES_MAXIMUM, CONTROL_EL_DEGREES_MAXIMUM)),
                        ROT_MOVE_DOWN_LEFT =>  Some((0.0, 0.0)),
                        ROT_MOVE_DOWN_RIGHT => Some((CONTROL_AZ_DEGREES_MAXIMUM, 0.0)),
                        _ => None,
                    };

                    if let Some((az, el)) = demand {
                        DEMAND_RUN_AZ_EL_DEGREES.lock(|f| {
                            f.replace((true, az, el));
                        });
                        if erp_active {
                            let _ = socket.write(b"move:\nRPRT 0\n").await;
                        } else {
                            let _ = socket.write(b"RPRT 0\n").await;
                        }
                    } else {
                        // Invalid direction — RPRT -2 (RIG_EINVAL)
                        if erp_active {
                            let _ = socket.write(b"move:\nRPRT -2\n").await;
                        } else {
                            let _ = socket.write(b"RPRT -2\n").await;
                        }
                    }
                },
                HamlibCommand::DumpCaps => {
                    info!("Parsed dump_caps!");

                    // Return capabilities in rotctld format
                    let mut buf = [0u8; 512];
                    let _ = format_no_std::show(
                        &mut buf,
                        format_args!("\
                            Model name: G-5500 Hamlib Adaptor\n\
                            Mfg name: Yaesu/Custom\n\
                            Backend version: {}\n\
                            Type: Az-El\n\
                            Min Azimuth: 0.00\n\
                            Max Azimuth: 450.00\n\
                            Min Elevation: 0.00\n\
                            Max Elevation: 180.00\n\
                            Can set Position: Y\n\
                            Can get Position: Y\n\
                            Can Stop: Y\n\
                            Can Park: Y\n\
                            Can Reset: Y\n\
                            Can Move: Y\n\
                            Can get Info: Y\n",
                            GIT_VERSION
                        ),
                    ).unwrap();

                    if erp_active {
                        let _ = socket.write(b"dump_caps:\n").await;
                        let _ = socket.write(&buf).await;
                        let _ = socket.write(b"RPRT 0\n").await;
                    } else {
                        let _ = socket.write(&buf).await;
                    }
                },
                HamlibCommand::GetStatus => {
                    info!("Parsed get_status!");

                    let demand_run = DEMAND_RUN_AZ_EL_DEGREES.lock(|f| {
                        let (demand_run, _, _) = *f.borrow();
                        demand_run
                    });

                    let status: u32 = if demand_run { ROT_STATUS_MOVING } else { ROT_STATUS_NONE };

                    if erp_active {
                        let mut buf = [0u8; 48];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("get_status:\n{}\nRPRT 0\n", status),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    } else {
                        let mut buf = [0u8; 16];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("{}\n", status),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    }
                },
                HamlibCommand::SetConf => {
                    info!("Parsed set_conf!");

                    let result: i8 = if let Some((token_bytes, value_bytes)) = _str_args {
                        match ConfigToken::from_bytes(token_bytes) {
                            Some(ConfigToken::ParkAz) | Some(ConfigToken::ParkEl) => {
                                // Writable token — parse value as f32
                                if let Ok(val_str) = core::str::from_utf8(value_bytes) {
                                    if let Ok(value) = val_str.parse::<f32>() {
                                        let token = ConfigToken::from_bytes(token_bytes).unwrap();
                                        STORED_CONFIG.lock(|f| {
                                            if let Some(ref mut cfg) = *f.borrow_mut() {
                                                match token {
                                                    ConfigToken::ParkAz => cfg.park_az = value,
                                                    ConfigToken::ParkEl => cfg.park_el = value,
                                                    _ => {}
                                                }
                                            }
                                        });
                                        CONFIG_SAVE_PENDING.lock(|f| { f.replace(true); });
                                        0
                                    } else {
                                        -2
                                    }
                                } else {
                                    -2
                                }
                            },
                            Some(_) => {
                                // Read-only token (MinAz, MaxAz, MinEl, MaxEl) — accept silently
                                0
                            },
                            None => -2,
                        }
                    } else {
                        -2
                    };

                    if erp_active {
                        let mut buf = [0u8; 32];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("set_conf:\nRPRT {}\n", result),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    } else {
                        let mut buf = [0u8; 16];
                        let _ = format_no_std::show(
                            &mut buf,
                            format_args!("RPRT {}\n", result),
                        ).unwrap();
                        let _ = socket.write(&buf).await;
                    }
                },
                HamlibCommand::GetConf => {
                    info!("Parsed get_conf!");

                    if let Some((token_bytes, _)) = _str_args {
                        match ConfigToken::from_bytes(token_bytes) {
                            Some(token) => {
                                let value: f32 = match token {
                                    ConfigToken::MinAz => 0.0,
                                    ConfigToken::MaxAz => CONTROL_AZ_DEGREES_MAXIMUM,
                                    ConfigToken::MinEl => 0.0,
                                    ConfigToken::MaxEl => CONTROL_EL_DEGREES_MAXIMUM,
                                    ConfigToken::ParkAz => STORED_CONFIG.lock(|f| {
                                        match f.borrow().as_ref() {
                                            Some(cfg) => cfg.park_az,
                                            None => PARK_AZ_DEGREES,
                                        }
                                    }),
                                    ConfigToken::ParkEl => STORED_CONFIG.lock(|f| {
                                        match f.borrow().as_ref() {
                                            Some(cfg) => cfg.park_el,
                                            None => PARK_EL_DEGREES,
                                        }
                                    }),
                                };

                                if erp_active {
                                    let mut buf = [0u8; 48];
                                    let _ = format_no_std::show(
                                        &mut buf,
                                        format_args!("get_conf:\n{:.6}\nRPRT 0\n", value),
                                    ).unwrap();
                                    let _ = socket.write(&buf).await;
                                } else {
                                    let mut buf = [0u8; 24];
                                    let _ = format_no_std::show(
                                        &mut buf,
                                        format_args!("{:.6}\n", value),
                                    ).unwrap();
                                    let _ = socket.write(&buf).await;
                                }
                            },
                            None => {
                                if erp_active {
                                    let _ = socket.write(b"get_conf:\nRPRT -2\n").await;
                                } else {
                                    let _ = socket.write(b"RPRT -2\n").await;
                                }
                            }
                        }
                    } else {
                        if erp_active {
                            let _ = socket.write(b"get_conf:\nRPRT -2\n").await;
                        } else {
                            let _ = socket.write(b"RPRT -2\n").await;
                        }
                    }
                },
                HamlibCommand::DumpConf => {
                    info!("Parsed dump_conf!");

                    let (park_az, park_el) = STORED_CONFIG.lock(|f| {
                        match f.borrow().as_ref() {
                            Some(cfg) => (cfg.park_az, cfg.park_el),
                            None => (PARK_AZ_DEGREES, PARK_EL_DEGREES),
                        }
                    });

                    let mut buf = [0u8; 256];
                    let _ = format_no_std::show(
                        &mut buf,
                        format_args!("\
                            min_az={:.6}\n\
                            max_az={:.6}\n\
                            min_el={:.6}\n\
                            max_el={:.6}\n\
                            park_az={:.6}\n\
                            park_el={:.6}\n",
                            0.0f32,
                            CONTROL_AZ_DEGREES_MAXIMUM,
                            0.0f32,
                            CONTROL_EL_DEGREES_MAXIMUM,
                            park_az,
                            park_el,
                        ),
                    ).unwrap();

                    if erp_active {
                        let _ = socket.write(b"dump_conf:\n").await;
                        let _ = socket.write(&buf).await;
                        let _ = socket.write(b"RPRT 0\n").await;
                    } else {
                        let _ = socket.write(&buf).await;
                    }
                },
                HamlibCommand::SetLevel => {
                    if erp_active {
                        let _ = socket.write(b"set_level:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::GetLevel => {
                    if erp_active {
                        let _ = socket.write(b"get_level:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::SetFunc => {
                    if erp_active {
                        let _ = socket.write(b"set_func:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::GetFunc => {
                    if erp_active {
                        let _ = socket.write(b"get_func:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::SetParm => {
                    if erp_active {
                        let _ = socket.write(b"set_parm:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::GetParm => {
                    if erp_active {
                        let _ = socket.write(b"get_parm:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::SendCmd => {
                    if erp_active {
                        let _ = socket.write(b"send_cmd:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Lonlat2Loc => {
                    if erp_active {
                        let _ = socket.write(b"lonlat2loc:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Loc2Lonlat => {
                    if erp_active {
                        let _ = socket.write(b"loc2lonlat:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Dms2Dec => {
                    if erp_active {
                        let _ = socket.write(b"dms2dec:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Dec2Dms => {
                    if erp_active {
                        let _ = socket.write(b"dec2dms:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Dmmm2Dec => {
                    if erp_active {
                        let _ = socket.write(b"dmmm2dec:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Dec2Dmmm => {
                    if erp_active {
                        let _ = socket.write(b"dec2dmmm:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::Qrb => {
                    if erp_active {
                        let _ = socket.write(b"qrb:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::AzSp2AzLp => {
                    if erp_active {
                        let _ = socket.write(b"a_sp2a_lp:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::DistSp2DistLp => {
                    if erp_active {
                        let _ = socket.write(b"d_sp2d_lp:\nRPRT -4\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -4\n").await;
                    }
                },
                HamlibCommand::_None => {
                    if erp_active {
                        let _ = socket.write(b"RPRT -1\n").await;
                    } else {
                        let _ = socket.write(b"RPRT -1\n").await;
                    }
                },
            }
        }

        // Disconnected.

        SOCKETS_CONNECTED.lock(|f| {
            let socket_count = *f.borrow();
            f.replace(socket_count.saturating_sub(1));
        });
    }
}

/// Wait for DHCP configuration with timeout
/// 
/// Returns: Option<StaticConfigV4>
/// - Some(config) if DHCP succeeds within timeout
/// - None if DHCP times out
/// 
/// When DHCP fails, the device continues operating without network functionality.
async fn wait_for_dhcp_config(stack: Stack<'static>) -> Option<embassy_net::StaticConfigV4> {
    let start = Instant::now();
    
    loop {
        if let Some(config) = stack.config_v4() {
            return Some(config.clone());
        }
        
        // Check if we've exceeded the timeout
        if start.elapsed().as_millis() > DHCP_TIMEOUT_MS {
            return None;
        }
        
        yield_now().await;
    }
}

/// Check if a byte is a valid Extended Response Protocol prefix character.
/// Per rotctld spec: '+', ';', '|', ',' and other ispunct() chars except '\', '?', '_', '#'
#[inline]
fn is_erp_prefix(b: u8) -> bool {
    matches!(b, b'+' | b';' | b'|' | b',')
}
