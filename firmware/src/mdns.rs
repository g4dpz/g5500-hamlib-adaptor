/// Minimal mDNS responder for device discovery
///
/// Responds to A record queries for `<HOSTNAME>.local` and SRV/PTR queries
/// for `_rotctld._tcp.local` service discovery.
///
/// mDNS spec: RFC 6762, DNS-SD spec: RFC 6763

use embassy_net::udp::{UdpSocket, PacketMetadata};
use embassy_net::{Stack, Ipv4Address, IpAddress};
use defmt::*;

const MDNS_PORT: u16 = 5353;
const MDNS_MULTICAST_IPV4: Ipv4Address = Ipv4Address::new(224, 0, 0, 251);

// DNS constants
const DNS_TYPE_A: u16 = 1;
const DNS_TYPE_PTR: u16 = 12;
const DNS_TYPE_SRV: u16 = 33;
const DNS_TYPE_TXT: u16 = 16;
const DNS_CLASS_IN: u16 = 1;
const DNS_CLASS_IN_FLUSH: u16 = 0x8001; // Class IN with cache-flush bit

const TTL_SECS: u32 = 120;

/// Run the mDNS responder task
///
/// Joins the mDNS multicast group, listens for queries matching our hostname
/// or service type, and responds with appropriate records.
pub async fn mdns_responder(stack: Stack<'static>, hostname: &str, service_port: u16) -> ! {
    // Join mDNS multicast group
    if let Err(e) = stack.join_multicast_group(IpAddress::Ipv4(MDNS_MULTICAST_IPV4)) {
        error!("mDNS: failed to join multicast group: {:?}", e);
    } else {
        info!("mDNS: joined multicast group 224.0.0.251");
    }

    let mut rx_meta = [PacketMetadata::EMPTY; 2];
    let mut rx_buf = [0u8; 512];
    let mut tx_meta = [PacketMetadata::EMPTY; 2];
    let mut tx_buf = [0u8; 512];

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    if let Err(e) = socket.bind(MDNS_PORT) {
        error!("mDNS: failed to bind UDP port {}: {:?}", MDNS_PORT, e);
        // Can't recover, just loop forever
        loop {
            embassy_time::Timer::after(embassy_time::Duration::from_secs(60)).await;
        }
    }
    info!("mDNS: listening on port {}", MDNS_PORT);

    let mut recv_buf = [0u8; 512];
    let mut resp_buf = [0u8; 512];

    loop {
        let (n, _remote_ep) = match socket.recv_from(&mut recv_buf).await {
            Ok(r) => r,
            Err(e) => {
                warn!("mDNS: recv error: {:?}", e);
                continue;
            }
        };

        if n < 12 {
            continue; // Too short for DNS header
        }

        let data = &recv_buf[..n];

        // Parse DNS header
        let flags = u16::from_be_bytes([data[2], data[3]]);
        let qr = (flags >> 15) & 1;
        if qr != 0 {
            continue; // Not a query, skip
        }

        let qdcount = u16::from_be_bytes([data[4], data[5]]);
        if qdcount == 0 {
            continue;
        }

        // Get our IP from the stack
        let our_ip = match stack.config_v4() {
            Some(cfg) => cfg.address.address(),
            None => continue, // No IP yet
        };

        // Parse questions
        let mut offset = 12usize;
        for _ in 0..qdcount {
            if offset >= n {
                break;
            }

            // Parse the query name
            let mut name_buf = [0u8; 128];
            let mut name_len = 0usize;
            let name_start = offset;

            // Read labels
            loop {
                if offset >= n {
                    break;
                }
                let label_len = data[offset] as usize;
                if label_len == 0 {
                    offset += 1;
                    break;
                }
                if (label_len & 0xC0) == 0xC0 {
                    // Compression pointer — skip 2 bytes
                    offset += 2;
                    break;
                }
                offset += 1;
                if offset + label_len > n {
                    break;
                }
                if name_len > 0 && name_len < name_buf.len() {
                    name_buf[name_len] = b'.';
                    name_len += 1;
                }
                let end = (name_len + label_len).min(name_buf.len());
                let copy_len = end - name_len;
                // Copy and lowercase
                for i in 0..copy_len {
                    let b = data[offset + i];
                    name_buf[name_len + i] = if b >= b'A' && b <= b'Z' { b + 32 } else { b };
                }
                name_len += copy_len;
                offset += label_len;
            }

            if offset + 4 > n {
                break;
            }

            let qtype = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let _qclass = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            offset += 4;

            let query_name = &name_buf[..name_len];

            // Check if this is a query for our hostname.local (A record)
            let mut host_local = [0u8; 64];
            let host_local_len = build_host_local(hostname, &mut host_local);

            if qtype == DNS_TYPE_A && names_equal(query_name, &host_local[..host_local_len]) {
                debug!("mDNS: A query for our hostname");
                let resp_len = build_a_response(
                    &data[..12], // Copy transaction ID from query
                    &data[name_start..], // Original question section for name reference
                    hostname,
                    our_ip,
                    &mut resp_buf,
                );
                if resp_len > 0 {
                    let _ = socket.send_to(
                        &resp_buf[..resp_len],
                        (MDNS_MULTICAST_IPV4, MDNS_PORT),
                    ).await;
                }
            }

            // Check for _rotctld._tcp.local PTR query (service discovery)
            if qtype == DNS_TYPE_PTR && names_equal(query_name, b"_rotctld._tcp.local") {
                debug!("mDNS: PTR query for _rotctld._tcp.local");
                let resp_len = build_service_response(
                    &data[..12],
                    hostname,
                    our_ip,
                    service_port,
                    &mut resp_buf,
                );
                if resp_len > 0 {
                    let _ = socket.send_to(
                        &resp_buf[..resp_len],
                        (MDNS_MULTICAST_IPV4, MDNS_PORT),
                    ).await;
                }
            }
        }
    }
}

