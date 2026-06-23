//! Packet/byte decoding: pcap iteration, L3/L4 parsing, HTTP/TLS peek, F5
//! trailer.

use std::net::{Ipv4Addr, Ipv6Addr};

use indexmap::IndexMap;

use super::model::{Flow, FlowKey};
use crate::f5_trailer::{
    DPT_HDR_LEN, DPT_HDR_MAGIC, DPT_PROVIDER_NOISE, DPT_TLV_HDR_LEN, TrailerFmt, parse_trailer,
};
use crate::pcap_remap::{find_ip_offset, ipv6_l4_locator};
use crate::pcapng::{self, BLOCK_TYPE_EPB, BLOCK_TYPE_IDB, BLOCK_TYPE_SHB};

/// libpcap global-header magics → big-endian flag (we don't need the ns flag).
fn pcap_magic_big_endian(magic: u32) -> Option<bool> {
    match magic {
        0xA1B2_C3D4 | 0xA1B2_3C4D => Some(false), // little-endian on-disk
        0xD4C3_B2A1 | 0x4D3C_B2A1 => Some(true),  // big-endian on-disk
        _ => None,
    }
}

/// Yield `(linktype, packet_bytes)` for every packet in *data*.
fn iter_pcap_packets(data: &[u8]) -> Result<Vec<(u16, Vec<u8>)>, String> {
    if data.len() < 4 {
        return Ok(Vec::new());
    }
    if pcapng::is_pcapng_magic(&data[..4]) {
        iter_pcapng(data)
    } else {
        Ok(iter_libpcap(data))
    }
}

fn iter_libpcap(data: &[u8]) -> Vec<(u16, Vec<u8>)> {
    let mut out: Vec<(u16, Vec<u8>)> = Vec::new();
    if data.len() < 24 {
        return out;
    }
    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let Some(big_endian) = pcap_magic_big_endian(magic) else {
        // The CLI surfaces an unrecognised magic at the call site as
        // `error: pcap: unrecognised magic ...`; here a bad magic simply
        // yields no packets.
        return out;
    };
    let rd_u32 = |b: &[u8], off: usize| -> u32 {
        let arr = [b[off], b[off + 1], b[off + 2], b[off + 3]];
        if big_endian {
            u32::from_be_bytes(arr)
        } else {
            u32::from_le_bytes(arr)
        }
    };
    let linktype = (rd_u32(data, 20) & 0xFFFF) as u16;
    let mut cursor = 24usize;
    while cursor + 16 <= data.len() {
        let incl_len = rd_u32(data, cursor + 8) as usize;
        let body_start = cursor + 16;
        if data.len() - body_start < incl_len {
            return out;
        }
        out.push((linktype, data[body_start..body_start + incl_len].to_vec()));
        cursor = body_start + incl_len;
    }
    out
}

fn iter_pcapng(data: &[u8]) -> Result<Vec<(u16, Vec<u8>)>, String> {
    let blocks = pcapng::read_blocks(data)?;
    let mut out: Vec<(u16, Vec<u8>)> = Vec::new();
    let mut interface_linktypes: Vec<u16> = Vec::new();
    for block in &blocks {
        if block.block_type == BLOCK_TYPE_SHB {
            interface_linktypes = Vec::new();
        } else if block.block_type == BLOCK_TYPE_IDB {
            interface_linktypes.push(block.linktype.unwrap_or(0));
        } else if block.block_type == BLOCK_TYPE_EPB
            && let Some(packet_data) = &block.packet_data
        {
            let iface_idx = block.interface_id.unwrap_or(0) as usize;
            if iface_idx < interface_linktypes.len() {
                out.push((interface_linktypes[iface_idx], packet_data.clone()));
            }
        }
    }
    Ok(out)
}

/// Return `(proto, src_port, dst_port, tcp_flags)` or zeros if undecodable.
fn l4_ports_and_flags(packet: &[u8], ip_off: usize, is_v6: bool) -> (u8, u16, u16, u8) {
    let (proto, l4_off) = if is_v6 {
        match ipv6_l4_locator(packet, ip_off) {
            Some((p, off, _)) => (p, off),
            None => return (0, 0, 0, 0),
        }
    } else {
        if ip_off + 20 > packet.len() {
            return (0, 0, 0, 0);
        }
        let ihl = ((packet[ip_off] & 0x0F) as usize) * 4;
        if ihl < 20 {
            return (0, 0, 0, 0);
        }
        (packet[ip_off + 9], ip_off + ihl)
    };
    if (proto == 6 || proto == 17) && l4_off + 4 <= packet.len() {
        let src_port = (u16::from(packet[l4_off]) << 8) | u16::from(packet[l4_off + 1]);
        let dst_port = (u16::from(packet[l4_off + 2]) << 8) | u16::from(packet[l4_off + 3]);
        let mut flags = 0u8;
        if proto == 6 && l4_off + 14 <= packet.len() {
            flags = packet[l4_off + 13];
        }
        return (proto, src_port, dst_port, flags);
    }
    (proto, 0, 0, 0)
}

