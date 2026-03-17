# Code Patterns & Conventions

## Architecture Overview

### Task-Based Concurrency
The firmware uses Embassy's task-based async model with multiple concurrent tasks:

```rust
#[embassy_executor::task]
async fn task_name(param: Type) {
    // Task implementation
}

// Spawn task
spawner.spawn(task_name(param)).unwrap();
```

**Key Tasks:**
- `ethernet_task`: W5500 driver runner
- `net_task`: Network stack runner
- `led_blink_task`: System LED status indicator
- `control_task`: Rotator control (Az/El relay outputs)
- `adc_task`: Position monitoring (Az/El ADC inputs)
- `listen_task`: TCP socket handler (pool of 4)

### Shared State Management
Uses `Mutex<ThreadModeRawMutex, RefCell<T>>` for thread-safe shared state:

```rust
static CURRENT_AZ_EL_DEGREES: Mutex<ThreadModeRawMutex, RefCell<(f32, f32)>> = 
    Mutex::new(RefCell::new((0.0, 0.0)));

// Access pattern
CURRENT_AZ_EL_DEGREES.lock(|f| {
    let (az, el) = *f.borrow();
    // Use az, el
});

// Update pattern
CURRENT_AZ_EL_DEGREES.lock(|f| {
    f.replace((new_az, new_el));
});
```

### Resource Assignment
Uses `assign_resources!` macro for compile-time resource allocation:

```rust
assign_resources! {
    azel_adc: AzElAdc {
        adc: ADC,
        dma: DMA_CH2,
        pin_az: PIN_26,
        pin_el: PIN_27
    },
    azel_control: AzElControl {
        pin_az_cw: PIN_2,
        pin_az_ccw: PIN_3,
        pin_el_up: PIN_4,
        pin_el_dn: PIN_5
    }
}

let r = split_resources!(p);
spawner.spawn(adc_task(spawner, r.azel_adc)).unwrap();
```

## Protocol Parsing Pattern

Uses `nom` parser combinators for HamLib protocol:

```rust
#[derive(PartialEq, Eq)]
enum HamlibCommand {
    GetInfo,
    GetPos,
    Stop,
    Park,
    SetPos,
    Quit,
    DumpState,
    Reset,
    _None
}

impl Command {
    #[inline]
    fn parse_get_info(input: &[u8]) -> IResult<&[u8], &[u8]> {
        alt((tag("_"), tag("\\get_info"))).parse(input)
    }

    #[inline]
    fn parse(input: &[u8]) -> (HamlibCommand, f32, f32) {
        if Self::parse_get_info(input).is_ok() {
            return (HamlibCommand::GetInfo, 0.0, 0.0);
        }
        // ... other parsers
        (HamlibCommand::_None, 0.0, 0.0)
    }
}
```

## Logging Conventions

Uses `defmt` for structured logging:

```rust
info!("Message with value: {}", value);
warn!("Warning message");
error!("Error message");
debug!("Debug message");
```

**Log Levels:**
- `error!`: Critical failures
- `warn!`: Recoverable issues
- `info!`: Important state changes
- `debug!`: Detailed diagnostics

## Timing & Delays

Uses Embassy timers:

```rust
// One-shot delay
embassy_time::Timer::after(Duration::from_millis(100)).await;

// Periodic ticker
let mut ticker = Ticker::every(Duration::from_millis(250));
loop {
    // Do work
    ticker.next().await;
}

// Check elapsed time
let start = Instant::now();
if start.elapsed().as_millis() > timeout_ms {
    // Timeout occurred
}
```

## GPIO Patterns

### Output Control
```rust
let mut pin = Output::new(p.PIN_X, Level::Low);
pin.set_high();
pin.set_low();
pin.toggle();
pin.set_level(Level::High);
```

### Input Reading
```rust
let pin = Input::new(p.PIN_X, Pull::Up);
if pin.is_high() {
    // Pin is high
}
```

## ADC Sampling Pattern

Multi-channel ADC with DNL spike filtering:

```rust
const DNL_SPIKES: [u16; 4] = [512, 1536, 2560, 3584];

for i in 0..NUM_SAMPLES {
    let az_val = buf[2*i];
    if !DNL_SPIKES.contains(&az_val) {
        az_sum += az_val as u32;
        az_count += 1;
    }
}

let candidate_az_raw = az_sum as f32 / az_count as f32;
let candidate_az_degrees = ((candidate_az_raw - ADC_RAW_AZ_LOW) / 
    ((ADC_RAW_AZ_HIGH - ADC_RAW_AZ_LOW) / 450.0)).clamp(0.0, 450.0);
```

## Network Socket Pattern

TCP socket handling with timeout:

```rust
#[embassy_executor::task(pool_size = 4)]
async fn listen_task(stack: Stack<'static>, port: u16) {
    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];
    let mut buf = [0; 256];

    loop {
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(SOCKET_TIMEOUT_S)));

        if let Err(e) = socket.accept(port).await {
            warn!("TCP accept error: {:?}", e);
            continue;
        }

        // Handle connection
        loop {
            match socket.read(&mut buf).await {
                Ok(0) => break,  // EOF
                Ok(n) => {
                    // Process n bytes
                    let _ = socket.write(response).await;
                }
                Err(e) => {
                    warn!("TCP error: {:?}", e);
                    break;
                }
            }
        }
    }
}
```

## Error Handling

Prefer `unwrap!()` macro for critical paths (embedded context):

```rust
unwrap!(spawner.spawn(task_name(param)));
```

For recoverable errors, use pattern matching:

```rust
match result {
    Ok(value) => { /* handle success */ }
    Err(e) => {
        warn!("Error: {:?}", e);
        // Continue or retry
    }
}
```

## Constants & Configuration

Top-level constants for configuration:

```rust
const WATCHDOG_PERIOD_MS: u64 = 8300;
const DHCP_TIMEOUT_MS: u64 = 5000;
const NUMBER_HAMLIB_SOCKETS: u16 = 4;
const SOCKET_TIMEOUT_S: u64 = 60;
const PARK_AZ_DEGREES: f32 = 180.0;
const PARK_EL_DEGREES: f32 = 0.0;
const CONTROL_DEGREES_THRESHOLD: f32 = 3.0;
```

## Naming Conventions

- **Tasks**: `snake_case` with `_task` suffix
- **Constants**: `UPPER_SNAKE_CASE`
- **Functions**: `snake_case`
- **Types/Enums**: `PascalCase`
- **Variables**: `snake_case`
- **Static state**: `UPPER_SNAKE_CASE`

## Optimization Patterns

### Inline Hints
```rust
#[inline]
fn parse_get_info(input: &[u8]) -> IResult<&[u8], &[u8]> {
    // Hot path function
}
```

### Saturating Arithmetic
```rust
socket_count.saturating_sub(1)  // Prevents underflow
```

### Efficient Clamping
```rust
value.clamp(min, max)  // Single operation
```

### Simplified Boolean Logic
```rust
if local_demand_run { }  // Instead of: if local_demand_run == true { }
```

## Memory Considerations

- **Stack**: Limited (264KB total RAM)
- **Buffers**: Sized for actual use (1KB TCP buffers, 256B command buffer)
- **Static allocation**: Preferred over heap (no_std)
- **DMA**: Used for ADC and SPI transfers
- **Watchdog**: 8.3s timeout (includes DHCP wait)

## Testing Patterns

Use `\dump_state` command for diagnostics:

```
Product Name: G-5500 HamLib Adaptor - Phil Crump M0DNY
Firmware Version: <git-version>
Flash UUID: <8 bytes>
Uptime: <seconds>
Connected Clients: <current>/<max>
Current Azimuth: <degrees> [raw: <adc>/4096]
Current Elevation: <degrees> [raw: <adc>/4096]
Demand: RUN: <bool>, Az: <degrees> El: <degrees>
```

## Build-Time Patterns

### Git Version Embedding
```rust
const GIT_VERSION: &str = git_version!(args = ["--dirty", "--always"], fallback = "nogit");
```

### Memory Layout Configuration
```rust
// build.rs
println!("cargo:rustc-link-arg-bins=--nmagic");
println!("cargo:rustc-link-arg-bins=-Tlink.x");
println!("cargo:rustc-link-arg-bins=-Tlink-rp.x");
println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
```