/// Build "hostname.local" lowercase into buf, return length
fn build_host_local(hostname: &str, buf: &mut [u8]) -> usize {
    let mut i = 0;
    for b in hostname.as_bytes() {
        if i >= buf.len() { break; }
        buf[i] = if *b >= b'A' && *b <= b'Z' { *b + 32 } else { *b };
        i += 1;
    }
    if i + 6 <= buf.len() {
        buf[i..i + 6].copy_from_slice(b".local");
        i += 6;
    }
    i
}

/// Case-insensitive name comparison
fn names_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        let ca = if a[i] >= b'A' && a[i] <= b'Z' { a[i] + 32 } else { a[i] };
        let cb = if b[i] >= b'A' && b[i] <= b'Z' { b[i] + 32 } else { b[i] };
        if ca != cb {
            return false;
        }
    }
    true
}

/// Write a DNS name as labels into buf at offset, return new offset
fn write_dns_name(buf: &mut [u8], mut offset: usize, name: &str) -> usize {
    for label in name.split('.') {
        let len = label.len();
        if offset + 1 + len > buf.len() {
            return offset;
        }
        buf[offset] = len as u8;
        offset += 1;
        buf[offset..offset + len].copy_from_slice(label.as_bytes());
        offset += len;
    }
    if offset < buf.len() {
        buf[offset] = 0; // Root label
        offset += 1;
    }
    offset
}

/// Build an A record response for hostname.local
fn build_a_response(
    query_header: &[u8],
    _question_raw: &[u8],
    hostname: &str,
    ip: Ipv4Address,
    buf: &mut [u8],
) -> usize {
    let mut off = 0usize;

    // Transaction ID from query
    buf[off..off + 2].copy_from_slice(&query_header[0..2]);
    off += 2;

    // Flags: response, authoritative
    buf[off..off + 2].copy_from_slice(&0x8400u16.to_be_bytes());
    off += 2;

    // QDCOUNT=0, ANCOUNT=1, NSCOUNT=0, ARCOUNT=0
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&1u16.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2;

    // Answer: hostname.local A record
    let mut host_local_name = [0u8; 64];
    let hl_len = build_host_local_dns(hostname, &mut host_local_name);
    off = write_dns_name_raw(buf, off, &host_local_name[..hl_len]);

    // Type A
    buf[off..off + 2].copy_from_slice(&DNS_TYPE_A.to_be_bytes()); off += 2;
    // Class IN with cache-flush
    buf[off..off + 2].copy_from_slice(&DNS_CLASS_IN_FLUSH.to_be_bytes()); off += 2;
    // TTL
    buf[off..off + 4].copy_from_slice(&TTL_SECS.to_be_bytes()); off += 4;
    // RDLENGTH = 4 (IPv4)
    buf[off..off + 2].copy_from_slice(&4u16.to_be_bytes()); off += 2;
    // RDATA = IP address
    let octets = ip.octets();
    buf[off..off + 4].copy_from_slice(&octets); off += 4;

    off
}

