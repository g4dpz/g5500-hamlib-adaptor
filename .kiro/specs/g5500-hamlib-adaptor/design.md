# Design: G-5500 Hamlib Adaptor

## System Architecture

```
┌──────────────────────────────────────────────────────────┐
│                  Embassy Async Executor                   │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ ethernet_task │  │  net_task    │  │ led_blink_task│  │
│  │ (W5500 SPI)  │  │ (smolTCP)   │  │ (GPIO 25)     │  │
│  └──────────────┘  └──────────────┘  └───────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ control_task  │  │  adc_task   │  │ listen_task   │  │
│  │ (relays)     │  │ (ADC+DMA)   │  │ (×4 TCP pool) │  │
│  └──────┬───────┘  └──────┬──────┘  └───────┬───────┘  │
│         │                  │                  │          │
│  ┌──────▼──────────────────▼──────────────────▼───────┐  │
│  │          Shared State (Mutex<RefCell<T>>)           │  │
│  │  CURRENT_AZ_EL_DEGREES  DEMAND_RUN_AZ_EL_DEGREES  │  │
│  │  CURRENT_AZ_EL_RAW      NETWORK_CONNECTED          │  │
│  │  SOCKETS_CONNECTED      WATCHDOG_RESET_SYSTEM       │  │
│  │  FLASH_UUID                                         │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Shared State Layer

All inter-task communication uses `static Mutex<ThreadModeRawMutex, RefCell<T>>`. This is safe on single-core Cortex-M0+ where interrupts are the only concurrency concern, and Embassy tasks are cooperatively scheduled on a single thread.

| Static | Type | Writers | Readers |
|--------|------|---------|---------|
| `CURRENT_AZ_EL_DEGREES` | `(f32, f32)` | adc_task | listen_task, control_task |
| `CURRENT_AZ_EL_RAW` | `(f32, f32)` | adc_task | listen_task (dump_state) |
| `DEMAND_RUN_AZ_EL_DEGREES` | `(bool, f32, f32)` | listen_task | control_task |
| `NETWORK_CONNECTED` | `bool` | main | led_blink_task |
| `SOCKETS_CONNECTED` | `u16` | listen_task | main (LED control) |
| `WATCHDOG_RESET_SYSTEM` | `bool` | listen_task | main (watchdog) |
| `FLASH_UUID` | `[u8; 8]` | main (init) | listen_task (dump_state) |

Access pattern: `STATE.lock(|f| { ... f.borrow() / f.replace(...) })` — short critical sections, no blocking.

### 2. ADC Subsystem (`adc_task`)

- Multichannel ADC with DMA (`read_many_multichannel`) on channels 0 (Az) and 1 (El)
- 512 samples per channel at ~10kHz (DIV=2399), interleaved in buffer `[u16; 1024]`
- DNL spike filtering: skip samples matching `[512, 1536, 2560, 3584]`
- Averaging: sum valid samples, divide by count
- Voltage divider calibration: `raw → degrees` via linear mapping from ADC range to 0–450° (Az) or 0–180° (El)
- Calibration constants derived from: 10kΩ/10kΩ divider, 3.3V Vref, 0–5V G-5500 output range
- 100ms ticker cycle

### 3. Control Subsystem (`control_task`)

- 250ms ticker
- Reads demand state `(run, az, el)` and current position each tick
- If `run == true`:
  - Compare current vs demand for each axis
  - If error > 3° threshold: activate appropriate relay (CW/CCW or UP/DN), deactivate opposite
  - If within threshold on both axes: clear all relays, set `run = false`
- If `run == false`: all relays low
- Mutual exclusion on relay pairs (never CW+CCW or UP+DN simultaneously)

### 4. Protocol Parser (`Command::parse`)

- Uses `nom` parser combinators operating on `&[u8]` slices
- Each command has a dedicated `parse_*` method with `#[inline]` hint
- `parse()` tries each parser in sequence, returns `(HamlibCommand, f32, f32)`
- `SetPos` extracts two floats; all others return `(0.0, 0.0)` as unused
- Unrecognized input returns `HamlibCommand::_None`

### 5. Network Subsystem

- W5500 driven over SPI0 at 50MHz with DMA (CH0 tx, CH1 rx)
- `ethernet_task`: runs the W5500 hardware driver loop
- `net_task`: runs the smolTCP network stack
- DHCP with 5s timeout via `wait_for_dhcp_config()` polling with `yield_now()`
- On DHCP success: spawn 4 `listen_task` instances, set `NETWORK_CONNECTED = true`
- On DHCP failure: log error, skip TCP spawn, device continues with ADC/control only

### 6. TCP Socket Handler (`listen_task`, pool_size=4)

- Each instance: 1KB rx buffer, 1KB tx buffer, 256B command buffer
- Accept loop: `socket.accept(4533)` → read loop → disconnect → re-accept
- On connect: increment `SOCKETS_CONNECTED`
- On disconnect: decrement with `saturating_sub(1)`
- Socket timeout: 60 seconds idle
- Command dispatch via `Command::parse()`, response written directly to socket

### 7. LED Subsystem (`led_blink_task`)

- Reads `NETWORK_CONNECTED` each cycle
- Connected: 500ms toggle (2Hz blink)
- Disconnected: 1000ms toggle (1Hz blink)
- Independent task, no interaction with other subsystems beyond shared flag

### 8. Main Loop & Watchdog

- Initializes all peripherals, spawns tasks, runs DHCP
- 250ms ticker in main loop:
  - Update sockets LED based on `SOCKETS_CONNECTED`
  - Check `WATCHDOG_RESET_SYSTEM` flag → trigger reset if set
  - Feed watchdog
- Watchdog period: 8.3s (covers DHCP 5s + margin)

## Resource Allocation

### GPIO
| GPIO | Function | Direction | Notes |
|------|----------|-----------|-------|
| 2 | Az CW relay | Output | Active high |
| 3 | Az CCW relay | Output | Active high |
| 4 | El UP relay | Output | Active high |
| 5 | El DN relay | Output | Active high |
| 15 | Sockets LED | Output | External |
| 16 | SPI0 MISO | Alt (SPI) | W5500 |
| 17 | SPI0 CS | Output | W5500 |
| 18 | SPI0 CLK | Alt (SPI) | W5500 |
| 19 | SPI0 MOSI | Alt (SPI) | W5500 |
| 20 | W5500 Reset | Output | Active low |
| 21 | W5500 INT | Input | Pull-up |
| 25 | System LED | Output | Onboard |
| 26 | Az ADC | Analog | ADC0 |
| 27 | El ADC | Analog | ADC1 |

### DMA Channels
| Channel | Usage |
|---------|-------|
| CH0 | SPI0 TX (W5500) |
| CH1 | SPI0 RX (W5500) |
| CH2 | ADC multichannel |
| CH3 | Flash operations |

### Memory Map
- Flash: 2MB (boot2 256B + firmware ~244KB + reserved config 128KB)
- RAM: 264KB (static ~126KB + stack from remainder)

## Build Configuration

- Target: `thumbv6m-none-eabi`
- Release: `lto = "fat"`, `opt-level = 'z'`, `codegen-units = 1`, `panic = "abort"`
- Dev: `lto = true`, `opt-level = "z"`, `panic = "abort"`
- Linker scripts: `link.x` (Cortex-M), `link-rp.x` (RP2040), `defmt.x`
- Runner: `probe-rs run --chip RP2040`
- Log level: `DEFMT_LOG=debug`
