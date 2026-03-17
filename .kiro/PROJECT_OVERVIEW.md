# G-5500 Hamlib Adaptor - Project Overview

## Project Purpose
Network-connected physical adaptor providing a Hamlib interface for the G-5500 Az+El rotator, primarily for Gpredict. The device serves a partial implementation of the 'rotctld' protocol on TCP sockets and controls the rotator through the G-5500's GS-232 DIN port.

## Hardware Platform
- **Board**: Wiznet Pi Pico ("W5500-EVB-Pico")
- **Microcontroller**: RP2040 (Raspberry Pi Pico)
- **Network Interface**: W5500 Ethernet module (SPI-based)
- **Power**: Powered from the G-5500 DIN port

## Key Features
- Async/concurrent network handling (up to 4 simultaneous HamLib clients)
- Real-time azimuth/elevation monitoring via ADC
- Relay-based rotator control (CW/CCW for Az, UP/DN for El)
- DHCP with graceful failure (device continues operating without network)
- LED status indicators (network status + client connections)
- Watchdog timer for system reliability
- Git version tracking in firmware

## Technology Stack
- **Language**: Rust (no_std, embedded)
- **Framework**: Embassy (async runtime for embedded)
- **Networking**: Embassy-net with W5500 driver
- **Protocol**: HamLib rotctld (partial implementation)
- **Build System**: Cargo with custom build.rs
- **Debugging**: probe-rs with RTT support
- **Flashing**: UF2 bootloader (primary) or probe-rs

## Project Structure
```
.
├── README.md                          # Root project overview
├── LICENSE.txt                        # BSD 3-clause license
├── docs/                              # Documentation (G-5500 manual)
├── firmware/                          # Main firmware codebase
│   ├── src/
│   │   ├── main.rs                   # Main entry point, tasks, protocol parsing
│   │   ├── config.rs                 # Configuration management (WIP)
│   │   └── crc8_ccitt.rs             # CRC utilities
│   ├── Cargo.toml                    # Dependencies and build config
│   ├── Cargo.lock                    # Locked dependency versions
│   ├── build.rs                      # Build script for memory.x
│   ├── memory.x                      # RP2040 memory layout
│   ├── Probe.toml                    # probe-rs configuration
│   ├── .cargo/config.toml            # Cargo target and runner config
│   ├── embassy/                      # Embedded Embassy framework (git submodule)
│   └── [Documentation files]         # Guides and references
└── .kiro/                            # Kiro steering files (this directory)
```

## Licensing
BSD 3-clause License (New BSD License)
Copyright (c) Phil Crump 2025. All rights reserved.