/// Build a full service discovery response (PTR + SRV + TXT + A)
fn build_service_response(
    query_header: &[u8],
    hostname: &str,
    ip: Ipv4Address,
    port: u16,
    buf: &mut [u8],
) -> usize {
    let mut off = 0usize;

    // Transaction ID
    buf[off..off + 2].copy_from_slice(&query_header[0..2]);
    off += 2;

    // Flags: response, authoritative
    buf[off..off + 2].copy_from_slice(&0x8400u16.to_be_bytes());
    off += 2;

    // QDCOUNT=0, ANCOUNT=1, NSCOUNT=0, ARCOUNT=3 (SRV + TXT + A)
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2; // QD
    buf[off..off + 2].copy_from_slice(&1u16.to_be_bytes()); off += 2; // AN (PTR)
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2; // NS
    buf[off..off + 2].copy_from_slice(&3u16.to_be_bytes()); off += 2; // AR (SRV+TXT+A)

    // --- Answer: PTR record ---
    // Name: _rotctld._tcp.local
    off = write_dns_name(buf, off, "_rotctld._tcp.local");
    buf[off..off + 2].copy_from_slice(&DNS_TYPE_PTR.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&DNS_CLASS_IN.to_be_bytes()); off += 2;
    buf[off..off + 4].copy_from_slice(&TTL_SECS.to_be_bytes()); off += 4;

    // PTR RDATA: instance name = "G5500 HamLib._rotctld._tcp.local"
    // Build the instance name in DNS label format first to get RDLENGTH
    let instance_label = "G5500 HamLib";
    let rdata_start = off + 2; // Skip RDLENGTH for now
    off += 2;
    let rdata_begin = off;
    // Instance label
    if off < buf.len() { buf[off] = instance_label.len() as u8; off += 1; }
    buf[off..off + instance_label.len()].copy_from_slice(instance_label.as_bytes());
    off += instance_label.len();
    // _rotctld._tcp.local
    off = write_dns_name(buf, off, "_rotctld._tcp.local");
    let rdlength = (off - rdata_begin) as u16;
    buf[rdata_start..rdata_start + 2].copy_from_slice(&rdlength.to_be_bytes());

    // --- Additional: SRV record ---
    // Name: G5500 HamLib._rotctld._tcp.local
    let _srv_name_start = off;
    if off < buf.len() { buf[off] = instance_label.len() as u8; off += 1; }
    buf[off..off + instance_label.len()].copy_from_slice(instance_label.as_bytes());
    off += instance_label.len();
    off = write_dns_name(buf, off, "_rotctld._tcp.local");

    buf[off..off + 2].copy_from_slice(&DNS_TYPE_SRV.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&DNS_CLASS_IN_FLUSH.to_be_bytes()); off += 2;
    buf[off..off + 4].copy_from_slice(&TTL_SECS.to_be_bytes()); off += 4;

    // SRV RDATA: priority(2) + weight(2) + port(2) + target name
    let srv_rdata_start = off + 2;
    off += 2;
    let srv_rdata_begin = off;
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2; // Priority
    buf[off..off + 2].copy_from_slice(&0u16.to_be_bytes()); off += 2; // Weight
    buf[off..off + 2].copy_from_slice(&port.to_be_bytes()); off += 2; // Port
    // Target: hostname.local
    let mut hl = [0u8; 64];
    let hl_len = build_host_local_dns(hostname, &mut hl);
    off = write_dns_name_raw(buf, off, &hl[..hl_len]);
    let srv_rdlength = (off - srv_rdata_begin) as u16;
    buf[srv_rdata_start..srv_rdata_start + 2].copy_from_slice(&srv_rdlength.to_be_bytes());

    // --- Additional: TXT record (empty, required by DNS-SD) ---
    if off < buf.len() { buf[off] = instance_label.len() as u8; off += 1; }
    buf[off..off + instance_label.len()].copy_from_slice(instance_label.as_bytes());
    off += instance_label.len();
    off = write_dns_name(buf, off, "_rotctld._tcp.local");

    buf[off..off + 2].copy_from_slice(&DNS_TYPE_TXT.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&DNS_CLASS_IN_FLUSH.to_be_bytes()); off += 2;
    buf[off..off + 4].copy_from_slice(&TTL_SECS.to_be_bytes()); off += 4;
    // TXT RDATA: single empty string (length byte = 0)
    buf[off..off + 2].copy_from_slice(&1u16.to_be_bytes()); off += 2; // RDLENGTH=1
    buf[off] = 0; off += 1; // Empty TXT

    // --- Additional: A record ---
    let mut hl2 = [0u8; 64];
    let hl2_len = build_host_local_dns(hostname, &mut hl2);
    off = write_dns_name_raw(buf, off, &hl2[..hl2_len]);

    buf[off..off + 2].copy_from_slice(&DNS_TYPE_A.to_be_bytes()); off += 2;
    buf[off..off + 2].copy_from_slice(&DNS_CLASS_IN_FLUSH.to_be_bytes()); off += 2;
    buf[off..off + 4].copy_from_slice(&TTL_SECS.to_be_bytes()); off += 4;
    buf[off..off + 2].copy_from_slice(&4u16.to_be_bytes()); off += 2;
    let octets = ip.octets();
    buf[off..off + 4].copy_from_slice(&octets); off += 4;

    off
}

/// Build hostname.local as DNS wire-format labels (e.g. \x13g5500-hamlib-adaptor\x05local\x00)
fn build_host_local_dns(hostname: &str, buf: &mut [u8]) -> usize {
    let mut off = 0;
    let hb = hostname.as_bytes();
    if off + 1 + hb.len() > buf.len() { return 0; }
    buf[off] = hb.len() as u8;
    off += 1;
    buf[off..off + hb.len()].copy_from_slice(hb);
    off += hb.len();
    if off + 6 > buf.len() { return off; }
    buf[off] = 5; off += 1; // "local" length
    buf[off..off + 5].copy_from_slice(b"local");
    off += 5;
    buf[off] = 0; off += 1; // Root
    off
}

/// Write pre-built DNS wire-format name labels into buf
fn write_dns_name_raw(buf: &mut [u8], offset: usize, name_labels: &[u8]) -> usize {
    let len = name_labels.len();
    if offset + len > buf.len() { return offset; }
    buf[offset..offset + len].copy_from_slice(name_labels);
    offset + len
}