fn ip_addrs(packet: &[u8], ip_off: usize, is_v6: bool) -> (String, String) {
    if is_v6 {
        if ip_off + 40 > packet.len() {
            return (String::new(), String::new());
        }
        let mut s = [0u8; 16];
        let mut d = [0u8; 16];
        s.copy_from_slice(&packet[ip_off + 8..ip_off + 24]);
        d.copy_from_slice(&packet[ip_off + 24..ip_off + 40]);
        return (Ipv6Addr::from(s).to_string(), Ipv6Addr::from(d).to_string());
    }
    if ip_off + 20 > packet.len() {
        return (String::new(), String::new());
    }
    let src = Ipv4Addr::new(
        packet[ip_off + 12],
        packet[ip_off + 13],
        packet[ip_off + 14],
        packet[ip_off + 15],
    );
    let dst = Ipv4Addr::new(
        packet[ip_off + 16],
        packet[ip_off + 17],
        packet[ip_off + 18],
        packet[ip_off + 19],
    );
    (src.to_string(), dst.to_string())
}

/// Return the L4 payload bytes (after the TCP/UDP header).
fn l4_payload(packet: &[u8], ip_off: usize, is_v6: bool, proto: u8) -> Vec<u8> {
    let (l4_off, l4_len) = if is_v6 {
        match ipv6_l4_locator(packet, ip_off) {
            Some((_, off, len)) => (off, len),
            None => return Vec::new(),
        }
    } else {
        if ip_off + 20 > packet.len() {
            return Vec::new();
        }
        let ihl = ((packet[ip_off] & 0x0F) as usize) * 4;
        let total = (usize::from(packet[ip_off + 2]) << 8) | usize::from(packet[ip_off + 3]);
        (ip_off + ihl, total.wrapping_sub(ihl))
    };
    let (start, end) = if proto == 6 {
        if l4_off + 13 > packet.len() {
            return Vec::new();
        }
        let data_off = (((packet[l4_off + 12] >> 4) & 0xF) as usize) * 4;
        (l4_off + data_off, l4_off + l4_len)
    } else if proto == 17 {
        (l4_off + 8, l4_off + l4_len)
    } else {
        return Vec::new();
    };
    let end = end.min(packet.len());
    if start >= end {
        return Vec::new();
    }
    packet[start..end].to_vec()
}

const HTTP_METHODS: &[&[u8]] = &[
    b"GET ",
    b"POST ",
    b"PUT ",
    b"DELETE ",
    b"HEAD ",
    b"OPTIONS ",
    b"PATCH ",
    b"CONNECT ",
];
const HTTP_RESPONSE_PREFIX: &[u8] = b"HTTP/";

/// Decode CRLF-separated HTTP headers into a lowercase-keyed map, first-seen
/// position preserved, last value winning on a duplicate name.
fn split_http_headers(raw: &[u8]) -> IndexMap<String, String> {
    let mut out: IndexMap<String, String> = IndexMap::new();
    for line in raw.split(|&b| b == b'\n') {
        // The reference splits on CRLF; we split on \n and trim a trailing \r
        // so a bare-\n payload behaves the same as the CRLF form for our
        // purposes.
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() || !line.contains(&b':') {
            continue;
        }
        let idx = line.iter().position(|&b| b == b':').unwrap();
        let name = &line[..idx];
        let value = &line[idx + 1..];
        let key = decode_ascii_replace(name).trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        let val = decode_utf8_replace(value).trim().to_owned();
        if let Some(slot) = out.get_mut(&key) {
            *slot = val;
        } else {
            out.insert(key, val);
        }
    }
    out
}

