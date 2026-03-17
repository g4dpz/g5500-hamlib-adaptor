#!/bin/bash
# Flash script for RP2040

set -e

echo "Building firmware..."
cargo build

echo ""
echo "Choose flashing method:"
echo "1) UF2 Bootloader (hold BOOTSEL, plug in USB)"
echo "2) Debug Probe (probe-rs)"
echo "3) Debug Probe (OpenOCD)"
read -p "Enter choice [1-3]: " choice

case $choice in
    1)
        echo ""
        echo "Using UF2 bootloader..."
        echo "Make sure your Pico is in BOOTSEL mode (hold button while plugging in)"
        read -p "Press Enter when ready..."
        elf2uf2-rs --deploy --serial --verbose target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
        ;;
    2)
        echo ""
        echo "Using probe-rs..."
        probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
        ;;
    3)
        echo ""
        echo "Using OpenOCD..."
        if ! command -v openocd &> /dev/null; then
            echo "OpenOCD not found. Install with: brew install openocd"
            exit 1
        fi
        openocd -f interface/cmsis-dap.cfg -f target/rp2040.cfg \
            -c "adapter speed 5000" \
            -c "program target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor verify reset exit"
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "Done!"
