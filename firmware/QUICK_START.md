# Quick Start - Flash Your RP2040

## Fastest Method (UF2 Bootloader)

1. **Hold BOOTSEL button** on your Pico
2. **Plug in USB** while holding the button
3. **Release button** - Pico appears as USB drive
4. **Run:**
   ```bash
   cargo run
   ```

That's it! Your firmware is now running.

## Alternative: Use Helper Script

```bash
./flash.sh
```

Choose option 1 and follow prompts.

## View Serial Output

```bash
screen /dev/cu.usbmodem* 115200
```

Press `Ctrl+A` then `K` to exit.

## Debug with Probe (if USB access is fixed)

1. Quit Kiro completely
2. Unplug/replug debug probe  
3. From terminal:
   ```bash
   probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
   ```

## Files Created

- **DEBUGGING_GUIDE.md** - Complete debugging documentation
- **DEBUG_SETUP.md** - Detailed troubleshooting steps
- **flash.sh** - Interactive flashing script
- **.vscode/launch.json** - Debug configurations for IDE
- **Probe.toml** - probe-rs configuration

## Need Help?

Read **DEBUGGING_GUIDE.md** for complete instructions and troubleshooting.
