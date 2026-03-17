/// Minimal HTTP server for web-based configuration
///
/// Serves on port 80:
///   GET  /        — Status page with current position, config, and settings form
///   POST /config  — Apply new configuration settings and save to flash
///
/// No external HTTP crate — raw request parsing on TCP socket.

use embassy_net::Stack;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Instant};
use embedded_io_async::Write;
use defmt::*;

use crate::{
    PRODUCT_NAME, GIT_VERSION, NUMBER_HAMLIB_SOCKETS,
    CURRENT_AZ_EL_DEGREES, CURRENT_AZ_EL_RAW,
    DEMAND_RUN_AZ_EL_DEGREES, SOCKETS_CONNECTED,
    FLASH_UUID, STORED_CONFIG,
    config,
};

const HTTP_PORT: u16 = 80;

/// Run the HTTP server — accepts one connection at a time on port 80
pub async fn http_server(stack: Stack<'static>) -> ! {
    let mut rx_buf = [0u8; 1024];
    let mut tx_buf = [0u8; 2048];
    let mut req_buf = [0u8; 512];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
        socket.set_timeout(Some(Duration::from_secs(15)));

        if let Err(e) = socket.accept(HTTP_PORT).await {
            warn!("HTTP accept error: {:?}", e);
            continue;
        }

        // Read request
        let n = match socket.read(&mut req_buf).await {
            Ok(0) | Err(_) => {
                socket.close();
                continue;
            }
            Ok(n) => n,
        };

        let req = &req_buf[..n];

        if starts_with(req, b"GET / ") || starts_with(req, b"GET / HTTP") {
            let _ = serve_status_page(&mut socket).await;
        } else if starts_with(req, b"POST /config") {
            let _ = handle_config_post(&mut socket, req).await;
        } else if starts_with(req, b"POST /cal/capture-low") {
            let _ = handle_cal_capture_low(&mut socket, req).await;
        } else if starts_with(req, b"POST /cal/capture-high") {
            let _ = handle_cal_capture_high(&mut socket, req).await;
        } else if starts_with(req, b"POST /cal/clear") {
            let _ = handle_cal_clear(&mut socket, req).await;
        } else {
            let _ = socket.write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n").await;
        }

        socket.close();
    }
}

fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && &haystack[..needle.len()] == needle
}

