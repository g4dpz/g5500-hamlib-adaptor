# Hardware Architecture

## RP2040 Microcontroller

### Specifications
- **Architecture**: ARM Cortex-M0+
- **Clock Speed**: 125 MHz (configurable)
- **RAM**: 264KB (256KB main + 8KB scratch)
- **Flash**: 2MB
- **GPIO Pins**: 30 (28 usable)
- **ADC**: 4-channel 12-bit (3 GPIO + 1 temperature)
- **SPI**: 2 instances
- **UART**: 2 instances
- **I2C**: 2 instances
- **DMA**: 12 channels
- **Watchdog**: 8.3ms - 8.3s timeout
- **PIO**: 2 instances (8 state machines each)

## Board: W5500-EVB-Pico

### Features
- Raspberry Pi Pico form factor
- W5500 Ethernet controller (SPI-based)
- 10/100 Mbps Ethernet
- Integrated RJ45 connector
- 3.3V logic levels
- USB power input
- SWD debug connector

### Power
- **Input**: 5V via USB or external
- **3.3V Regulator**: 300mA max output
- **W5500 Draw**: ~130mA typical
- **Total Available**: ~170mA for other components

## Pin Assignments

### System & Debug
| Function | GPIO | Pin | Purpose |
|----------|------|-----|---------|
| System LED | 25 | 30 | Onboard LED (network status) |
| Sockets LED | 15 | 20 | External LED (client connections) |
| SWDIO | 24 | 28 | SWD debug (optional) |
| SWCLK | 25 | 29 | SWD debug (optional) |

### Rotator Control Outputs
| Function | GPIO | Pin | Purpose |
|----------|------|-----|---------|
| Az CW | 2 | 4 | Azimuth clockwise relay |
| Az CCW | 3 | 5 | Azimuth counter-clockwise relay |
| El UP | 4 | 6 | Elevation up relay |
| El DN | 5 | 7 | Elevation down relay |

**Characteristics:**
- 3.3V logic output
- Max 12mA per pin
- Require external relay drivers for rotator control
- Active high (set high to activate)

### Rotator Position Inputs (ADC)
| Function | GPIO | Pin | ADC Channel | Purpose |
|----------|------|-----|-------------|---------|
| Az Position | 26 | 31 | ADC0 | Azimuth voltage input |
| El Position | 27 | 32 | ADC1 | Elevation voltage input |

**Characteristics:**
- 12-bit ADC (0-4095 counts)
- 3.3V max input (requires voltage divider from 5V)
- 10kΩ/10kΩ voltage divider configured
- Sampling: 10kHz, 512 samples per 100ms

### W5500 Ethernet (SPI0)
| Function | GPIO | Pin | W5500 Pin | Purpose |
|----------|------|-----|-----------|---------|
| SPI CLK | 18 | 24 | SCK | SPI clock |
| SPI MOSI | 19 | 25 | MOSI | SPI data out |
| SPI MISO | 16 | 21 | MISO | SPI data in |
| SPI CS | 17 | 22 | CS/SS | Chip select |
| Interrupt | 21 | 27 | INT | Interrupt signal |
| Reset | 20 | 26 | RST | Reset signal |
| 3.3V | - | 36 | VCC | Power |
| GND | - | 38 | GND | Ground |

**Characteristics:**
- SPI frequency: 50 MHz
- DMA channels: CH0 (TX), CH1 (RX)
- Interrupt: Active low, pull-up enabled
- Reset: Active low, normally high

### Available GPIO (Unused)
- GPIO 0, 1: UART0 (available if not used)
- GPIO 6-14: Available
- GPIO 22: Available
- GPIO 28: Available

## Voltage Divider for ADC

### Purpose
G-5500 outputs 0-5V, but RP2040 ADC accepts 0-3.3V max.

### Configuration
```
G-5500 Output (0-5V)
    │
    ├─── 10kΩ ───┬─── To Pico ADC (GPIO 26/27)
                 │
                10kΩ
                 │
                GND
```

### Calculation
- Divider ratio: 10k/(10k+10k) = 0.5
- 5V input → 2.5V at ADC
- 0V input → 0V at ADC
- ADC range: 0-2.5V (0-2048 counts)

### Firmware Calibration
```rust
const UPPER_RESISTOR_K: f32 = 10.0;
const LOWER_RESISTOR_K: f32 = 10.0;
const LADDER_RATIO: f32 = LOWER_RESISTOR_K / (UPPER_RESISTOR_K + LOWER_RESISTOR_K);
const VREF_V: f32 = 3.3;
const G5500_VOLTAGE_LOW: f32 = 0.0;
const G5500_VOLTAGE_HIGH: f32 = 5.0;
```

## Relay Driver Interface

### Control Outputs
- **Type**: GPIO outputs (3.3V logic)
- **Current**: Max 12mA per pin
- **Voltage**: 3.3V high, 0V low

