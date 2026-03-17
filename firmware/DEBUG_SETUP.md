# Debugging RP2040 with Raspberry Pi Debug Probe

## Current Status
- ✅ probe-rs installed (v0.24.0)
- ✅ Debug probe detected: CMSIS-DAP (2e8a:000c)
- ✅ RP2040 chip detected via SWD
- ❌ USB access error when trying to flash (known macOS CMSIS-DAP issue)

## Quick Solution: Use UF2 Bootloader Instead

The easiest way to flash your RP2040 on macOS is using the UF2 bootloader:

1. Hold the BOOTSEL button on your Pico while plugging it in
2. It will appear as a USB drive
3. Run: `cargo run`
4. The firmware will be automatically converted to UF2 and flashed

This method is now configured as the default runner in `.cargo/config.toml`.

## Fixing the probe-rs USB Access Error on macOS

This is a known issue with CMSIS-DAP probes on macOS. Here are solutions:

### Solution 1: Restart Kiro IDE
The Kiro cortex-debug extension may be holding the USB device:
1. Quit Kiro completely (Cmd+Q)
2. Unplug and replug the debug probe
3. Restart Kiro
4. Try debugging again

### Solution 2: Kill Conflicting Processes
```bash
# Find processes using USB
lsof 2>/dev/null | grep -i "usb" | grep -i "kiro\|code"

# If you find any, note the PID and kill them
# kill -9 <PID>
```

### Solution 3: Use probe-rs from Terminal
Sometimes probe-rs works from terminal but not from IDE:
```bash
# Build first
cargo build

# Then flash and debug from terminal
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
```

### Solution 4: Update probe-rs
```bash
cargo install probe-rs --locked --force
```

### Solution 5: Try Different USB Port
Some USB hubs or ports can cause issues. Try:
- A different USB port on your Mac
- Connecting directly (not through a hub)
- Using a different USB cable

### Option 5: Use OpenOCD Instead
If probe-rs continues to have issues, you can use OpenOCD:

1. Install OpenOCD:
```bash
brew install openocd
```

2. Flash with OpenOCD:
```bash
openocd -f interface/cmsis-dap.cfg -f target/rp2040.cfg -c "adapter speed 5000" -c "program target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor verify reset exit"
```

## Using the Debug Configurations

### In Kiro IDE:
1. Open the Run and Debug panel (Cmd+Shift+D)
2. Select "Debug RP2040 (probe-rs)" from the dropdown
3. Press F5 or click the green play button

### Available Configurations:
- **Debug RP2040 (probe-rs)**: Uses probe-rs (recommended, faster)
- **Debug RP2040 (OpenOCD)**: Uses OpenOCD (fallback option)

## Manual Debugging Commands

### Flash and Run:
```bash
cargo run
```

### Flash Only:
```bash
probe-rs download --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
```

### Attach Debugger:
```bash
probe-rs attach --chip RP2040
```

### View RTT Logs:
```bash
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
```

## Wiring Check

Make sure your Raspberry Pi Debug Probe is connected correctly:
- **SWDIO** → Target SWDIO
- **SWCLK** → Target SWCLK  
- **GND** → Target GND
- **VREF** → Target 3.3V (or leave disconnected if target is self-powered)

## Common Issues

### "Failed to open the debug probe"
- Try with sudo
- Close other applications using the USB device
- Unplug/replug the debug probe
- Check USB cable quality

### "No probe found"
- Check USB connection
- Try a different USB port
- Check if the debug probe LED is on

### "Could not find target"
- Check wiring between debug probe and target
- Ensure target is powered
- Try slower adapter speed: `adapter speed 1000`
