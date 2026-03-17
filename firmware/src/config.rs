/// Persistent configuration storage in flash
///
/// Flash layout v2 (within a single 4KB erase sector):
///   [0]       Magic byte (0xAE)
///   [1]       Config version (0x02)
///   [2]       Flags (bit 0 = static IP enabled)
///   [3..7]    Static IP: [ip0, ip1, ip2, ip3]
///   [7..11]   Az calibration offset (f32 LE)
///   [11..15]  El calibration offset (f32 LE)
///   [15..19]  Park Az degrees (f32 LE)
///   [19..23]  Park El degrees (f32 LE)
///   [23]      Calibration flags (bit 0 = calibration_valid)
///   [24..28]  az_raw_low (f32 LE)
///   [28..32]  az_raw_high (f32 LE)
///   [32..36]  el_raw_low (f32 LE)
///   [36..40]  el_raw_high (f32 LE)
///   [40..44]  az_deg_low (f32 LE)
///   [44..48]  az_deg_high (f32 LE)
///   [48..52]  el_deg_low (f32 LE)
///   [52..56]  el_deg_high (f32 LE)
///   [56..59]  Reserved (0x00)
///   [59]      CRC8-CCITT over bytes [0..59]
///
/// Total: 60 bytes

use embassy_rp::flash::{Async, ERASE_SIZE};
use embassy_rp::peripherals::FLASH;
use defmt::*;

use crate::crc8_ccitt;

const FLASH_SIZE: usize = 2 * 1024 * 1024;

/// Config lives in the last 4KB sector of the reserved 128KB region
/// Reserved region: 0x101E0000–0x10200000 (offsets 0x1E0000–0x200000)
/// We use the first sector at offset 0x1E0000
const CONFIG_FLASH_OFFSET: u32 = 0x1E_0000;

const CONFIG_MAGIC: u8 = 0xAE;
const CONFIG_VERSION: u8 = 0x02;
const CONFIG_SIZE: usize = 60;

const CONFIG_SIZE_V1: usize = 24;

const FLAG_STATIC_IP_ENABLED: u8 = 0x01;
const FLAG_CALIBRATION_VALID: u8 = 0x01;

#[derive(Clone)]
pub struct Config {
    pub static_ip_enabled: bool,
    pub static_ip: [u8; 4],
    pub az_cal_offset: f32,
    pub el_cal_offset: f32,
    pub park_az: f32,
    pub park_el: f32,
    pub calibration_valid: bool,
    pub az_raw_low: f32,
    pub az_raw_high: f32,
    pub el_raw_low: f32,
    pub el_raw_high: f32,
    pub az_deg_low: f32,
    pub az_deg_high: f32,
    pub el_deg_low: f32,
    pub el_deg_high: f32,
}

impl Config {
    pub fn default() -> Self {
        Config {
            static_ip_enabled: false,
            static_ip: [192, 168, 1, 100],
            az_cal_offset: 0.0,
            el_cal_offset: 0.0,
            park_az: 180.0,
            park_el: 0.0,
            calibration_valid: false,
            az_raw_low: 0.0,
            az_raw_high: 0.0,
            el_raw_low: 0.0,
            el_raw_high: 0.0,
            az_deg_low: 0.0,
            az_deg_high: 0.0,
            el_deg_low: 0.0,
            el_deg_high: 0.0,
        }
    }

    /// Serialize config to a 60-byte v2 buffer with magic, version, and CRC8
    pub fn to_bytes(&self) -> [u8; CONFIG_SIZE] {
        let mut buf = [0u8; CONFIG_SIZE];
        buf[0] = CONFIG_MAGIC;
        buf[1] = CONFIG_VERSION;
        buf[2] = if self.static_ip_enabled { FLAG_STATIC_IP_ENABLED } else { 0 };
        buf[3..7].copy_from_slice(&self.static_ip);
        buf[7..11].copy_from_slice(&self.az_cal_offset.to_le_bytes());
        buf[11..15].copy_from_slice(&self.el_cal_offset.to_le_bytes());
        buf[15..19].copy_from_slice(&self.park_az.to_le_bytes());
        buf[19..23].copy_from_slice(&self.park_el.to_le_bytes());
        buf[23] = if self.calibration_valid { FLAG_CALIBRATION_VALID } else { 0 };
        buf[24..28].copy_from_slice(&self.az_raw_low.to_le_bytes());
        buf[28..32].copy_from_slice(&self.az_raw_high.to_le_bytes());
        buf[32..36].copy_from_slice(&self.el_raw_low.to_le_bytes());
        buf[36..40].copy_from_slice(&self.el_raw_high.to_le_bytes());
        buf[40..44].copy_from_slice(&self.az_deg_low.to_le_bytes());
        buf[44..48].copy_from_slice(&self.az_deg_high.to_le_bytes());
        buf[48..52].copy_from_slice(&self.el_deg_low.to_le_bytes());
        buf[52..56].copy_from_slice(&self.el_deg_high.to_le_bytes());
        // bytes 56..59 are reserved, already 0x00
        buf[59] = crc8_ccitt::crc8_ccitt_buffer(&buf[..59]);
        buf
    }

    /// Deserialize config from a 60-byte buffer, dispatching on version byte
    pub fn from_bytes(buf: &[u8; CONFIG_SIZE]) -> Option<Self> {
        if buf[0] != CONFIG_MAGIC {
            return None;
        }

        match buf[1] {
            0x01 => Self::from_bytes_v1(buf),
            0x02 => Self::from_bytes_v2(buf),
            _ => None,
        }
    }

