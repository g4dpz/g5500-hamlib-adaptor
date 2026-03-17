# Complete Debugging Guide for RP2040

## The Problem
You're experiencing a "USB access error" when trying to use the Raspberry Pi Debug Probe with probe-rs on macOS. This is a known issue with CMSIS-DAP probes on macOS.

## Quick Solutions (Easiest First)

### Method 1: UF2 Bootloader (Recommended for macOS)
This is the easiest and most reliable method on macOS:

1. **Enter bootloader mode:**
   - Unplug your Pico
   - Hold the BOOTSEL button
   - Plug in the USB cable while holding BOOTSEL
   - Release the button
   - Your Pico will appear as a USB drive named "RPI-RP2"

2. **Flash the firmware:**
   ```bash
   cargo run
   ```
   
   The firmware will automatically be converted to UF2 format and flashed.

3. **View logs:**
   ```bash
   # After flashing, connect to serial port
   screen /dev/cu.usbmodem* 115200
   # Press Ctrl+A then K to exit screen
   ```

### Method 2: Use the Flash Script
I've created a helper script:

```bash
./flash.sh
```

Choose option 1 for UF2 bootloader (easiest).

### Method 3: Fix probe-rs USB Access
If you really need the debug probe to work:

1. **Completely quit Kiro:**
   ```bash
   # Quit Kiro (Cmd+Q)
   # Then from terminal:
   killall "Kiro Helper" 2>/dev/null || true
   ```

2. **Unplug and replug the debug probe**

3. **Try from terminal:**
   ```bash
   probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
   ```

4. **If still failing, check for conflicts:**
   ```bash
   lsof 2>/dev/null | grep -i "2e8a:000c"
   ```

## Debugging Methods Comparison

| Method | Pros | Cons | Best For |
|--------|------|------|----------|
| **UF2 Bootloader** | ✅ Always works<br>✅ No extra hardware<br>✅ Fast flashing | ❌ No breakpoints<br>❌ No step debugging<br>❌ Manual button press | Quick testing, macOS users |
| **probe-rs** | ✅ Full debugging<br>✅ Breakpoints<br>✅ Fast | ❌ USB issues on macOS<br>❌ Requires debug probe | When debugging works |
| **OpenOCD** | ✅ Mature tool<br>✅ Better macOS support | ❌ Slower<br>❌ More complex setup | When probe-rs fails |

## Using Debugging in Kiro IDE

Once you fix the USB access issue:

1. **Open Run and Debug panel:** Cmd+Shift+D
2. **Select configuration:** "Debug RP2040 (probe-rs)"
3. **Start debugging:** Press F5

### Setting Breakpoints
- Click in the gutter (left of line numbers) to set breakpoints
- Red dots indicate active breakpoints

### Debug Controls
- **Continue (F5):** Run until next breakpoint
- **Step Over (F10):** Execute current line
- **Step Into (F11):** Step into function calls
- **Step Out (Shift+F11):** Step out of current function

## Viewing Logs (RTT)

Your firmware uses defmt for logging. To see logs:

### With probe-rs:
```bash
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
```

Logs will appear in the terminal automatically.

### With UF2 bootloader:
You'll need to add USB serial output or use a separate UART connection.

## Troubleshooting

### "Failed to open the debug probe"
- **Cause:** USB access conflict (common on macOS)
- **Fix:** Quit Kiro, unplug/replug probe, try from terminal

### "No probe found"
- **Check:** `probe-rs list`
- **Fix:** Check USB connection, try different port

### "Could not find target"
- **Cause:** Wiring issue or target not powered
- **Fix:** Check SWDIO, SWCLK, GND connections

### "Permission denied" on /dev/cu.usbmodem*
```bash
sudo chmod 666 /dev/cu.usbmodem*
```

## Hardware Connections

### Debug Probe to Pico:
```
Debug Probe    →    Pico
─────────────────────────
SWDIO          →    SWDIO (GPIO 24 / Pin 31)
SWCLK          →    SWCLK (GPIO 25 / Pin 34)  
GND            →    GND (any GND pin)
VREF           →    3V3 (Pin 36) [optional]
```

### Important Notes:
- VREF connection is optional if Pico is USB powered
- Make sure both devices share a common ground
- Use short wires for reliable connection

## Configuration Files Created

- **`.vscode/launch.json`** - Debug configurations for Kiro IDE
- **`.vscode/tasks.json`** - Build tasks
- **`Probe.toml`** - probe-rs configuration
- **`flash.sh`** - Helper script for flashing
- **`.cargo/config.toml`** - Updated to use UF2 by default

## Recommended Workflow

For development on macOS:

1. **Quick testing:** Use UF2 bootloader (`cargo run`)
2. **Debugging:** Try probe-rs from terminal first
3. **If probe-rs fails:** Use OpenOCD or continue with UF2

## Getting Help

If you're still stuck:

1. Check probe-rs version: `probe-rs --version`
2. List probes: `probe-rs list`
3. Check target info: `probe-rs info --chip RP2040`
4. Check for conflicts: `lsof 2>/dev/null | grep -i usb`

Share the output of these commands for further troubleshooting.