async fn serve_status_page(socket: &mut TcpSocket<'_>) -> Result<(), embassy_net::tcp::Error> {
    // Gather current state
    let (az_deg, el_deg) = CURRENT_AZ_EL_DEGREES.lock(|f| *f.borrow());
    let (az_raw, el_raw) = CURRENT_AZ_EL_RAW.lock(|f| *f.borrow());
    let (run, demand_az, demand_el) = DEMAND_RUN_AZ_EL_DEGREES.lock(|f| *f.borrow());
    let sockets = SOCKETS_CONNECTED.lock(|f| *f.borrow());
    let uuid = FLASH_UUID.lock(|f| *f.borrow());
    let uptime = Instant::now().as_secs();

    let cfg = STORED_CONFIG.lock(|f| {
        f.borrow().clone().unwrap_or_else(config::Config::default)
    });

    // Build HTML in chunks to stay within buffer limits
    // Send header first
    socket.write_all(b"HTTP/1.0 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n").await?;

    // HTML head
    socket.write_all(b"<!DOCTYPE html><html><head><title>G-5500 Hamlib Adaptor</title>\
        <meta name=viewport content='width=device-width,initial-scale=1'>\
        <style>body{font-family:monospace;max-width:600px;margin:0 auto;padding:1em;background:#1a1a2e;color:#e0e0e0}\
        h1{color:#0f0;font-size:1.2em}h2{color:#0a0;font-size:1em;margin-top:1.5em}\
        table{border-collapse:collapse;width:100%}td{padding:4px 8px;border-bottom:1px solid #333}\
        td:first-child{color:#888}input{background:#222;color:#e0e0e0;border:1px solid #444;padding:4px;width:80px}\
        button{background:#0a0;color:#000;border:none;padding:8px 16px;cursor:pointer;margin-top:8px}\
        .warn{color:#fa0}</style></head><body>").await?;

    // Title and system info
    let mut buf = [0u8; 512];
    let len = format_to_buf(&mut buf, format_args!(
        "<h1>{}</h1><p>Firmware: {}</p>\
        <p>UUID: {:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x} | Uptime: {}s</p>",
        PRODUCT_NAME, GIT_VERSION,
        uuid[0], uuid[1], uuid[2], uuid[3], uuid[4], uuid[5], uuid[6], uuid[7],
        uptime
    ));
    socket.write_all(&buf[..len]).await?;

    // Status table
    let len = format_to_buf(&mut buf, format_args!(
        "<h2>Status</h2><table>\
        <tr><td>Azimuth</td><td>{:.1}&deg; (raw {:.0}/4096)</td></tr>\
        <tr><td>Elevation</td><td>{:.1}&deg; (raw {:.0}/4096)</td></tr>\
        <tr><td>Demand</td><td>{} Az {:.1}&deg; El {:.1}&deg;</td></tr>\
        <tr><td>Clients</td><td>{}/{}</td></tr>\
        </table>",
        az_deg, az_raw, el_deg, el_raw,
        if run { "RUN" } else { "STOP" }, demand_az, demand_el,
        sockets, NUMBER_HAMLIB_SOCKETS
    ));
    socket.write_all(&buf[..len]).await?;

    // Config form
    let ip = cfg.static_ip;
    let len = format_to_buf(&mut buf, format_args!(
        "<h2>Configuration</h2>\
        <form method=POST action=/config>\
        <table>\
        <tr><td>Park Az (&deg;)</td><td><input name=paz value='{:.1}'></td></tr>\
        <tr><td>Park El (&deg;)</td><td><input name=pel value='{:.1}'></td></tr>\
        <tr><td>Az Cal Offset</td><td><input name=azo value='{:.2}'></td></tr>\
        <tr><td>El Cal Offset</td><td><input name=elo value='{:.2}'></td></tr>\
        <tr><td>Static IP</td><td><input name=ip value='{}.{}.{}.{}'></td></tr>\
        <tr><td>Use Static IP</td><td><input type=checkbox name=sip value=1{}></td></tr>\
        </table>\
        <button type=submit>Save to Flash</button></form>",
        cfg.park_az, cfg.park_el,
        cfg.az_cal_offset, cfg.el_cal_offset,
        ip[0], ip[1], ip[2], ip[3],
        if cfg.static_ip_enabled { " checked" } else { "" }
    ));
    socket.write_all(&buf[..len]).await?;

    // --- Calibration section (Task 7.1: status + reference values) ---
    if cfg.calibration_valid {
        socket.write_all(b"<h2>Calibration <span style='color:#0f0'>[Calibrated]</span></h2>").await?;
    } else {
        socket.write_all(b"<h2>Calibration <span class=warn>[Uncalibrated]</span></h2>").await?;
    }

    // Live raw ADC values
    let len = format_to_buf(&mut buf, format_args!(
        "<table><tr><td>Live Az Raw</td><td>{:.0}</td></tr>\
        <tr><td>Live El Raw</td><td>{:.0}</td></tr></table>",
        az_raw, el_raw
    ));
    socket.write_all(&buf[..len]).await?;

    // Stored reference values (only when calibrated)
    if cfg.calibration_valid {
        let len = format_to_buf(&mut buf, format_args!(
            "<h3 style='color:#0a0;font-size:0.9em'>Stored Reference Points</h3>\
            <table>\
            <tr><td>Az Low</td><td>raw {:.0} = {:.1}&deg;</td></tr>\
            <tr><td>Az High</td><td>raw {:.0} = {:.1}&deg;</td></tr>",
            cfg.az_raw_low, cfg.az_deg_low,
            cfg.az_raw_high, cfg.az_deg_high
        ));
        socket.write_all(&buf[..len]).await?;

        let len = format_to_buf(&mut buf, format_args!(
            "<tr><td>El Low</td><td>raw {:.0} = {:.1}&deg;</td></tr>\
            <tr><td>El High</td><td>raw {:.0} = {:.1}&deg;</td></tr>\
            </table>",
            cfg.el_raw_low, cfg.el_deg_low,
            cfg.el_raw_high, cfg.el_deg_high
        ));
        socket.write_all(&buf[..len]).await?;
    }

    // --- Task 7.2: Capture Low form ---
    socket.write_all(b"<h3 style='color:#0a0;font-size:0.9em'>Capture Low</h3>\
        <form method=POST action=/cal/capture-low><table>\
        <tr><td>Az Deg Low</td><td><input name=adl value='0.0'></td></tr>\
        <tr><td>El Deg Low</td><td><input name=edl value='0.0'></td></tr>\
        </table><button type=submit>Capture Low</button></form>").await?;

    // Capture High form
    socket.write_all(b"<h3 style='color:#0a0;font-size:0.9em'>Capture High</h3>\
        <form method=POST action=/cal/capture-high><table>\
        <tr><td>Az Deg High</td><td><input name=adh value='450.0'></td></tr>\
        <tr><td>El Deg High</td><td><input name=edh value='180.0'></td></tr>\
        </table><button type=submit>Capture High</button></form>").await?;

    // Clear Calibration form
    socket.write_all(b"<form method=POST action=/cal/clear>\
        <button type=submit style='background:#a00;margin-top:12px'>Clear Calibration</button></form>").await?;

    socket.write_all(b"<p><small>Auto-refreshes on next load</small></p></body></html>").await?;
    socket.flush().await?;
    Ok(())
}

