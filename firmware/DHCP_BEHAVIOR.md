# DHCP Behavior

## Overview

The firmware attempts to obtain an IP address via DHCP on startup. If DHCP fails, the device continues operating without network functionality.

## Behavior Flow

```
Device Startup
    ↓
Initialize Hardware
    ↓
Start DHCP Request
    ↓
Wait up to 5 seconds
    ↓
    ├─→ DHCP Success
    │       ↓
    │   Start TCP Server (port 4533)
    │       ↓
    │   Accept HamLib Connections
    │       ↓
    │   Normal Operation
    │
    └─→ DHCP Timeout
            ↓
        Log Error Messages
            ↓
        Continue Without Network
            ↓
        ADC & Control Still Work
```

## DHCP Success

When DHCP succeeds:
- Device obtains IP address from DHCP server
- Logs: `"DHCP successful - IP address: x.x.x.x"`
- System LED switches to fast blink (0.5 seconds)
- TCP server starts on port 4533
- Up to 4 simultaneous HamLib connections accepted
- Sockets LED turns on when clients connect

## DHCP Failure

When DHCP fails (timeout after 5 seconds):
- Logs: `"DHCP failed after 5000ms timeout"`
- Logs: `"Network functionality disabled - check network connection"`
- Logs: `"Device will continue operating without network access"`
- System LED continues slow blink (1 second)
- TCP server is NOT started
- No network connections possible
- Sockets LED remains off

**Device continues to operate:**
- System LED blinks (heartbeat)
- ADC continues sampling azimuth/elevation
- Control outputs continue to function
- Watchdog continues to be fed

## Configuration

### Timeout Setting

Edit in `src/main.rs`:
```rust
const DHCP_TIMEOUT_MS:u64 = 5000;  // 5 seconds
```

Recommended values:
- **Fast networks**: 2000-3000ms
- **Normal networks**: 5000ms (default)
- **Slow networks**: 10000ms
- **Maximum**: Keep under 8000ms to avoid watchdog timeout

### DHCP Hostname

The device identifies itself as:
```rust
const DHCP_HOSTNAME:&str = "g5500-hamlib-adaptor";
```

This hostname can be used to find the device on your network.

## Troubleshooting

### DHCP Always Fails

**Check:**
1. Network cable is connected
2. DHCP server is running on your network
3. W5500 module is properly wired
4. Power supply is adequate

**Solutions:**
1. Increase timeout if your DHCP server is slow
2. Check router DHCP settings
3. Try a different network port
4. Configure static IP instead (see NETWORK_CONFIG.md)

### Need Static IP Instead

If you prefer static IP over DHCP, modify the network configuration in `main()`:

```rust
// Replace DHCP config:
let net_config = embassy_net::Config::dhcpv4(dhcp_config);

// With static config:
use embassy_net::{Ipv4Address, Ipv4Cidr, StaticConfigV4};
let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 100), 24),
    gateway: Some(Ipv4Address::new(192, 168, 1, 1)),
    dns_servers: heapless::Vec::new(),
});
```

Then you can remove the DHCP wait code entirely.

## Watchdog Considerations

The watchdog timeout is set to 8300ms, which is longer than the DHCP timeout (5000ms). This ensures:
- Device won't reset during DHCP wait
- Watchdog is fed before and after DHCP attempt
- Safe operation even if DHCP is slow

If you increase `DHCP_TIMEOUT_MS`, ensure it stays below `WATCHDOG_PERIOD_MS`.

## Recovery from DHCP Failure

If DHCP fails on startup, the device will NOT retry automatically. To restore network functionality:

1. **Fix the network issue** (connect cable, start DHCP server, etc.)
2. **Reset the device** using one of these methods:
   - Power cycle
   - Press reset button
   - Send HamLib reset command (if previously connected)
   - Trigger watchdog reset

The device will attempt DHCP again on the next boot.

## Monitoring

View DHCP status in real-time via defmt logs:

```bash
# Using probe-rs:
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor

# Or via serial after UF2 flash:
screen /dev/cu.usbmodem* 115200
```

Look for these messages:
- `"Waiting for DHCP..."`
- `"DHCP successful - IP address: x.x.x.x"` (success)
- `"DHCP failed after 5000ms timeout"` (failure)
