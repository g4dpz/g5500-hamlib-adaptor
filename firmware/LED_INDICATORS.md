# LED Indicators

## Overview

The device has two LED indicators that provide visual feedback about system status.

## System LED (GPIO 25 / PIN_25)

The system LED uses the Raspberry Pi Pico's onboard LED and indicates the network connection status through different blink rates:

### Blink Patterns

| Pattern | Rate | Meaning |
|---------|------|---------|
| **Slow Blink** | 1 second (1 Hz) | Waiting for network connection (DHCP in progress or failed) |
| **Fast Blink** | 0.5 seconds (2 Hz) | Network connected successfully |

### Behavior

**During Startup:**
- Blinks slowly (1 Hz) while waiting for DHCP
- Continues slow blink if DHCP fails
- Switches to fast blink (2 Hz) when DHCP succeeds

**During Operation:**
- Fast blink (2 Hz) = Network operational, ready for connections
- Slow blink (1 Hz) = No network connectivity

## Sockets LED (GPIO 15 / PIN_15)

The sockets LED is an external LED that indicates active client connections:

| State | Meaning |
|-------|---------|
| **OFF** | No clients connected |
| **ON** | One or more clients connected (max 4) |

### Behavior

- Turns ON when first client connects
- Remains ON while any client is connected
- Turns OFF when last client disconnects

## LED Summary

```
System LED (GPIO 25 - Onboard LED):
  Slow (1s)  = No network / Waiting for DHCP
  Fast (0.5s) = Network connected

Sockets LED (GPIO 15 - External LED):
  OFF = No clients
  ON  = Clients connected
```

## Hardware Setup

### System LED
- Uses the Raspberry Pi Pico's built-in LED (GPIO 25)
- No external components required

### Sockets LED
- Connect an external LED to GPIO 15 (PIN_15)
- Recommended: LED + 330Ω resistor to GND
- Or adjust the pin number in `src/main.rs` to match your hardware

## Troubleshooting

### System LED blinks slowly forever
- **Cause**: DHCP failed or no network connection
- **Check**: Network cable, DHCP server, W5500 module
- **See**: DHCP_BEHAVIOR.md for troubleshooting

### System LED blinks fast but Sockets LED stays off
- **Normal**: Network connected but no clients
- **Action**: Connect HamLib client to port 4533

### Both LEDs off
- **Cause**: Device not powered or firmware not running
- **Check**: Power supply, USB connection, firmware flash

### Sockets LED on but no response
- **Possible**: Client connected but communication issue
- **Check**: Firewall, correct IP address, port 4533

## Implementation

The system LED is controlled by a dedicated task that:
1. Checks network connection status
2. Adjusts blink rate accordingly
3. Runs independently of other tasks

The sockets LED is controlled by the main loop based on the socket counter.