/// Decoded HTTP request/response fields from a payload prefix.
#[derive(Default)]
pub struct HttpPeek {
    /// `true` for a status line, `false` for a request line.
    pub is_response: bool,
    /// Request method.
    pub method: String,
    /// Full request URI.
    pub uri: String,
    /// URI minus the query string.
    pub path: String,
    /// Query-string portion of the URI.
    pub query: String,
    /// `Host` header value.
    pub host: String,
    /// HTTP version token.
    pub version: String,
    /// Response status code.
    pub status: String,
    /// Response reason phrase.
    pub phrase: String,
    /// Lowercase-keyed headers in first-seen order.
    pub headers: IndexMap<String, String>,
}

/// Return decoded HTTP fields, or `None` if the payload isn't an HTTP request
/// line / status line.
fn peek_http(payload: &[u8]) -> Option<HttpPeek> {
    if payload.is_empty() {
        return None;
    }
    if payload.starts_with(HTTP_RESPONSE_PREFIX) {
        let head = split_once(payload, b"\r\n\r\n").map_or(payload, |(h, _)| h);
        let (first_line, rest) = split_once(head, b"\r\n").unwrap_or((head, b""));
        let parts: Vec<&[u8]> = splitn_space(first_line, 3);
        let ver = decode_ascii_replace(parts.first().copied().unwrap_or(b""));
        let status = parts
            .get(1)
            .map_or_else(String::new, |p| decode_ascii_replace(p));
        let phrase = parts
            .get(2)
            .map_or_else(String::new, |p| decode_ascii_replace(p));
        return Some(HttpPeek {
            is_response: true,
            version: ver,
            status,
            phrase,
            headers: split_http_headers(rest),
            ..Default::default()
        });
    }
    for method in HTTP_METHODS {
        if payload.starts_with(method) {
            let head = split_once(payload, b"\r\n\r\n").map_or(payload, |(h, _)| h);
            let (first_line, rest) = split_once(head, b"\r\n").unwrap_or((head, b""));
            let parts: Vec<&[u8]> = splitn_space(first_line, 3);
            let m = decode_ascii_replace(parts.first().copied().unwrap_or(b""));
            let u = parts
                .get(1)
                .map_or_else(String::new, |p| decode_ascii_replace(p));
            let ver = parts
                .get(2)
                .map_or_else(String::new, |p| decode_ascii_replace(p));
            let (path, query) = match u.split_once('?') {
                Some((p, q)) => (p.to_owned(), q.to_owned()),
                None => (u.clone(), String::new()),
            };
            let headers = split_http_headers(rest);
            let host = headers.get("host").cloned().unwrap_or_default();
            return Some(HttpPeek {
                is_response: false,
                method: m,
                uri: u,
                path,
                query,
                version: ver,
                host,
                headers,
                ..Default::default()
            });
        }
    }
    None
}

/// Return `(is_clienthello, version_text, sni)` from a TLS `ClientHello`.
fn peek_tls_clienthello(payload: &[u8]) -> (bool, String, String) {
    if payload.len() < 11 || payload[0] != 22 || payload[5] != 1 {
        return (false, String::new(), String::new());
    }
    let ver_major = payload[9];
    let ver_minor = payload[10];
    let version_text = match (ver_major, ver_minor) {
        (3, 1) => "TLS1.0".to_owned(),
        (3, 2) => "TLS1.1".to_owned(),
        (3, 3) => "TLS1.2".to_owned(),
        (3, 4) => "TLS1.3".to_owned(),
        _ => format!("0x{ver_major:02x}{ver_minor:02x}"),
    };
    // Skip random (32 bytes) -> offset 11 + 32 = 43.
    let mut cur = 43usize;
    if cur >= payload.len() {
        return (true, version_text, String::new());
    }
    let sid_len = payload[cur] as usize;
    cur += 1 + sid_len;
    if cur + 2 > payload.len() {
        return (true, version_text, String::new());
    }
    let cipher_suites_len = (usize::from(payload[cur]) << 8) | usize::from(payload[cur + 1]);
    cur += 2 + cipher_suites_len;
    if cur >= payload.len() {
        return (true, version_text, String::new());
    }
    let compression_len = payload[cur] as usize;
    cur += 1 + compression_len;
    if cur + 2 > payload.len() {
        return (true, version_text, String::new());
    }
    let ext_total = (usize::from(payload[cur]) << 8) | usize::from(payload[cur + 1]);
    cur += 2;
    let ext_end = (cur + ext_total).min(payload.len());
    while cur + 4 <= ext_end {
        let ext_type = (usize::from(payload[cur]) << 8) | usize::from(payload[cur + 1]);
        let ext_len = (usize::from(payload[cur + 2]) << 8) | usize::from(payload[cur + 3]);
        cur += 4;
        if cur + ext_len > ext_end {
            break;
        }
        if ext_type == 0 {
            // server_name: list_len(2) + name_type(1) + name_len(2) + name.
            if ext_len >= 5 {
                let name_type = payload[cur + 2];
                let name_len = (usize::from(payload[cur + 3]) << 8) | usize::from(payload[cur + 4]);
                if name_type == 0 && cur + 5 + name_len <= cur + ext_len {
                    let sni = decode_ascii_replace(&payload[cur + 5..cur + 5 + name_len]);
                    return (true, version_text, sni);
                }
            }
        }
        cur += ext_len;
    }
    (true, version_text, String::new())
}

