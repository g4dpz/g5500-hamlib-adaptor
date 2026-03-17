---
inclusion: always
---

# G-5500 Hamlib Adaptor — Project Context

## What This Is
A Rust embedded firmware for the W5500-EVB-Pico (RP2040) that provides a network-connected HamLib rotctld interface for the Yaesu G-5500 Az+El rotator. Primarily used with Gpredict for satellite tracking.

## Tech Stack
- Rust (no_std, no_main) targeting `thumbv6m-none-eabi` (Cortex-M0+)
- Embassy async runtime (executor 0.7.0, rp 0.4.0, net 0.7.0)
- W5500 Ethernet via SPI (embassy-net-wiznet 0.2.0)
- defmt + RTT for logging
- nom 8.0.0 for protocol parsing
- heapless 0.8.0 for no-std collections

## Architecture
Six concurrent Embassy tasks:
- `ethernet_task` — W5500 SPI driver runner
- `net_task` — smolTCP network stack runner
- `led_blink_task` — system LED (slow=no network, fast=connected)
- `control_task` — rotator relay control (250ms tick)
- `adc_task` — position monitoring (10kHz sampling, 512 samples/100ms)
- `listen_task` — TCP socket handler (pool_size=4, port 4533)

Shared state uses `Mutex<ThreadModeRawMutex, RefCell<T>>` statics.

## Memory Constraints
- RAM: 264KB total, ~126KB static, ~138KB available
- Flash: 2MB total, ~244KB firmware
- No heap allocation — everything is static
- TCP buffers: 1KB rx/tx per socket
- Command buffer: 256 bytes

## Key Pin Assignments
- GPIO 2-5: Rotator relays (Az CW/CCW, El UP/DN)
- GPIO 26-27: ADC position inputs (Az/El via voltage divider)
- GPIO 25: System LED (onboard), GPIO 15: Sockets LED
- GPIO 16-21: W5500 SPI0 + control

## Build & Flash
```bash
cd firmware
cargo build --release    # Size-optimized (lto=fat, opt-level=z)
cargo run                # Flash via probe-rs or UF2
```

## HamLib Protocol (partial rotctld)
Supported: `_`/`\get_info`, `p`/`\get_pos`, `S`/`\stop`, `K`/`\park`, `P`/`\set_pos`, `q`/`\quit`, `\dump_state`, `R`/`\reset`

## License
BSD 3-clause — Copyright Phil Crump 2025