async fn handle_config_post(socket: &mut TcpSocket<'_>, req: &[u8]) -> Result<(), embassy_net::tcp::Error> {
    // Find the body after \r\n\r\n
    let body = find_body(req).unwrap_or(b"");

    // Parse form fields from URL-encoded body
    let mut new_cfg = STORED_CONFIG.lock(|f| {
        f.borrow().clone().unwrap_or_else(config::Config::default)
    });

    // Parse each field
    if let Some(v) = get_form_value(body, b"paz=") {
        if let Some(f) = parse_f32(v) {
            new_cfg.park_az = f.clamp(0.0, 450.0);
        }
    }
    if let Some(v) = get_form_value(body, b"pel=") {
        if let Some(f) = parse_f32(v) {
            new_cfg.park_el = f.clamp(0.0, 180.0);
        }
    }
    if let Some(v) = get_form_value(body, b"azo=") {
        if let Some(f) = parse_f32(v) {
            new_cfg.az_cal_offset = f.clamp(-50.0, 50.0);
        }
    }
    if let Some(v) = get_form_value(body, b"elo=") {
        if let Some(f) = parse_f32(v) {
            new_cfg.el_cal_offset = f.clamp(-50.0, 50.0);
        }
    }
    if let Some(v) = get_form_value(body, b"ip=") {
        if let Some(ip) = parse_ip(v) {
            new_cfg.static_ip = ip;
        }
    }
    // Checkbox: present = enabled, absent = disabled
    new_cfg.static_ip_enabled = get_form_value(body, b"sip=").is_some();

    // Update shared config
    STORED_CONFIG.lock(|f| {
        f.replace(Some(new_cfg.clone()));
    });

    // Signal main to save config to flash
    crate::CONFIG_SAVE_PENDING.lock(|f| f.replace(true));

    info!("Config updated via HTTP (park: {}/{})", new_cfg.park_az, new_cfg.park_el);

    // Redirect back to status page
    socket.write_all(b"HTTP/1.0 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\n\r\n").await?;
    socket.flush().await?;
    Ok(())
}

async fn handle_cal_capture_low(socket: &mut TcpSocket<'_>, req: &[u8]) -> Result<(), embassy_net::tcp::Error> {
    let body = find_body(req).unwrap_or(b"");
    let (az_raw, el_raw) = CURRENT_AZ_EL_RAW.lock(|f| *f.borrow());

    let az_deg = get_form_value(body, b"adl=").and_then(parse_f32).unwrap_or(0.0);
    let el_deg = get_form_value(body, b"edl=").and_then(parse_f32).unwrap_or(0.0);

    STORED_CONFIG.lock(|f| {
        if let Some(ref mut cfg) = *f.borrow_mut() {
            cfg.az_raw_low = az_raw;
            cfg.el_raw_low = el_raw;
            cfg.az_deg_low = az_deg;
            cfg.el_deg_low = el_deg;
        }
    });
    crate::CONFIG_SAVE_PENDING.lock(|f| f.replace(true));

    info!("Cal capture low: az_raw={}, el_raw={}, az_deg={}, el_deg={}", az_raw, el_raw, az_deg, el_deg);

    socket.write_all(b"HTTP/1.0 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\n\r\n").await?;
    socket.flush().await?;
    Ok(())
}

async fn handle_cal_capture_high(socket: &mut TcpSocket<'_>, req: &[u8]) -> Result<(), embassy_net::tcp::Error> {
    let body = find_body(req).unwrap_or(b"");
    let (az_raw, el_raw) = CURRENT_AZ_EL_RAW.lock(|f| *f.borrow());

    let az_deg = get_form_value(body, b"adh=").and_then(parse_f32).unwrap_or(450.0);
    let el_deg = get_form_value(body, b"edh=").and_then(parse_f32).unwrap_or(180.0);

    STORED_CONFIG.lock(|f| {
        if let Some(ref mut cfg) = *f.borrow_mut() {
            cfg.az_raw_high = az_raw;
            cfg.el_raw_high = el_raw;
            cfg.az_deg_high = az_deg;
            cfg.el_deg_high = el_deg;
            cfg.calibration_valid = true;
        }
    });
    crate::CONFIG_SAVE_PENDING.lock(|f| f.replace(true));

    info!("Cal capture high: az_raw={}, el_raw={}, az_deg={}, el_deg={}", az_raw, el_raw, az_deg, el_deg);

    socket.write_all(b"HTTP/1.0 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\n\r\n").await?;
    socket.flush().await?;
    Ok(())
}