/// `looks_ipv4_mapped`: a 16-byte address with the `::ffff:` prefix.
fn looks_ipv4_mapped(sixteen: &[u8]) -> bool {
    sixteen.len() == 16 && sixteen[..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff]
}

fn ip_of(sixteen: &[u8]) -> String {
    if sixteen.len() != 16 {
        return String::new();
    }
    if looks_ipv4_mapped(sixteen) {
        Ipv4Addr::new(sixteen[12], sixteen[13], sixteen[14], sixteen[15]).to_string()
    } else {
        let mut arr = [0u8; 16];
        arr.copy_from_slice(sixteen);
        Ipv6Addr::from(arr).to_string()
    }
}

const LEGACY_TYPE_HIGH: u8 = 3;
const LEGACY_TYPE_LOW: u16 = 1;
const LEGACY_TYPE_MED: u16 = 2;
/// Minimum DPT NOISE/HIGH/v1 TLV length carrying a full peer 5-tuple.
const NOISE_HIGH_V1_MIN_LEN: usize = 47;

/// Return the peer-side `(remote_ip, remote_port, local_ip, local_port)` from a
/// HIGH TLV in the F5 ethernet trailer, or `None`.
fn extract_peer_tuple_from_trailer(t: &[u8]) -> Option<(String, u16, String, u16)> {
    let n = t.len();
    if n < 3 {
        return None;
    }

    // DPT format: 4-byte magic, 4-byte envelope, then DPT TLVs.
    if n >= DPT_HDR_LEN && u32::from_be_bytes([t[0], t[1], t[2], t[3]]) == DPT_HDR_MAGIC {
        let total_len = ((u16::from(t[4]) << 8) | u16::from(t[5])) as usize;
        let end = n.min(total_len);
        let mut pos = DPT_HDR_LEN;
        while pos + DPT_TLV_HDR_LEN <= end {
            let provider = (u16::from(t[pos]) << 8) | u16::from(t[pos + 1]);
            let type_ = (u16::from(t[pos + 2]) << 8) | u16::from(t[pos + 3]);
            let length = ((u16::from(t[pos + 4]) << 8) | u16::from(t[pos + 5])) as usize;
            let version = (u16::from(t[pos + 6]) << 8) | u16::from(t[pos + 7]);
            if length < DPT_TLV_HDR_LEN || pos + length > end {
                break;
            }
            if provider == DPT_PROVIDER_NOISE
                && type_ == u16::from(LEGACY_TYPE_HIGH)
                && version == 1
                && length >= NOISE_HIGH_V1_MIN_LEN
            {
                let remote_addr = &t[pos + 11..pos + 27];
                let local_addr = &t[pos + 27..pos + 43];
                let remote_port = (u16::from(t[pos + 43]) << 8) | u16::from(t[pos + 44]);
                let local_port = (u16::from(t[pos + 45]) << 8) | u16::from(t[pos + 46]);
                let remote_ip = ip_of(remote_addr);
                let local_ip = ip_of(local_addr);
                if !remote_ip.is_empty() && !local_ip.is_empty() {
                    return Some((remote_ip, remote_port, local_ip, local_port));
                }
            }
            pos += length;
        }
        return None;
    }

    // Legacy format: walk the chain looking for a HIGH v0 entry (length 42).
    let mut pos = 0usize;
    while pos + 3 <= n {
        let type_ = t[pos];
        let wire_length = t[pos + 1] as usize;
        let total_length = wire_length + 2;
        if total_length < 5 || pos + total_length > n {
            break;
        }
        if type_ == LEGACY_TYPE_HIGH && total_length == 42 {
            let remote_addr = &t[pos + 6..pos + 22];
            let local_addr = &t[pos + 22..pos + 38];
            let remote_port = (u16::from(t[pos + 38]) << 8) | u16::from(t[pos + 39]);
            let local_port = (u16::from(t[pos + 40]) << 8) | u16::from(t[pos + 41]);
            let remote_ip = ip_of(remote_addr);
            let local_ip = ip_of(local_addr);
            if !remote_ip.is_empty() && !local_ip.is_empty() {
                return Some((remote_ip, remote_port, local_ip, local_port));
            }
        }
        pos += total_length;
    }
    None
}