    /// Deserialize a v2 (60-byte) config with CRC8 validation
    fn from_bytes_v2(buf: &[u8; CONFIG_SIZE]) -> Option<Self> {
        if !crc8_ccitt::crc8_ccitt_validate(buf) {
            return None;
        }

        let static_ip_enabled = (buf[2] & FLAG_STATIC_IP_ENABLED) != 0;
        let static_ip = [buf[3], buf[4], buf[5], buf[6]];
        let az_cal_offset = f32::from_le_bytes([buf[7], buf[8], buf[9], buf[10]]);
        let el_cal_offset = f32::from_le_bytes([buf[11], buf[12], buf[13], buf[14]]);
        let park_az = f32::from_le_bytes([buf[15], buf[16], buf[17], buf[18]]);
        let park_el = f32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]);
        let calibration_valid = (buf[23] & FLAG_CALIBRATION_VALID) != 0;
        let az_raw_low = f32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        let az_raw_high = f32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);
        let el_raw_low = f32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        let el_raw_high = f32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
        let az_deg_low = f32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]);
        let az_deg_high = f32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]);
        let el_deg_low = f32::from_le_bytes([buf[48], buf[49], buf[50], buf[51]]);
        let el_deg_high = f32::from_le_bytes([buf[52], buf[53], buf[54], buf[55]]);

        Some(Config {
            static_ip_enabled,
            static_ip,
            az_cal_offset,
            el_cal_offset,
            park_az,
            park_el,
            calibration_valid,
            az_raw_low,
            az_raw_high,
            el_raw_low,
            el_raw_high,
            az_deg_low,
            az_deg_high,
            el_deg_low,
            el_deg_high,
        })
    }

    /// Migrate a v1 (24-byte) config from a 60-byte buffer
    /// Validates CRC8 over only the first 24 bytes (v1 layout)
    fn from_bytes_v1(buf: &[u8; CONFIG_SIZE]) -> Option<Self> {
        // V1 CRC covers bytes 0..23, CRC at byte 23
        if !crc8_ccitt::crc8_ccitt_validate(&buf[..CONFIG_SIZE_V1]) {
            return None;
        }

        let static_ip_enabled = (buf[2] & FLAG_STATIC_IP_ENABLED) != 0;
        let static_ip = [buf[3], buf[4], buf[5], buf[6]];
        let az_cal_offset = f32::from_le_bytes([buf[7], buf[8], buf[9], buf[10]]);
        let el_cal_offset = f32::from_le_bytes([buf[11], buf[12], buf[13], buf[14]]);
        let park_az = f32::from_le_bytes([buf[15], buf[16], buf[17], buf[18]]);
        let park_el = f32::from_le_bytes([buf[19], buf[20], buf[21], buf[22]]);

        Some(Config {
            static_ip_enabled,
            static_ip,
            az_cal_offset,
            el_cal_offset,
            park_az,
            park_el,
            calibration_valid: false,
            az_raw_low: 0.0,
            az_raw_high: 0.0,
            el_raw_low: 0.0,
            el_raw_high: 0.0,
            az_deg_low: 0.0,
            az_deg_high: 0.0,
            el_deg_low: 0.0,
            el_deg_high: 0.0,
        })
    }
}

/// Load config from flash. Returns default config if flash is uninitialised or corrupted.
pub fn load_config(flash: &mut embassy_rp::flash::Flash<'_, FLASH, Async, FLASH_SIZE>) -> Config {
    let mut buf = [0u8; CONFIG_SIZE];

    match flash.blocking_read(CONFIG_FLASH_OFFSET, &mut buf) {
        Ok(()) => {},
        Err(e) => {
            error!("Flash read error: {:?}", e);
            return Config::default();
        }
    }

    match Config::from_bytes(&buf) {
        Some(config) => {
            info!("Config loaded from flash (park: {}°/{}°, cal: {}/{}, cal_valid: {})",
                config.park_az, config.park_el, config.az_cal_offset, config.el_cal_offset,
                config.calibration_valid);
            config
        }
        None => {
            warn!("No valid config in flash, using defaults");
            Config::default()
        }
    }
}

/// Save config to flash. Erases the sector first, then writes.
#[allow(dead_code)]
pub fn save_config(flash: &mut embassy_rp::flash::Flash<'_, FLASH, Async, FLASH_SIZE>, config: &Config) -> bool {
    let buf = config.to_bytes();

    // Erase the 4KB sector containing our config
    if let Err(e) = flash.blocking_erase(CONFIG_FLASH_OFFSET, CONFIG_FLASH_OFFSET + ERASE_SIZE as u32) {
        error!("Flash erase error: {:?}", e);
        return false;
    }

    // Write config bytes
    if let Err(e) = flash.blocking_write(CONFIG_FLASH_OFFSET, &buf) {
        error!("Flash write error: {:?}", e);
        return false;
    }

    // Verify by reading back
    let mut verify = [0u8; CONFIG_SIZE];
    if let Err(e) = flash.blocking_read(CONFIG_FLASH_OFFSET, &mut verify) {
        error!("Flash verify read error: {:?}", e);
        return false;
    }

    if verify != buf {
        error!("Flash verify mismatch");
        return false;
    }

    info!("Config saved to flash");
    true
}
