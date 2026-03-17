# Kiro Steering Files - G-5500 Hamlib Adaptor

This directory contains comprehensive steering files that capture the project's requirements, architecture, conventions, and best practices. These files are designed to help developers understand and work with the codebase efficiently.

## Files Overview

### 1. PROJECT_OVERVIEW.md
High-level project summary including:
- Project purpose and goals
- Hardware platform specifications
- Key features and capabilities
- Technology stack overview
- Project structure and organization
- Licensing information

**Use this when**: You need to understand what the project does and how it's organized.

### 2. TECH_STACK.md
Detailed technology and dependency information:
- Core language and runtime (Rust, no_std)
- Async runtime (Embassy framework)
- Hardware abstraction layers
- Networking stack
- Build tools and profiles
- Memory layout and constraints
- Optimization strategies

**Use this when**: You need to understand dependencies, build configuration, or memory constraints.

### 3. CODE_PATTERNS.md
Coding conventions and architectural patterns:
- Task-based concurrency model
- Shared state management patterns
- Protocol parsing approach
- Logging conventions
- Timing and delay patterns
- GPIO and ADC patterns
- Network socket patterns
- Error handling strategies
- Naming conventions
- Optimization techniques

**Use this when**: You're writing new code or modifying existing code and need to follow project conventions.

### 4. BUILD_DEPLOYMENT.md
Build system and deployment procedures:
- Cargo configuration and build scripts
- Build profiles (release vs debug)
- Flashing methods (UF2, probe-rs, scripts)
- Cargo commands and analysis tools
- Memory layout and usage
- Deployment checklist
- Troubleshooting guide
- Release process

**Use this when**: You need to build, flash, or deploy the firmware.

### 5. HARDWARE_ARCHITECTURE.md
Detailed hardware specifications and pin assignments:
- RP2040 microcontroller specs
- W5500-EVB-Pico board features
- Complete pin assignments
- Voltage divider configuration
- Relay driver interface
- Power distribution and budget
- Clock configuration
- Watchdog timer setup
- DMA channel allocation
- Memory mapping
- Debugging interfaces
- Expansion possibilities

**Use this when**: You need to understand hardware connections, pin assignments, or add new hardware features.

## Quick Reference

### Common Tasks

**Building the firmware:**
```bash
cd firmware
cargo build --release
```

**Flashing to device:**
```bash
cd firmware
cargo run
```

**Viewing logs:**
```bash
cd firmware
cargo run --release
# Logs appear in RTT output
```

**Checking code size:**
```bash
cd firmware
arm-none-eabi-size target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor
```

**Testing the device:**
```bash
telnet <device-ip> 4533
_                    # Get info
p                    # Get position
\dump_state          # Full diagnostics
```

### Key Constants (in firmware/src/main.rs)

| Constant | Value | Purpose |
|----------|-------|---------|
| `WATCHDOG_PERIOD_MS` | 8300 | Watchdog timeout |
| `DHCP_TIMEOUT_MS` | 5000 | DHCP configuration timeout |
| `NUMBER_HAMLIB_SOCKETS` | 4 | Max concurrent clients |
| `SOCKET_TIMEOUT_S` | 60 | Per-socket idle timeout |
| `PARK_AZ_DEGREES` | 180.0 | Park azimuth position |
| `PARK_EL_DEGREES` | 0.0 | Park elevation position |
| `CONTROL_DEGREES_THRESHOLD` | 3.0 | Position tolerance |

### Pin Assignments (Quick Reference)

**Control Outputs:**
- GPIO 2: Azimuth CW
- GPIO 3: Azimuth CCW
- GPIO 4: Elevation UP
- GPIO 5: Elevation DN

**Position Inputs (ADC):**
- GPIO 26: Azimuth position
- GPIO 27: Elevation position

**Status LEDs:**
- GPIO 25: System LED (onboard)
- GPIO 15: Sockets LED (external)

