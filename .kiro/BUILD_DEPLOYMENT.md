# Build System & Deployment

## Build System Overview

### Cargo Configuration
- **Workspace**: Single package (firmware)
- **Edition**: 2021
- **Target**: `thumbv6m-none-eabi` (Cortex-M0+)
- **Runner**: `probe-rs run --chip RP2040` (primary)

### Build Script (build.rs)
Handles:
1. Copying `memory.x` to output directory
2. Setting linker search paths
3. Configuring linker scripts
4. Setting compiler flags

**Linker Scripts Used:**
- `link.x`: ARM Cortex-M generic
- `link-rp.x`: RP2040-specific
- `defmt.x`: defmt logging support

## Build Profiles

### Release Build (Production)
```bash
cargo build --release
```

**Optimizations:**
- `lto = "fat"`: Full link-time optimization
- `opt-level = 'z'`: Optimize for size
- `codegen-units = 1`: Single codegen unit
- `panic = "abort"`: Smaller panic handler
- `debug = 2`: Include debug symbols

**Output Size:** ~244KB (12% of 2MB flash)

### Debug Build (Development)
```bash
cargo build
```

**Optimizations:**
- `lto = true`: Link-time optimization
- `opt-level = "z"`: Optimize for size
- `panic = "abort"`: Abort on panic
- `debug = 2`: Include debug symbols

## Flashing Methods

### Method 1: UF2 Bootloader (Recommended - macOS Compatible)
```bash
# Hold BOOTSEL button, plug in USB, release button
# Pico appears as USB drive

cargo run
```

**Advantages:**
- No special hardware required
- Works on macOS without USB issues
- Simple and reliable
- Automatic via cargo runner

### Method 2: probe-rs (SWD Debugger)
```bash
# Requires SWD debug probe connected
cargo run
```

**Advantages:**
- Real-time debugging
- RTT logging
- Breakpoints and stepping
- Faster flashing

**Disadvantages:**
- Requires debug probe hardware
- macOS USB access issues (workaround: quit IDE)

### Method 3: Helper Script
```bash
./flash.sh
```

**Interactive script with options:**
1. UF2 bootloader method
2. probe-rs method
3. Manual flashing

## Cargo Commands

### Build
```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check
```

### Run & Flash
```bash
# Build, flash, and run (uses configured runner)
cargo run

# Release version
cargo run --release
```

### Analysis
```bash
# Check binary size
arm-none-eabi-size target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor

# Analyze what's taking space
cargo bloat --release

# View dependency tree
cargo tree

# Check for unused dependencies
cargo udeps
```

### Cleaning
```bash
# Clean build artifacts
cargo clean

# Clean specific target
cargo clean --release
```

## Environment Configuration

### .cargo/config.toml
```toml
[target.'cfg(all(target_arch = "arm", target_os = "none"))']
runner = "probe-rs run --chip RP2040"

[build]
target = "thumbv6m-none-eabi"

[env]
DEFMT_LOG = "debug"
```

### Probe.toml
```toml
[default.general]
chip = "RP2040"

[default.flashing]
enabled = true
halt_afterwards = false

[default.reset]
enabled = true
halt_afterwards = true

[default.rtt]
enabled = true
up_mode = "NoBlockSkip"
timeout = 3000
```

## Memory Layout

### Flash (2MB)
```
0x10000000 - 0x10000100: Boot2 (256 bytes)
0x10000100 - 0x101E0000: Firmware (~1920KB)
0x101E0000 - 0x10200000: Config (128KB, reserved)
```

### RAM (264KB)
```
0x20000000 - 0x20040000: Main RAM (256KB)
0x20040000 - 0x20041000: Scratch A (4KB, optional)
0x20041000 - 0x20042000: Scratch B (4KB, optional)
```

**Current Usage:**
- BSS (static data): ~126KB (48%)
- Stack: Allocated from remaining
- Heap: Not used (no_std)

## Deployment Checklist

### Pre-Deployment
- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] Binary size acceptable (<1.5MB)
- [ ] Memory usage verified
- [ ] Git version embedded correctly
- [ ] Release build tested

### Flashing
- [ ] Device connected (USB or SWD)
- [ ] Correct board selected (W5500-EVB-Pico)
- [ ] Firmware flashed successfully
- [ ] Device boots and runs

### Post-Deployment
- [ ] System LED blinks (network status)
- [ ] DHCP succeeds (if network available)
- [ ] TCP server listening on port 4533
- [ ] HamLib commands respond correctly
- [ ] ADC readings stable
- [ ] Control outputs functional

## Troubleshooting Build Issues

### Compilation Errors
```bash
# Clean and rebuild
cargo clean
cargo build

# Check for dependency issues
cargo update
cargo build
```

### Linker Errors
- Verify `memory.x` exists
- Check `build.rs` is correct
- Ensure linker scripts are present

### Size Issues
```bash
# Check what's taking space
cargo bloat --release

# Reduce buffer sizes if needed
# Edit constants in src/main.rs
```

### Runtime Issues
```bash
# View logs via RTT
cargo run --release

# Check defmt output
# Adjust DEFMT_LOG level in .cargo/config.toml
```

## Continuous Integration

### Build Verification
```bash
cargo check
cargo build --release
arm-none-eabi-size target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor
```

### Code Quality
```bash
cargo clippy
cargo fmt --check
```

## Version Management

### Git Version Embedding
```rust
const GIT_VERSION: &str = git_version!(
    args = ["--dirty", "--always"],
    fallback = "nogit"
);
```

**Includes:**
- Commit hash
- Dirty flag (if uncommitted changes)
- Fallback if not in git repo

**View via:**
```bash
# Connect to device and send \dump_state command
# Firmware Version: <git-version>
```

## Performance Optimization

### Binary Size
- Current: ~244KB (12% of flash)
- Optimization: `opt-level = 'z'` + LTO
- Further: Reduce buffer sizes, remove unused features

### Runtime Performance
- ADC sampling: 10kHz (100ms per sample)
- Control loop: 250ms tick
- Network: Async, non-blocking
- Watchdog: 8.3s timeout

### Memory Usage
- RAM: ~126KB static + stack
- Headroom: ~138KB available
- Buffers: 1KB TCP rx/tx, 256B command

## Debugging Deployment

### Serial Output
```bash
# View defmt logs via RTT
cargo run --release

# Or via serial (if USB serial available)
screen /dev/cu.usbmodem* 115200
```

### Network Testing
```bash
# Connect to device
telnet <ip-address> 4533

# Send HamLib commands
_
p
\dump_state
```

### Hardware Verification
```bash
# Check device is running
# System LED should blink (1Hz = no network, 2Hz = network OK)
# Sockets LED should be off (no clients)

# Connect client
# Sockets LED should turn on
```

## Release Process

1. **Prepare**
   - Update CHANGELOG.md
   - Verify all tests pass
   - Build release binary

2. **Build**
   ```bash
   cargo build --release
   ```

3. **Verify**
   ```bash
   arm-none-eabi-size target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor
   cargo bloat --release
   ```

4. **Test**
   - Flash to device
   - Verify all features work
   - Check logs for errors

5. **Tag**
   ```bash
   git tag -a v0.1.0 -m "Release version 0.1.0"
   git push origin v0.1.0
   ```

6. **Document**
   - Update README with version
   - Document any breaking changes
   - Add release notes