### Relay Requirements
- **Voltage**: Depends on rotator (typically 12V or 24V)
- **Current**: Depends on rotator (typically 100-500mA)
- **Driver**: Relay module or MOSFET driver required

### Example Relay Module
```
GPIO Output (3.3V)
    │
    ├─── 1kΩ resistor ───┬─── Relay module input
                         │
                        GND
```

## Power Distribution

### Power Budget
```
USB Input: 5V @ 500mA (typical)
    │
    ├─── RP2040 Core: ~50mA
    ├─── W5500: ~130mA
    ├─── Relays: ~200mA (external power recommended)
    └─── Other: ~20mA
    
Total: ~400mA (within USB limit)
```

### Recommendations
- **USB Power**: Sufficient for W5500 + RP2040
- **Relay Power**: Use external 12V/24V supply
- **Ground**: Common ground between all supplies
- **Decoupling**: 100nF capacitors near power pins

## Clock Configuration

### System Clock
- **Source**: Internal oscillator (ROSC)
- **Frequency**: 125 MHz (default)
- **Accuracy**: ±5% (sufficient for Ethernet)

### Peripheral Clocks
- **SPI**: 50 MHz (W5500 compatible)
- **ADC**: 48 MHz (internal)
- **Timer**: 1 MHz (for delays)

## Watchdog Timer

### Configuration
```rust
const WATCHDOG_PERIOD_MS: u64 = 8300;  // Max is 8388ms
watchdog.start(Duration::from_millis(WATCHDOG_PERIOD_MS));
```

### Behavior
- **Timeout**: 8.3 seconds
- **Action**: System reset
- **Feed**: Every 250ms in main loop
- **Purpose**: Prevent system hang

### DHCP Consideration
- DHCP timeout: 5 seconds
- Watchdog timeout: 8.3 seconds
- Margin: 3.3 seconds (safe)

## DMA Channels

### Allocation
- **CH0**: SPI0 TX (W5500)
- **CH1**: SPI0 RX (W5500)
- **CH2**: ADC (Az/El sampling)
- **CH3**: Flash operations
- **CH4-11**: Available

### ADC DMA
```rust
const NUM_SAMPLES: usize = 512;
const DIV: u16 = 2399;  // ~10kHz sample rate
let mut buf = [0_u16; NUM_SAMPLES * 2];  // 2 channels
adc.read_many_multichannel(&mut pins, &mut buf, DIV, &mut dma).await
```

## Interrupt Handling

### Configured Interrupts
- **ADC_IRQ_FIFO**: ADC data ready
- **W5500 INT**: Ethernet interrupt (GPIO 21)

### Binding
```rust
bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});
```

## Memory Mapping

### Flash
```
0x10000000: Boot2 (256 bytes)
0x10000100: Firmware start
0x101E0000: Config area (128KB)
0x10200000: End of flash
```

### RAM
```
0x20000000: Main RAM (256KB)
0x20040000: Scratch A (4KB, optional)
0x20041000: Scratch B (4KB, optional)
0x20042000: End of RAM
```

## Thermal Considerations

### Operating Temperature
- **Range**: 0°C to 50°C (typical)
- **Thermal Shutdown**: ~85°C
- **Cooling**: Passive (no active cooling)

### Power Dissipation
- **Typical**: ~200mW (W5500 + RP2040)
- **Peak**: ~400mW (with relays)
- **Thermal Resistance**: ~50°C/W (typical)

## Reliability Features

### Watchdog
- Prevents system hang
- 8.3s timeout
- Fed every 250ms

### Reset
- Power-on reset
- Watchdog reset
- Software reset (via HamLib command)

### Error Handling
- DHCP timeout (5s) with graceful failure
- Socket timeout (60s) per connection
- ADC DNL spike filtering
- Saturating arithmetic for counters

## Debugging Interface

### SWD (Serial Wire Debug)
- **SWDIO**: GPIO 24 (Pin 28)
- **SWCLK**: GPIO 25 (Pin 29)
- **Connector**: 3-pin header (optional)
- **Tool**: probe-rs or OpenOCD

### RTT (Real-Time Transfer)
- **Transport**: SWD
- **Speed**: Real-time logging
- **Tool**: probe-rs with RTT support
- **Log Level**: Configurable via DEFMT_LOG

### Serial (USB)
- **Type**: USB device (not implemented)
- **Alternative**: RTT via SWD
- **Baud Rate**: N/A (USB)

## Expansion Possibilities

### Available Resources
- GPIO: 0, 1, 6-14, 22, 28
- SPI: SPI1 (not used)
- UART: UART0, UART1 (not used)
- I2C: I2C0, I2C1 (not used)
- DMA: CH4-11 (not used)
- PIO: Both instances (not used)

### Future Enhancements
- Additional sensors (temperature, humidity)
- Local display (I2C OLED)
- Configuration storage (EEPROM)
- Backup power monitoring
- Additional relay outputs