**Ethernet (W5500):**
- GPIO 16-21: SPI + control signals
- GPIO 18: SPI CLK
- GPIO 19: SPI MOSI
- GPIO 16: SPI MISO
- GPIO 17: SPI CS

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────┐
│                    RP2040 (Cortex-M0+)                  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │         Embassy Async Runtime                    │  │
│  │  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │ Ethernet     │  │ Network      │             │  │
│  │  │ Task         │  │ Task         │             │  │
│  │  └──────────────┘  └──────────────┘             │  │
│  │  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │ LED Blink    │  │ Control      │             │  │
│  │  │ Task         │  │ Task         │             │  │
│  │  └──────────────┘  └──────────────┘             │  │
│  │  ┌──────────────┐  ┌──────────────┐             │  │
│  │  │ ADC          │  │ Listen       │             │  │
│  │  │ Task         │  │ Tasks (x4)   │             │  │
│  │  └──────────────┘  └──────────────┘             │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │         Shared State (Mutex + RefCell)           │  │
│  │  • Current Az/El degrees                         │  │
│  │  • Demand Az/El + run flag                       │  │
│  │  • Network connected status                      │  │
│  │  • Socket count                                  │  │
│  └──────────────────────────────────────────────────┘  │
│                                                         │
│  ┌──────────────────────────────────────────────────┐  │
│  │         Hardware Peripherals                      │  │
│  │  • SPI0 (W5500)  • ADC (Az/El)                   │  │
│  │  • GPIO (LEDs)   • GPIO (Relays)                 │  │
│  │  • Watchdog      • DMA                           │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
         │                                    │
         ├─── SPI ──────────────────────────┤
         │                                    │
    ┌────▼────────────────────────────────────▼────┐
    │         W5500 Ethernet Controller             │
    │  • 10/100 Mbps                                │
    │  • TCP/IP Stack (smolTCP)                     │
    │  • DHCP Client                                │
    └────┬────────────────────────────────────┬────┘
         │                                    │
         └─── RJ45 ──────────────────────────┘
              Network
```

## Development Workflow

1. **Understand the project**: Read PROJECT_OVERVIEW.md
2. **Set up environment**: Follow BUILD_DEPLOYMENT.md
3. **Understand architecture**: Read CODE_PATTERNS.md and HARDWARE_ARCHITECTURE.md
4. **Make changes**: Follow conventions in CODE_PATTERNS.md
5. **Build and test**: Use commands in BUILD_DEPLOYMENT.md
6. **Deploy**: Follow deployment checklist in BUILD_DEPLOYMENT.md

## Important Notes

### Memory Constraints
- Total RAM: 264KB
- Static data: ~126KB (48%)
- Available for stack: ~138KB
- No heap (no_std environment)

### Performance Targets
- ADC sampling: 10kHz (100ms per sample)
- Control loop: 250ms tick
- Network: Async, non-blocking
- Watchdog: 8.3s timeout

### Reliability Features
- Watchdog timer (prevents hang)
- DHCP timeout with graceful failure
- Socket timeout (60s idle)
- ADC DNL spike filtering
- Saturating arithmetic for counters

## Getting Help

- **Build issues**: See BUILD_DEPLOYMENT.md troubleshooting section
- **Hardware questions**: See HARDWARE_ARCHITECTURE.md
- **Code patterns**: See CODE_PATTERNS.md
- **Dependencies**: See TECH_STACK.md
- **Project overview**: See PROJECT_OVERVIEW.md

## Contributing

When contributing to this project:
1. Follow naming conventions in CODE_PATTERNS.md
2. Use established patterns for new features
3. Keep memory usage in mind (see TECH_STACK.md)
4. Update relevant steering files if adding new patterns
5. Test on actual hardware before submitting

## Version Information

- **Rust**: 1.85.1+ (latest stable)
- **Embassy**: 0.7.0 (executor), 0.4.0 (rp)
- **Target**: thumbv6m-none-eabi (Cortex-M0+)
- **Board**: W5500-EVB-Pico
- **Firmware Version**: See `\dump_state` command on device

## License

BSD 3-clause License (New BSD License)
Copyright (c) Phil Crump 2025. All rights reserved.

---

**Last Updated**: 2025
**Maintained By**: Project Contributors
