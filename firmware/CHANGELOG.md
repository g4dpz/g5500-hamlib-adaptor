# Changelog

## Recent Changes

### Corrected LED Pins for Raspberry Pi Pico (Latest)

**Updated LED pin assignments:**

- System LED now uses GPIO 25 (onboard LED on Raspberry Pi Pico)
- Sockets LED moved to GPIO 15 (external LED, configurable)
- No external LED needed for system status - uses built-in LED
- Updated all documentation with correct pin assignments

**Hardware changes:**
- System LED: PIN_6 → PIN_25 (GPIO 25, onboard)
- Sockets LED: PIN_7 → PIN_15 (GPIO 15, external)

**Documentation:**
- Created `HARDWARE_WIRING.md` with complete wiring guide
- Updated `LED_INDICATORS.md`, `NETWORK_CONFIG.md`, `QUICK_REFERENCE.md`

### LED Status Indicators

**Added intelligent LED status indication:**

- System LED now indicates network connection status
- Slow blink (1 second) = Waiting for network / DHCP failed
- Fast blink (0.5 seconds) = Network connected successfully
- Dedicated LED task for independent operation
- Sockets LED continues to show client connections

**Benefits:**
- Visual feedback of network status at a glance
- Easy troubleshooting without serial connection
- Clear indication of DHCP success/failure
- Independent task doesn't block other operations

**See:** `LED_INDICATORS.md` for complete LED behavior guide

### Code Optimizations

**Optimized firmware for size and performance:**

- Reduced code size by 596 bytes (124,160 → 123,564 bytes)
- Reduced TCP buffer sizes (4KB → 1KB rx/tx, 4KB → 256B command buffer)
- Saved ~96KB of stack memory across socket tasks
- Optimized mutex access patterns (eliminated unnecessary cloning)
- Simplified boolean comparisons and control flow
- Added inline hints to hot path functions
- Improved ADC loop efficiency
- Enhanced compiler optimizations (fat LTO, single codegen unit)

**Performance improvements:**
- Faster command parsing with simplified control flow
- Reduced memory allocations in critical paths
- More efficient float clamping operations
- Optimized LED control logic

**See:** `OPTIMIZATIONS.md` for detailed analysis

## Recent Changes

### DHCP Graceful Failure Handling (Latest)

**Added graceful DHCP failure handling:**

- Device now has a 5-second timeout for DHCP configuration
- If DHCP fails, device continues operating without network functionality
- Prevents device from hanging indefinitely if no DHCP server is available
- Logs clearly indicate whether DHCP succeeded or failed
- Network services (TCP sockets) are only started if DHCP succeeds

**Configuration constant:**
```rust
const DHCP_TIMEOUT_MS:u64 = 5000;  // DHCP timeout in milliseconds
```

**Benefits:**
- Device continues to operate even without network connectivity
- Clear error reporting when DHCP fails
- Watchdog-friendly (won't timeout during DHCP wait)
- ADC and control functions remain operational
- Simple and predictable behavior

**See:** `NETWORK_CONFIG.md` for detailed configuration options

### Build System Fixes

**Fixed compilation errors:**
- Corrected RP2040 peripheral naming (UART1 vs USART1)
- Fixed embassy-rp API compatibility issues
- Resolved task signature constraints (no generics, static lifetimes)
- Fixed SPI configuration for W5500

### Debug Configuration

**Added comprehensive debugging support:**
- Created `.vscode/launch.json` with probe-rs and OpenOCD configurations
- Added `flash.sh` helper script for easy flashing
- Configured UF2 bootloader as default flashing method (macOS compatible)
- Created detailed debugging guides

**Files added:**
- `DEBUGGING_GUIDE.md` - Complete debugging documentation
- `DEBUG_SETUP.md` - Troubleshooting for probe-rs issues
- `QUICK_START.md` - Fast reference for flashing
- `flash.sh` - Interactive flashing script
- `Probe.toml` - probe-rs configuration

## Known Issues

### macOS Debug Probe USB Access
- probe-rs may fail with "USB access error" on macOS
- Workaround: Use UF2 bootloader method or quit Kiro IDE before using probe-rs
- See `DEBUGGING_GUIDE.md` for solutions

## Future Enhancements

Potential improvements:
- [ ] mDNS/Bonjour support for easy device discovery
- [ ] Web interface for configuration
- [ ] EEPROM storage for network settings
- [ ] Multiple static IP profiles
- [ ] Link status detection and auto-recovery
