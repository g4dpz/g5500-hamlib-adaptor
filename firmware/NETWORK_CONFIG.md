# Network Configuration

## DHCP with Graceful Failure

The firmware is configured to use DHCP by default. If DHCP fails, the device continues operating without network functionality.

### Default Behavior

1. **On startup**, the device attempts to obtain an IP address via DHCP
2. **Hostname**: `g5500-hamlib-adaptor` (configurable in `src/main.rs`)
3. **Timeout**: 5 seconds (configurable via `DHCP_TIMEOUT_MS`)
4. **If DHCP succeeds**: Device starts TCP server and accepts HamLib connections
5. **If DHCP fails**: Device continues operating but network functionality is disabled

### DHCP Timeout Configuration

Edit this constant in `src/main.rs`:

```rust
const DHCP_TIMEOUT_MS:u64 = 5000;  // DHCP timeout in milliseconds
```

### Examples

**Increase DHCP timeout to 10 seconds:**
```rust
const DHCP_TIMEOUT_MS:u64 = 10000;
```

**Decrease timeout to 2 seconds:**
```rust
const DHCP_TIMEOUT_MS:u64 = 2000;
```

## Monitoring Network Status

The firmware logs network status via defmt:

- **DHCP success**: `"DHCP successful - IP address: x.x.x.x"`
- **DHCP failure**: `"DHCP failed after 5000ms timeout"`
- **Network disabled**: `"Network functionality disabled - check network connection"`

View these logs using:
```bash
probe-rs run --chip RP2040 target/thumbv6m-none-eabi/debug/g5500-hamlib-adaptor
```

Or via serial connection after flashing with UF2.

## Network Services

Once the device has an IP address (via DHCP or fallback), it provides:

- **HamLib TCP Server**: Port 4533
- **Concurrent Connections**: Up to 4 simultaneous clients
- **Socket Timeout**: 60 seconds of inactivity

## Troubleshooting

### Device not getting DHCP address

**Symptoms:**
- Logs show "DHCP failed after 5000ms timeout"
- Logs show "Network functionality disabled"
- Cannot connect to device via network

**Possible causes:**
1. No DHCP server on the network
2. Network cable not connected
3. W5500 not properly initialized
4. DHCP server not responding fast enough

**Solutions:**
1. Check network cable connection
2. Verify DHCP server is running on your network
3. Increase `DHCP_TIMEOUT_MS` if your DHCP server is slow
4. Configure a static IP (see below)

### Cannot connect to device

**If DHCP should be working:**
1. Check your router's DHCP leases for hostname `g5500-hamlib-adaptor`
2. Look for the MAC address starting with `02:00:00:00:00:00`
3. Check device logs to confirm DHCP succeeded
4. Verify network cable is connected

**If DHCP is failing:**
1. Configure a static IP (see below)
2. Check physical network connection
3. Verify W5500 module is properly connected

### Using Static IP Instead of DHCP

If you want to use a static IP instead of DHCP, modify the network configuration in `main()`:

```rust
// Replace this line:
let net_config = embassy_net::Config::dhcpv4(dhcp_config);

// With static config:
use embassy_net::{Ipv4Address, Ipv4Cidr, StaticConfigV4};
let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 100), 24),
    gateway: Some(Ipv4Address::new(192, 168, 1, 1)),
    dns_servers: heapless::Vec::new(),
});
```

Then remove or comment out the DHCP wait code:
```rust
// Comment out or remove:
// info!("Waiting for DHCP...");
// let dhcp_result = wait_for_dhcp_config(stack).await;
```

## Device Operation Without Network

If DHCP fails, the device continues to operate with the following behavior:

- **System LED**: Continues blinking (heartbeat)
- **Sockets LED**: Remains off (no network connections possible)
- **ADC readings**: Continue to be sampled
- **Control outputs**: Continue to function
- **TCP server**: Not started (no network connections accepted)

The device will continue to monitor azimuth and elevation, but remote control via HamLib will not be available until network connectivity is restored (requires reboot).

## LED Indicators

- **System LED (GPIO 25 - Onboard)**: 
  - Slow blink (1 second) = Waiting for network / DHCP failed
  - Fast blink (0.5 seconds) = Network connected successfully
- **Sockets LED (GPIO 15 - External)**: 
  - ON when one or more clients are connected
  - OFF when no clients connected

**See:** `LED_INDICATORS.md` for detailed LED behavior and troubleshooting

## Security Notes

- The device accepts connections from any IP address
- No authentication is required for HamLib protocol
- Consider using firewall rules to restrict access if needed
- The device is intended for use on trusted local networks only
