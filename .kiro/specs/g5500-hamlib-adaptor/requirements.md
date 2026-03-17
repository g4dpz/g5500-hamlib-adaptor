# Requirements: G-5500 Hamlib Adaptor

## Overview
Network-connected embedded firmware providing a HamLib rotctld TCP interface for the Yaesu G-5500 Az+El rotator, targeting the W5500-EVB-Pico (RP2040) board. Primary use case is satellite tracking via Gpredict.

## Functional Requirements

### FR-1: HamLib rotctld Protocol Support
The firmware shall implement a partial rotctld protocol over TCP on port 4533.

#### FR-1.1: Get Info
- Support `_` and `\get_info` commands
- Return product name and firmware git version in HamLib info format

#### FR-1.2: Get Position
- Support `p` and `\get_pos` commands
- Return current azimuth and elevation in degrees (format: `AZ\nEL\n`)

#### FR-1.3: Set Position
- Support `P AZ EL` and `\set_pos AZ EL` commands
- Accept floating-point azimuth (0–450°) and elevation (0–180°)
- Clamp values to valid ranges
- Return `RPRT 0\n` on success

#### FR-1.4: Stop
- Support `S` and `\stop` commands
- Immediately halt all rotator movement
- Return `RPRT 0\n`

#### FR-1.5: Park
- Support `K` and `\park` commands
- Move rotator to park position (Az: 180°, El: 0°)
- Return `RPRT 0\n`

#### FR-1.6: Quit
- Support `q` and `\quit` commands
- Gracefully close the TCP connection

#### FR-1.7: Dump State
- Support `\dump_state` command
- Return diagnostics: product name, firmware version, flash UUID, uptime, connected clients, current/demand positions, raw ADC values

#### FR-1.8: Reset
- Support `R` and `\reset` commands
- Trigger a full system reset via watchdog

#### FR-1.9: Error Response
- Return `RPRT 1\n` for unrecognized commands

### FR-2: Concurrent TCP Connections
- Support up to 4 simultaneous HamLib client connections on port 4533
- Each socket shall have a 60-second idle timeout
- Track connected socket count in shared state

### FR-3: Rotator Position Monitoring (ADC)
- Read azimuth (GPIO 26 / ADC0) and elevation (GPIO 27 / ADC1) via multichannel ADC with DMA
- Sample at 10kHz, 512 samples per 100ms cycle, averaged
- Filter RP2040 ADC DNL spikes at known values (512, 1536, 2560, 3584)
- Convert raw ADC to degrees using voltage divider calibration (10kΩ/10kΩ, 0–5V G-5500 output mapped through 0.5 ratio to 3.3V ADC ref)
- Clamp azimuth to 0–450° and elevation to 0–180°

### FR-4: Rotator Control (Relay Outputs)
- Drive 4 GPIO relay outputs: Az CW (GPIO 2), Az CCW (GPIO 3), El UP (GPIO 4), El DN (GPIO 5)
- Control loop runs on a 250ms tick
- Move toward demand position; stop when within ±3° threshold
- Automatically clear demand run flag when on-target
- All relays off when stopped

### FR-5: Network (Ethernet / DHCP)
- Connect via W5500 Ethernet over SPI0 at 50MHz
- Obtain IP address via DHCP with hostname `g5500-hamlib-adaptor`
- DHCP timeout of 5 seconds; if DHCP fails, device continues operating without network
- TCP socket services only started on successful DHCP

### FR-6: LED Status Indicators
- System LED (GPIO 25, onboard): blink at 1s interval (no network) or 0.5s interval (network connected)
- Sockets LED (GPIO 15, external): on when ≥1 client connected, off otherwise

### FR-7: Watchdog
- Watchdog timer with 8.3s timeout (max 8388ms)
- Fed every 250ms in main loop
- System reset triggered on timeout or explicit reset command

### FR-8: Flash UUID
- Read 8-byte unique ID from RP2040 flash at startup
- Expose in `\dump_state` diagnostics

## Non-Functional Requirements

### NFR-1: no_std / No Heap
- Firmware must run without standard library or heap allocation
- All buffers and state statically allocated

### NFR-2: Binary Size
- Release binary shall fit within 2MB flash (current: ~244KB, 12%)
- Build with `opt-level = 'z'`, fat LTO, single codegen unit, panic=abort

### NFR-3: Memory Usage
- Static RAM usage shall not exceed 200KB of the 264KB available
- TCP buffers: 1KB rx + 1KB tx per socket; command buffer: 256 bytes

### NFR-4: Target Platform
- W5500-EVB-Pico board only (not W55RP20-EVB-PICO)
- RP2040 Cortex-M0+ at 125MHz default clock

### NFR-5: Reliability
- Watchdog prevents system hang
- Saturating arithmetic on counters to prevent overflow
- Graceful DHCP failure (device remains functional)
- Socket idle timeout prevents resource leaks

### NFR-6: Licensing
- BSD 3-clause License
- Copyright Phil Crump 2025
