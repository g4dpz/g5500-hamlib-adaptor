# Hardware Wiring Guide

## Raspberry Pi Pico Pin Connections

### LEDs

#### System LED (Built-in)
- **GPIO 25** - Onboard LED (no wiring needed)
- Indicates network connection status
- Slow blink = No network
- Fast blink = Network connected

#### Sockets LED (External)
- **GPIO 15 (Pin 20)** - Sockets status LED
- Connect: GPIO 15 → LED (anode) → 330Ω resistor → GND
- Or adjust pin in code to match your hardware
- ON = Clients connected
- OFF = No clients

### Rotator Control Outputs

| Function | GPIO | Physical Pin | Connection |
|----------|------|--------------|------------|
| Az CW | 2 | Pin 4 | Azimuth clockwise relay/driver |
| Az CCW | 3 | Pin 5 | Azimuth counter-clockwise relay/driver |
| El UP | 4 | Pin 6 | Elevation up relay/driver |
| El DN | 5 | Pin 7 | Elevation down relay/driver |

**Note:** These outputs are 3.3V logic. Use appropriate relay modules or motor drivers rated for your rotator's voltage/current requirements.

### Rotator Position Inputs (ADC)

| Function | GPIO | Physical Pin | ADC Channel | Connection |
|----------|------|--------------|-------------|------------|
| Az Position | 26 | Pin 31 | ADC0 | Azimuth voltage (0-3.3V) |
| El Position | 27 | Pin 32 | ADC1 | Elevation voltage (0-3.3V) |

**Voltage Divider Required:**
- G-5500 outputs 0-5V
- Pico ADC accepts 0-3.3V max
- Use 10kΩ/10kΩ voltage divider (configured in firmware)

```
G-5500 Output (0-5V)
    │
    ├─── 10kΩ ───┬─── To Pico ADC (GPIO 26/27)
                 │
                10kΩ
                 │
                GND
```

### W5500 Ethernet Module (SPI)

| Function | GPIO | Physical Pin | W5500 Pin |
|----------|------|--------------|-----------|
| SPI CLK | 18 | Pin 24 | SCK |
| SPI MOSI | 19 | Pin 25 | MOSI |
| SPI MISO | 16 | Pin 21 | MISO |
| SPI CS | 17 | Pin 22 | CS/SS |
| Interrupt | 21 | Pin 27 | INT |
| Reset | 20 | Pin 26 | RST |
| 3.3V | 3V3 | Pin 36 | VCC |
| Ground | GND | Pin 38 | GND |

**Important:**
- W5500 requires stable 3.3V power
- Consider separate power supply if using USB power
- Keep SPI wires short for reliable communication
- SPI frequency: 50 MHz

### Power

| Pin | Function | Notes |
|-----|----------|-------|
| VBUS (Pin 40) | 5V USB power | When powered via USB |
| VSYS (Pin 39) | System voltage | 1.8-5.5V external power |
| 3V3 (Pin 36) | 3.3V output | Max 300mA total |
| GND | Ground | Multiple GND pins available |

**Power Considerations:**
- USB power: 5V via VBUS
- External power: 1.8-5.5V via VSYS
- 3.3V regulator output: 300mA max
- W5500 draws ~130mA typical
- Leave headroom for other components

## Complete Wiring Diagram

```
Raspberry Pi Pico
┌─────────────────────────────────────┐
│                                     │
│  GPIO 25 (Onboard LED) ●            │  System Status
│                                     │
│  GPIO 15 ●─────────────────────────┼──→ External LED + 330Ω → GND
│                                     │
│  GPIO 2  ●─────────────────────────┼──→ Az CW Relay
│  GPIO 3  ●─────────────────────────┼──→ Az CCW Relay
│  GPIO 4  ●─────────────────────────┼──→ El UP Relay
│  GPIO 5  ●─────────────────────────┼──→ El DN Relay
│                                     │
│  GPIO 26 ●─────────────────────────┼──→ Az Position (via divider)
│  GPIO 27 ●─────────────────────────┼──→ El Position (via divider)
│                                     │
│  GPIO 16 ●─────────────────────────┼──→ W5500 MISO
│  GPIO 17 ●─────────────────────────┼──→ W5500 CS
│  GPIO 18 ●─────────────────────────┼──→ W5500 SCK
│  GPIO 19 ●─────────────────────────┼──→ W5500 MOSI
│  GPIO 20 ●─────────────────────────┼──→ W5500 RST
│  GPIO 21 ●─────────────────────────┼──→ W5500 INT
│                                     │
│  3V3     ●─────────────────────────┼──→ W5500 VCC
│  GND     ●─────────────────────────┼──→ Common Ground
│                                     │
└─────────────────────────────────────┘
```

## Customizing Pin Assignments

To change the sockets LED pin, edit `src/main.rs`:

```rust
// Change this line:
let mut sockets_led = Output::new(p.PIN_15, Level::Low);

// To your desired pin, e.g.:
let mut sockets_led = Output::new(p.PIN_14, Level::Low);
```

Available GPIO pins (not used by this project):
- GPIO 0, 1, 6-15, 22, 28

## Safety Notes

1. **Never exceed 3.3V on any GPIO pin** - Pico is NOT 5V tolerant
2. **Use voltage dividers** for 5V signals (G-5500 position outputs)
3. **Use appropriate drivers** for rotator control (relays, MOSFETs, etc.)
4. **Isolate high voltage** circuits from Pico
5. **Check current limits** - GPIO max 12mA, total 50mA per bank
6. **Use external power** for relays and motors

## Testing

### LED Test
1. Power on Pico
2. Onboard LED should start blinking slowly (1 Hz)
3. If external LED connected to GPIO 15, it should be off

### Network Test
1. Connect Ethernet cable
2. Onboard LED should switch to fast blink (2 Hz) when DHCP succeeds
3. External LED turns on when client connects

### ADC Test
Use `\dump_state` command to view raw ADC values and verify position readings.

### Control Test
Use `\set_pos` command to test relay outputs (ensure rotator is safe to move).

## Troubleshooting

### No LEDs
- Check power supply
- Verify firmware is flashed
- Check USB connection

### Onboard LED not blinking
- Firmware may not be running
- Check for build errors
- Try reflashing

### External LED not working
- Check wiring (GPIO 15 → LED → resistor → GND)
- Verify LED polarity (anode to GPIO, cathode to resistor)
- Check resistor value (330Ω recommended)
- Try different GPIO pin and update code

### W5500 not working
- Check SPI wiring
- Verify 3.3V power supply
- Check ground connection
- Ensure short, direct wiring
- Try lower SPI frequency in code

### ADC readings incorrect
- Verify voltage divider values
- Check G-5500 output voltage
- Ensure common ground
- Check for loose connections