/// Pull printable ASCII RST-cause strings out of LOW/MED trailer TLVs.
fn extract_rst_cause_from_trailer(trailer: &[u8]) -> Vec<String> {
    let parsed = parse_trailer(trailer);
    let Some(fmt) = parsed.fmt else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for tlv in &parsed.tlvs {
        if fmt == TrailerFmt::Legacy && tlv.type_ != LEGACY_TYPE_LOW && tlv.type_ != LEGACY_TYPE_MED
        {
            continue;
        }
        let data = &trailer[tlv.offset..(tlv.offset + tlv.length).min(trailer.len())];
        for run in ascii_runs(data, 7) {
            let text = decode_ascii_replace(run);
            let text = text.trim();
            let up = text.to_uppercase();
            if (up.contains("RST") || up.contains("RESET") || up.contains("CAUSE"))
                && !out.iter().any(|e| e == text)
            {
                out.push(text.to_owned());
            }
        }
    }
    out
}

/// Walk *pcap* bytes and accumulate one [`Flow`] per unique 5-tuple.
///
/// Returns an `IndexMap` keyed as `(src_ip, src_port, dst_ip, dst_port,
/// proto)`, preserving first-seen insertion order.
#[allow(clippy::too_many_lines)]
pub fn extract_flows(data: &[u8]) -> Result<IndexMap<FlowKey, Flow>, String> {
    let mut flows: IndexMap<FlowKey, Flow> = IndexMap::new();
    for (linktype, packet) in iter_pcap_packets(data)? {
        let Some((ip_off, is_v6)) = find_ip_offset(&packet, linktype) else {
            continue;
        };
        let (src_ip, dst_ip) = ip_addrs(&packet, ip_off, is_v6);
        if src_ip.is_empty() {
            continue;
        }
        let (proto, sp, dp, tcp_flags) = l4_ports_and_flags(&packet, ip_off, is_v6);
        if proto == 0 {
            continue;
        }
        let key: FlowKey = (src_ip.clone(), sp, dst_ip.clone(), dp, proto);
        let flow = flows
            .entry(key)
            .or_insert_with(|| Flow::new(src_ip, sp, dst_ip, dp, proto));
        flow.packets += 1;
        flow.bytes_total += packet.len() as u64;

        // Compute trailer offset once; reused for peer-tuple and reset-cause.
        let trailer_off = if is_v6 {
            let payload_len =
                (usize::from(packet[ip_off + 4]) << 8) | usize::from(packet[ip_off + 5]);
            ip_off + 40 + payload_len
        } else {
            let total_len =
                (usize::from(packet[ip_off + 2]) << 8) | usize::from(packet[ip_off + 3]);
            ip_off + total_len
        };
        let trailer: &[u8] = if trailer_off > 0 && trailer_off < packet.len() {
            &packet[trailer_off..]
        } else {
            &[]
        };
        if !trailer.is_empty()
            && flow.peer_remote_ip.is_empty()
            && let Some((rr, rp, lr, lp)) = extract_peer_tuple_from_trailer(trailer)
        {
            flow.peer_remote_ip = rr;
            flow.peer_remote_port = rp;
            flow.peer_local_ip = lr;
            flow.peer_local_port = lp;
        }

        if proto == 6 {
            if tcp_flags & 0x02 != 0 {
                if tcp_flags & 0x10 != 0 {
                    flow.tcp_synack = true;
                } else {
                    flow.tcp_syn = true;
                }
            }
            if tcp_flags & 0x01 != 0 {
                flow.tcp_fin = true;
            }
            if tcp_flags & 0x04 != 0 {
                if !flow.tcp_rst {
                    flow.tcp_rst_after_bytes = flow.bytes_total;
                }
                flow.tcp_rst = true;
                flow.tcp_rst_count += 1;
                if !trailer.is_empty() {
                    for c in extract_rst_cause_from_trailer(trailer) {
                        if !flow.f5_reset_causes.contains(&c) {
                            flow.f5_reset_causes.push(c);
                        }
                    }
                }
            }
        }

        let payload = l4_payload(&packet, ip_off, is_v6, proto);
        if !payload.is_empty() {
            let (is_ch, ver, sni) = peek_tls_clienthello(&payload);
            if is_ch {
                flow.tls_clienthello = true;
                if !ver.is_empty() && flow.tls_version.is_empty() {
                    flow.tls_version = ver;
                }
                if !sni.is_empty() && flow.tls_sni.is_empty() {
                    flow.tls_sni = sni;
                }
            }
            if let Some(http) = peek_http(&payload) {
                if http.is_response {
                    flow.http_response_seen = true;
                    set_if_empty(&mut flow.http_response_code, http.status);
                    set_if_empty(&mut flow.http_response_phrase, http.phrase);
                    if flow.http_response_headers.is_empty() {
                        flow.http_response_headers = imap_to_vec(&http.headers);
                    }
                    set_if_empty(
                        &mut flow.http_response_content_type,
                        http.headers
                            .get("content-type")
                            .cloned()
                            .unwrap_or_default(),
                    );
                    set_if_empty(
                        &mut flow.http_response_content_length,
                        http.headers
                            .get("content-length")
                            .cloned()
                            .unwrap_or_default(),
                    );
                } else if !http.method.is_empty() {
                    flow.http_request_seen = true;
                    set_if_empty(&mut flow.http_method, http.method);
                    set_if_empty(&mut flow.http_uri, http.uri);
                    set_if_empty(&mut flow.http_path, http.path);
                    set_if_empty(&mut flow.http_query, http.query);
                    set_if_empty(&mut flow.http_host, http.host);
                    set_if_empty(&mut flow.http_request_version, http.version);
                    if flow.http_request_headers.is_empty() {
                        flow.http_request_headers = imap_to_vec(&http.headers);
                    }
                    set_if_empty(
                        &mut flow.http_user_agent,
                        http.headers.get("user-agent").cloned().unwrap_or_default(),
                    );
                    set_if_empty(
                        &mut flow.http_cookie,
                        http.headers.get("cookie").cloned().unwrap_or_default(),
                    );
                    set_if_empty(
                        &mut flow.http_referer,
                        http.headers.get("referer").cloned().unwrap_or_default(),
                    );
                    set_if_empty(
                        &mut flow.http_request_content_type,
                        http.headers
                            .get("content-type")
                            .cloned()
                            .unwrap_or_default(),
                    );
                    set_if_empty(
                        &mut flow.http_request_content_length,
                        http.headers
                            .get("content-length")
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
            }
        }
    }
    Ok(flows)
}

fn set_if_empty(slot: &mut String, value: String) {
    if slot.is_empty() {
        *slot = value;
    }
}

fn imap_to_vec(m: &IndexMap<String, String>) -> Vec<(String, String)> {
    m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

// Byte-string helpers.

fn split_once<'a>(hay: &'a [u8], sep: &[u8]) -> Option<(&'a [u8], &'a [u8])> {
    hay.windows(sep.len())
        .position(|w| w == sep)
        .map(|i| (&hay[..i], &hay[i + sep.len()..]))
}

/// Split on ASCII space into at most `n` parts.
fn splitn_space(hay: &[u8], n: usize) -> Vec<&[u8]> {
    let mut parts: Vec<&[u8]> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < hay.len() {
        if parts.len() + 1 >= n {
            break;
        }
        if hay[i] == b' ' {
            parts.push(&hay[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    parts.push(&hay[start..]);
    parts
}

/// Decode as ASCII, non-ASCII bytes become U+FFFD.
fn decode_ascii_replace(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| if b < 0x80 { b as char } else { '\u{FFFD}' })
        .collect()
}

/// Decode as UTF-8 with lossy replacement.
fn decode_utf8_replace(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Find ASCII runs (`[\x20-\x7e]`) of at least `min_len` bytes.
fn ascii_runs(data: &[u8], min_len: usize) -> Vec<&[u8]> {
    let mut runs: Vec<&[u8]> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &b) in data.iter().enumerate() {
        if (0x20..=0x7e).contains(&b) {
            start.get_or_insert(i);
        } else if let Some(s) = start.take()
            && i - s >= min_len
        {
            runs.push(&data[s..i]);
        }
    }
    if let Some(s) = start
        && data.len() - s >= min_len
    {
        runs.push(&data[s..]);
    }
    runs
}