async fn handle_cal_clear(socket: &mut TcpSocket<'_>, _req: &[u8]) -> Result<(), embassy_net::tcp::Error> {
    STORED_CONFIG.lock(|f| {
        if let Some(ref mut cfg) = *f.borrow_mut() {
            cfg.calibration_valid = false;
            cfg.az_raw_low = 0.0;
            cfg.az_raw_high = 0.0;
            cfg.el_raw_low = 0.0;
            cfg.el_raw_high = 0.0;
            cfg.az_deg_low = 0.0;
            cfg.az_deg_high = 0.0;
            cfg.el_deg_low = 0.0;
            cfg.el_deg_high = 0.0;
        }
    });
    crate::CONFIG_SAVE_PENDING.lock(|f| f.replace(true));

    info!("Calibration cleared");

    socket.write_all(b"HTTP/1.0 303 See Other\r\nLocation: /\r\nContent-Length: 0\r\n\r\n").await?;
    socket.flush().await?;
    Ok(())
}

fn find_body(req: &[u8]) -> Option<&[u8]> {
    for i in 0..req.len().saturating_sub(3) {
        if &req[i..i + 4] == b"\r\n\r\n" {
            return Some(&req[i + 4..]);
        }
    }
    None
}

/// Extract a form field value from URL-encoded body (e.g., "paz=180.0&pel=0.0")
fn get_form_value<'a>(body: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    // Search for key at start or after &
    let mut pos = 0;
    while pos < body.len() {
        let remaining = &body[pos..];
        if starts_with(remaining, key) {
            let val_start = pos + key.len();
            let val_end = body[val_start..]
                .iter()
                .position(|&b| b == b'&')
                .map(|p| val_start + p)
                .unwrap_or(body.len());
            return Some(&body[val_start..val_end]);
        }
        // Skip to next &
        match body[pos..].iter().position(|&b| b == b'&') {
            Some(p) => pos += p + 1,
            None => break,
        }
    }
    None
}

/// Parse a float from ASCII bytes (simple: handles digits, '.', '-')
fn parse_f32(bytes: &[u8]) -> Option<f32> {
    // URL-decode: replace %2E with '.', %2D with '-'
    let mut decoded = [0u8; 32];
    let len = url_decode(bytes, &mut decoded);
    let s = core::str::from_utf8(&decoded[..len]).ok()?;

    // Manual float parse for no_std
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let negative = s.starts_with('-');
    let s = if negative { &s[1..] } else { s };

    let mut integer_part: u32 = 0;
    let mut frac_part: u32 = 0;
    let mut frac_digits: u32 = 0;
    let mut in_frac = false;

    for &b in s.as_bytes() {
        match b {
            b'0'..=b'9' => {
                if in_frac {
                    if frac_digits < 6 {
                        frac_part = frac_part * 10 + (b - b'0') as u32;
                        frac_digits += 1;
                    }
                } else {
                    integer_part = integer_part * 10 + (b - b'0') as u32;
                }
            }
            b'.' => in_frac = true,
            _ => return None,
        }
    }

    let mut result = integer_part as f32;
    if frac_digits > 0 {
        let mut divisor = 1u32;
        for _ in 0..frac_digits {
            divisor *= 10;
        }
        result += frac_part as f32 / divisor as f32;
    }
    if negative {
        result = -result;
    }
    Some(result)
}

/// Parse an IP address from "192.168.1.100" or URL-encoded "192%2E168%2E1%2E100"
fn parse_ip(bytes: &[u8]) -> Option<[u8; 4]> {
    let mut decoded = [0u8; 32];
    let len = url_decode(bytes, &mut decoded);
    let s = core::str::from_utf8(&decoded[..len]).ok()?;

    let mut octets = [0u8; 4];
    let mut idx = 0;

    for part in s.split('.') {
        if idx >= 4 {
            return None;
        }
        let val: u16 = part.parse().ok()?;
        if val > 255 {
            return None;
        }
        octets[idx] = val as u8;
        idx += 1;
    }

    if idx == 4 { Some(octets) } else { None }
}

/// Simple URL decode: handles %XX sequences
fn url_decode(input: &[u8], output: &mut [u8]) -> usize {
    let mut i = 0;
    let mut o = 0;
    while i < input.len() && o < output.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            if let (Some(h), Some(l)) = (hex_val(input[i + 1]), hex_val(input[i + 2])) {
                output[o] = (h << 4) | l;
                i += 3;
                o += 1;
                continue;
            }
        }
        if input[i] == b'+' {
            output[o] = b' ';
        } else {
            output[o] = input[i];
        }
        i += 1;
        o += 1;
    }
    o
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Format into a fixed buffer, return the number of bytes written
fn format_to_buf(buf: &mut [u8], args: core::fmt::Arguments<'_>) -> usize {
    match format_no_std::show(buf, args) {
        Ok(s) => s.len(),
        Err(_) => buf.len(), // truncated — send what we have
    }
}
