# Technology Stack & Dependencies

## Core Language & Runtime
- **Rust**: Latest stable (1.85.1+)
- **Target**: `thumbv6m-none-eabi` (Cortex-M0/M0+)
- **Edition**: 2021
- **no_std**: Yes (embedded, no standard library)
- **no_main**: Yes (custom entry point)

## Async Runtime
- **Embassy**: Async executor for embedded systems
  - `embassy-executor`: Task scheduling and execution
  - `embassy-time`: Timer and delay utilities
  - `embassy-futures`: Future utilities
  - `embassy-sync`: Synchronization primitives (Mutex, Channel)

## Hardware Abstraction
- **embassy-rp**: RP2040-specific HAL
  - GPIO, SPI, ADC, Watchdog, Flash, USB
- **embassy-embedded-hal**: Generic embedded HAL traits
- **embedded-hal**: Standard embedded hardware traits
- **embedded-hal-async**: Async hardware traits
- **embedded-hal-bus**: Bus sharing utilities
- **embedded-io-async**: Async I/O traits

## Networking
- **embassy-net**: TCP/IP stack (smolTCP-based)
  - Features: TCP, DHCP, IPv4, Ethernet
- **embassy-net-wiznet**: W5500 Ethernet driver
- **embassy-usb**: USB device support

## Utilities & Libraries
- **defmt**: Embedded logging framework
- **defmt-rtt**: RTT transport for defmt
- **panic-probe**: Panic handler with probe-rs integration
- **cortex-m**: ARM Cortex-M utilities
- **cortex-m-rt**: Cortex-M runtime
- **critical-section**: Critical section implementation
- **static_cell**: Static cell allocation
- **portable-atomic**: Atomic operations
- **rand**: Random number generation (RoscRng)
- **log**: Logging facade
- **heapless**: No-std collections (Vec, String)
- **nom**: Parser combinator library (for HamLib protocol)
- **format_no_std**: Formatting without std
- **serde**: Serialization framework
- **serde-json-core**: JSON parsing (no_std)
- **fixed**: Fixed-point arithmetic
- **git-version**: Git version embedding

## Build & Development Tools
- **Cargo**: Rust package manager and build system
- **probe-rs**: SWD debugging and flashing
- **elf2uf2-rs**: ELF to UF2 conversion (optional)

## Build Profiles

### Release Profile (Production)
```toml
[profile.release]
debug = 2                    # Include debug symbols
lto = "fat"                  # Full link-time optimization
opt-level = 'z'              # Optimize for size
codegen-units = 1            # Single codegen unit
strip = false                # Keep symbols
panic = "abort"              # Abort on panic (smaller)
```

### Dev Profile (Development)
```toml
[profile.dev]
debug = 2                    # Include debug symbols
lto = true                   # Link-time optimization
opt-level = "z"              # Optimize for size
panic = "abort"              # Abort on panic
```

## Compiler Flags
- `--nmagic`: No magic number in linker
- `-Tlink.x`: ARM Cortex-M linker script
- `-Tlink-rp.x`: RP2040-specific linker script
- `-Tdefmt.x`: defmt linker script

## Memory Layout (RP2040)
- **Flash**: 2MB total
  - Boot2: 256 bytes
  - Firmware: ~1920KB
  - Config: 128KB (reserved)
- **RAM**: 264KB total
  - Used for stack, static data, buffers

## Key Dependencies Versions
- embassy-executor: 0.7.0
- embassy-rp: 0.4.0
- embassy-net: 0.7.0
- embassy-net-wiznet: 0.2.0
- defmt: 0.3
- cortex-m: 0.7.6
- embedded-hal: 1.0
- nom: 8.0.0
- heapless: 0.8.0

## Build Commands
```bash
# Build debug
cargo build

# Build release (optimized)
cargo build --release

# Flash and run
cargo run

# Check size
arm-none-eabi-size target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor

# Analyze binary
cargo bloat --release

# View dependencies
cargo tree
```

## Debugging Tools
- **probe-rs**: Primary debugger (SWD)
- **RTT**: Real-time transfer for logging
- **defmt**: Structured logging
- **OpenOCD**: Alternative debugger (optional)

## Environment Variables
- `DEFMT_LOG=debug`: Set defmt log level (debug, info, warn, error)
- `RUST_BACKTRACE=1`: Enable backtraces (if applicable)

## Target Specifications
- **Architecture**: ARM Cortex-M0+
- **Instruction Set**: Thumb-2
- **Floating Point**: Software (no FPU)
- **Endianness**: Little-endian
- **ABI**: EABI

## Optimization Strategy
- Size-optimized (`opt-level = 'z'`)
- Link-time optimization enabled
- Single codegen unit for better cross-crate optimization
- Panic abort instead of unwind (smaller code)
- Inline hints on hot paths
