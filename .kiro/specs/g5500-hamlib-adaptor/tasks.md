# Tasks: G-5500 Hamlib Adaptor

## Implementation Tasks

### Task 1: Project Scaffolding & Build System
- [x] Set up Cargo workspace with `no_std`/`no_main` for RP2040
- [x] Configure `.cargo/config.toml` (target, runner, DEFMT_LOG)
- [x] Create `build.rs` for memory.x copy and linker script flags
- [x] Configure release profile (fat LTO, opt-level z, codegen-units 1, panic abort)
- [x] Configure dev profile (LTO, opt-level z, panic abort)
- [x] Add `Probe.toml` for probe-rs configuration
- [x] Add Embassy framework as git submodule

### Task 2: Hardware Initialization (`main`)
- [x] Initialize RP2040 peripherals via `embassy_rp::init`
- [x] Configure system LED (GPIO 25) and sockets LED (GPIO 15) as outputs
- [x] Initialize watchdog with 8.3s timeout
- [x] Read flash unique ID (8 bytes) into `FLASH_UUID` static
- [x] Configure SPI0 at 50MHz for W5500 (MISO=16, MOSI=19, CLK=18, CS=17)
- [x] Set up W5500 interrupt (GPIO 21, pull-up) and reset (GPIO 20) pins
- [x] Initialize W5500 driver with MAC address, spawn `ethernet_task`
- [x] Configure DHCP with hostname, create network stack, spawn `net_task`
- [x] Split resources via `assign_resources!` macro, spawn `adc_task` and `control_task`
- [x] Spawn `led_blink_task`

### Task 3: DHCP & Network Startup
- [x] Implement `wait_for_dhcp_config()` with 5s timeout polling
- [x] Feed watchdog before and after DHCP wait
- [x] On success: log IP, set `NETWORK_CONNECTED = true`, spawn 4 `listen_task` instances
- [x] On failure: log error, skip TCP spawn, device continues without network

### Task 4: ADC Position Monitoring (`adc_task`)
- [x] Configure ADC with DMA (CH2) for multichannel reading (GPIO 26, 27)
- [x] Implement 512-sample multichannel read at 10kHz (DIV=2399)
- [x] Filter DNL spikes at values [512, 1536, 2560, 3584]
- [x] Average valid samples per channel
- [x] Convert raw ADC to degrees via voltage divider calibration constants
- [x] Clamp azimuth 0–450°, elevation 0–180°
- [x] Update `CURRENT_AZ_EL_DEGREES` and `CURRENT_AZ_EL_RAW` statics
- [x] Run on 100ms ticker

### Task 5: Rotator Control (`control_task`)
- [x] Configure 4 relay GPIO outputs (2=Az CW, 3=Az CCW, 4=El UP, 5=El DN)
- [x] Read demand state and current position each 250ms tick
- [x] Implement directional control with ±3° dead-band threshold
- [x] Ensure mutual exclusion on relay pairs (never CW+CCW or UP+DN)
- [x] Auto-clear run flag when on-target
- [x] All relays off when stopped

### Task 6: HamLib Protocol Parser
- [x] Define `HamlibCommand` enum (GetInfo, GetPos, Stop, Park, SetPos, Quit, DumpState, Reset, _None)
- [x] Implement `nom` parsers for each command with `#[inline]` hints
- [x] `parse_set_pos`: extract two floats from `P AZ EL` / `\set_pos AZ EL`
- [x] `parse()`: try each parser in sequence, return command + optional floats

### Task 7: TCP Socket Handler (`listen_task`)
- [x] Implement as `pool_size = 4` task
- [x] Allocate 1KB rx, 1KB tx, 256B command buffers per instance
- [x] Accept on port 4533 with 60s socket timeout
- [x] Increment/decrement `SOCKETS_CONNECTED` on connect/disconnect (saturating_sub)
- [x] Dispatch parsed commands:
  - [x] GetInfo: format product name + git version
  - [x] GetPos: read and format current Az/El degrees
  - [x] Stop: clear run flag, respond RPRT 0
  - [x] Park: set demand to (180°, 0°) with run=true, respond RPRT 0
  - [x] SetPos: clamp and set demand with run=true, respond RPRT 0
  - [x] Quit: break connection loop
  - [x] DumpState: format full diagnostics (UUID, uptime, clients, positions, demand)
  - [x] Reset: set `WATCHDOG_RESET_SYSTEM` flag
  - [x] _None: respond RPRT 1

### Task 8: LED Status Indicators (`led_blink_task`)
- [x] Read `NETWORK_CONNECTED` flag each cycle
- [x] Toggle system LED at 500ms (connected) or 1000ms (disconnected)

### Task 9: Main Loop & Watchdog
- [x] 250ms ticker in main loop
- [x] Update sockets LED from `SOCKETS_CONNECTED`
- [x] Check `WATCHDOG_RESET_SYSTEM` flag → trigger watchdog reset
- [x] Feed watchdog each tick

## Future / Planned Tasks

### Task 10: mDNS/Bonjour Discovery
- [x] Add `udp` and `multicast` features to embassy-net in Cargo.toml
- [x] Increase `StackResources` from 5 to 6 for the UDP socket
- [x] Implement minimal mDNS responder in `firmware/src/mdns.rs` (no extra crates)
- [x] Join multicast group 224.0.0.251, listen on UDP port 5353
- [x] Respond to A record queries for `g5500-hamlib-adaptor.local`
- [x] Respond to PTR queries for `_rotctld._tcp.local` with SRV + TXT + A records
- [x] Spawn `mdns_task` after DHCP success in main

### Task 11: Persistent Configuration Storage
- [x] Implement config read/write to reserved flash region (0x101E0000–0x10200000)
- [x] Store: static IP fallback, calibration offsets, park position
- [x] CRC8 integrity check on stored config

### Task 12: Web Configuration Interface
- [x] Serve minimal HTTP on a secondary port for browser-based config
- [x] Display current status, allow network/calibration settings changes

### Task 13: Static IP Fallback
- [x] If DHCP fails, fall back to a configurable static IP
- [x] Store static IP config in flash

### Task 14: Extended rotctld Commands
- [x] Implement additional rotctld commands as needed (e.g., `M`, `\move`)
- [x] Improve protocol compliance for broader client compatibility
