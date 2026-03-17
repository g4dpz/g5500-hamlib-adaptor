# Quick Reference Card

## LED Status at a Glance

```
┌─────────────────────────────────────────────────┐
│  System LED (GPIO 25 - Onboard)                 │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  Slow Blink (1s)   = No Network / Waiting      │
│  Fast Blink (0.5s) = Network Connected          │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  Sockets LED (GPIO 15 - External)               │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  OFF = No Clients                               │
│  ON  = Clients Connected (1-4)                  │
└─────────────────────────────────────────────────┘
```

## Network Configuration

| Setting | Value |
|---------|-------|
| **DHCP Hostname** | g5500-hamlib-adaptor |
| **DHCP Timeout** | 5 seconds |
| **TCP Port** | 4533 |
| **Max Clients** | 4 simultaneous |
| **Socket Timeout** | 60 seconds |

## Pin Assignments

| Function | GPIO | Pin | Notes |
|----------|------|-----|-------|
| System LED | 25 | PIN_25 | Onboard LED |
| Sockets LED | 15 | PIN_15 | External LED (configurable) |
| Az CW | 2 | PIN_2 | Azimuth clockwise |
| Az CCW | 3 | PIN_3 | Azimuth counter-clockwise |
| El UP | 4 | PIN_4 | Elevation up |
| El DN | 5 | PIN_5 | Elevation down |
| Az ADC | 26 | PIN_26 | Azimuth position (ADC0) |
| El ADC | 27 | PIN_27 | Elevation position (ADC1) |
| SPI CLK | 18 | PIN_18 | W5500 clock |
| SPI MOSI | 19 | PIN_19 | W5500 data out |
| SPI MISO | 16 | PIN_16 | W5500 data in |
| SPI CS | 17 | PIN_17 | W5500 chip select |
| W5500 INT | 21 | PIN_21 | W5500 interrupt |
| W5500 RST | 20 | PIN_20 | W5500 reset |

## HamLib Commands

| Command | Short | Description |
|---------|-------|-------------|
| `\get_info` | `_` | Get device info |
| `\get_pos` | `p` | Get current position |
| `\set_pos AZ EL` | `P AZ EL` | Set target position |
| `\stop` | `S` | Stop movement |
| `\park` | `K` | Park (180° Az, 0° El) |
| `\quit` | `q` | Close connection |
| `\dump_state` | - | Get detailed status |
| `\reset` | `R` | Reset device |

## Flashing Methods

### UF2 Bootloader (Easiest)
```bash
# 1. Hold BOOTSEL, plug in USB
# 2. Run:
cargo run
```

### Debug Probe
```bash
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
```

### Helper Script
```bash
./flash.sh
```

## Troubleshooting

| Symptom | Likely Cause | Solution |
|---------|--------------|----------|
| Slow LED blink | No network | Check cable, DHCP server |
| Fast LED, no clients | Normal | Connect HamLib client |
| Both LEDs off | No power | Check power/USB |
| Sockets LED on, no response | Connection issue | Check IP, port 4533 |

## Connecting with HamLib

```bash
# Find device IP (check router DHCP leases)
# Or use hostname: g5500-hamlib-adaptor

# Test connection
telnet 192.168.1.x 4533

# Use with rotctld
rotctld -m 202 -r 192.168.1.x:4533

# Use with gpredict
# Set rotator to "ROT2PROG" on port 4533
```

## Configuration Files

- `src/main.rs` - Main firmware code
- `Cargo.toml` - Build configuration
- `.cargo/config.toml` - Target and runner config
- `memory.x` - Memory layout

## Documentation

- `LED_INDICATORS.md` - LED behavior guide
- `NETWORK_CONFIG.md` - Network configuration
- `DHCP_BEHAVIOR.md` - DHCP operation details
- `DEBUGGING_GUIDE.md` - Debug setup
- `OPTIMIZATIONS.md` - Code optimizations
- `CHANGELOG.md` - Recent changes

## Build Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Check size
arm-none-eabi-size target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor

# Flash with UF2
cargo run

# Flash with probe
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/release/g5500-hamlib-adaptor
```

## Current Firmware Stats

- **Code Size**: ~124 KB
- **RAM Usage**: ~126 KB (static)
- **Flash Usage**: 12% of 2MB
- **Watchdog**: 8.3 seconds
- **ADC Rate**: 10 Hz (100ms)
- **Control Rate**: 4 Hz (250ms)
