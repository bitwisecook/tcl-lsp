// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Apply a [`RedactionMap`] to a PCAP capture file.
//!
//! Powers `f5 pcap-remap`. For each packet record it parses the link-layer
//! header, locates the IPv4 / IPv6 header, rewrites src/dst via the redaction map,
//! recomputes the IPv4 header checksum and the TCP / UDP / ICMP / ICMPv6
//! checksum (including the pseudo-header), and rewrites peer-IP fields in the
//! F5 Ethernet trailer at their schema-known offsets.
//!
//! Both classic libpcap (`0xa1b2c3d4` / `0xd4c3b2a1`, microsecond and
//! nanosecond variants) and PCAPNG are supported.

#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::doc_markdown
)]

use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::f5_trailer::{
    DPT_TLV_HDR_LEN, IpKind, SchemaOverlay, TrailerFmt, classify_v6_or_v4mapped, parse_trailer_with,
};
use crate::pcapng;
use crate::redact::{RedactionMap, apply_map, map_address, unmap_address};

// Link-layer types we know how to walk to the IP layer.
const LINKTYPE_ETHERNET: u16 = 1;
const LINKTYPE_RAW: u16 = 101;
const LINKTYPE_RAW_IPV4: u16 = 228;
const LINKTYPE_RAW_IPV6: u16 = 229;
const LINKTYPE_LINUX_SLL: u16 = 113;
const LINKTYPE_LINUX_SLL2: u16 = 276;
const LINKTYPE_F5_USER0: u16 = 147;

/// Summary counts returned by [`remap_pcap`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PcapRemapResult {
    /// Total packets seen.
    pub packets_total: usize,
    /// Packets in which at least one byte changed.
    pub packets_rewritten: usize,
    /// Total addresses rewritten (IP layer + trailer).
    pub addresses_rewritten: usize,
    /// Total trailer TLVs parsed.
    pub trailer_tlvs_total: usize,
    /// Trailer TLVs in which at least one IP was rewritten.
    pub trailer_tlvs_rewritten: usize,
    /// Trailer TLVs structurally valid but lacking a registered IP schema.
    pub trailer_tlvs_unknown: usize,
}

/// Policy for trailer TLVs whose `(type, version)` has no registered schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownPolicy {
    /// Raise [`PcapError::UnknownTrailer`] (default).
    Error,
    /// Leave that TLV unchanged; report the count.
    Preserve,
    /// Byte-replace known IPs within just that TLV's data section.
    Sweep,
}

/// Error type for [`remap_pcap`].
#[derive(Debug)]
pub enum PcapError {
    /// An unknown trailer TLV under [`UnknownPolicy::Error`].
    UnknownTrailer {
        /// Trailer framing format (`"legacy"` / `"dpt"`).
        fmt: String,
        /// TLV type.
        type_: u16,
        /// TLV version.
        version: u16,
        /// TLV length.
        length: usize,
    },
    /// Any other malformed-input / value error.
    Value(String),
}

impl std::fmt::Display for PcapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PcapError::UnknownTrailer {
                fmt,
                type_,
                version,
                length,
            } => write!(
                f,
                "f5 trailer entry has no registered schema: \
                 format={fmt} type={type_} version={version} length={length}"
            ),
            PcapError::Value(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for PcapError {}

/// Packed IPv4 -> IPv4 byte lookup for the sweep fallback.
type V4Lookup = HashMap<[u8; 4], [u8; 4]>;
/// Packed IPv6 -> IPv6 byte lookup for the sweep fallback.
type V6Lookup = HashMap<[u8; 16], [u8; 16]>;

// checksum helpers

fn u16_at(buf: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([buf[off], buf[off + 1]])
}

fn set_u16(buf: &mut [u8], off: usize, value: u16) {
    buf[off] = (value >> 8) as u8;
    buf[off + 1] = (value & 0xFF) as u8;
}

/// Standard Internet checksum (RFC 1071) over `data`.
fn ones_complement_sum(data: &[u8]) -> u16 {
    let mut total: u32 = 0;
    let mut i = 0;
    let len = data.len();
    while i + 1 < len {
        total += ((data[i] as u32) << 8) | data[i + 1] as u32;
        total = (total & 0xFFFF) + (total >> 16);
        i += 2;
    }
    if len % 2 == 1 {
        total += (data[len - 1] as u32) << 8;
        total = (total & 0xFFFF) + (total >> 16);
    }
    (!total & 0xFFFF) as u16
}

fn ipv4_header_checksum(packet: &mut [u8], ip_off: usize, ihl_bytes: usize) -> u16 {
    let saved = (packet[ip_off + 10], packet[ip_off + 11]);
    packet[ip_off + 10] = 0;
    packet[ip_off + 11] = 0;
    let cksum = ones_complement_sum(&packet[ip_off..ip_off + ihl_bytes]);
    packet[ip_off + 10] = saved.0;
    packet[ip_off + 11] = saved.1;
    cksum
}

// IPv6 extension-header next_header values we walk past to find the L4 header.
const IPV6_EXT_HEADERS: [u8; 6] = [0, 43, 60, 135, 139, 140];
const IPV6_FRAGMENT: u8 = 44;
const IPV6_ESP: u8 = 50;
const IPV6_AH: u8 = 51;

/// Walk IPv6 extension headers past `ip_off` to find the L4 header. Returns
/// `(final_next_header, l4_off, l4_len)` or `None` when the L4 isn't
/// recompute-safe.
/// Locate the L4 header inside an IPv6 packet, walking extension headers.
///
/// Returns `(proto, l4_off, l4_len)` — the upper-layer protocol number, the
/// absolute offset of the L4 header, and the remaining L4 length — or `None`
/// when the chain is fragmented / encrypted / malformed. Exposed for the
/// flow extractor so there is a single canonical walker.
#[must_use]
pub fn ipv6_l4_locator(packet: &[u8], ip_off: usize) -> Option<(u8, usize, usize)> {
    if ip_off + 40 > packet.len() {
        return None;
    }
    let ip_payload_len = u16_at(packet, ip_off + 4) as usize;
    let mut next_header = packet[ip_off + 6];
    let mut cursor = ip_off + 40;
    let end_of_ip = ip_off + 40 + ip_payload_len;
    if end_of_ip > packet.len() {
        return None;
    }
    for _ in 0..16 {
        if IPV6_EXT_HEADERS.contains(&next_header) {
            if cursor + 2 > end_of_ip {
                return None;
            }
            let ext_len_bytes = (packet[cursor + 1] as usize + 1) * 8;
            if cursor + ext_len_bytes > end_of_ip {
                return None;
            }
            let next_next = packet[cursor];
            cursor += ext_len_bytes;
            next_header = next_next;
            continue;
        }
        if next_header == IPV6_FRAGMENT {
            if cursor + 8 > end_of_ip {
                return None;
            }
            return None;
        }
        if next_header == IPV6_ESP || next_header == IPV6_AH {
            return None;
        }
        return Some((next_header, cursor, end_of_ip - cursor));
    }
    None
}

/// True if the IPv4 packet at `ip_off` is a fragment.
fn is_ipv4_fragment(packet: &[u8], ip_off: usize) -> bool {
    let flags_frag = u16_at(packet, ip_off + 6);
    let mf = (flags_frag >> 13) & 0x1;
    let frag_off = flags_frag & 0x1FFF;
    mf == 1 || frag_off != 0
}

/// IP/L4 layer descriptor for [`l4_checksum`], bundled to keep the arg count low.
struct L4Ctx {
    ip_off: usize,
    ihl_bytes: usize,
    proto: u8,
    is_v6: bool,
    v6_l4_off: Option<usize>,
    v6_l4_len: Option<usize>,
}

fn l4_checksum(packet: &mut [u8], ctx: &L4Ctx) -> Option<u16> {
    let &L4Ctx {
        ip_off,
        ihl_bytes,
        proto,
        is_v6,
        v6_l4_off,
        v6_l4_len,
    } = ctx;
    if !matches!(proto, 6 | 17 | 1 | 58) {
        return None;
    }
    if !is_v6 && ihl_bytes < 20 {
        return None;
    }
    if !is_v6 && is_ipv4_fragment(packet, ip_off) {
        return None;
    }

    let (src, dst, l4_off, payload_len): ([u8; 16], [u8; 16], usize, usize);
    if is_v6 {
        let (off, len) = (v6_l4_off?, v6_l4_len?);
        let mut s = [0u8; 16];
        let mut d = [0u8; 16];
        s.copy_from_slice(&packet[ip_off + 8..ip_off + 24]);
        d.copy_from_slice(&packet[ip_off + 24..ip_off + 40]);
        src = s;
        dst = d;
        l4_off = off;
        payload_len = len;
    } else {
        let mut s = [0u8; 16];
        let mut d = [0u8; 16];
        s[..4].copy_from_slice(&packet[ip_off + 12..ip_off + 16]);
        d[..4].copy_from_slice(&packet[ip_off + 16..ip_off + 20]);
        src = s;
        dst = d;
        l4_off = ip_off + ihl_bytes;
        let total_len = u16_at(packet, ip_off + 2) as usize;
        if total_len < ihl_bytes {
            return None; // payload_len <= 0
        }
        payload_len = total_len - ihl_bytes;
    }

    if payload_len == 0 {
        return None;
    }
    if packet.len() < l4_off || packet.len() - l4_off < payload_len {
        return None;
    }
    if payload_len == 0 {
        return None;
    }

    if proto == 1 {
        // ICMPv4 — no pseudo-header.
        if payload_len < 4 {
            return None;
        }
        let cksum_off = l4_off + 2;
        let saved = (packet[cksum_off], packet[cksum_off + 1]);
        packet[cksum_off] = 0;
        packet[cksum_off + 1] = 0;
        let cksum = ones_complement_sum(&packet[l4_off..l4_off + payload_len]);
        packet[cksum_off] = saved.0;
        packet[cksum_off + 1] = saved.1;
        return Some(cksum);
    }

    // TCP / UDP / ICMPv6 use a pseudo-header.
    let mut pseudo: Vec<u8> = Vec::with_capacity(40);

    let cksum_off = if proto == 58 {
        // ICMPv6 pseudo-header: src + dst + length(4) + zero(3) + nh(1).
        pseudo.extend_from_slice(&src);
        pseudo.extend_from_slice(&dst);
        pseudo.extend_from_slice(&(payload_len as u32).to_be_bytes());
        pseudo.extend_from_slice(&[0, 0, 0, proto]);
        l4_off + 2
    } else if is_v6 {
        pseudo.extend_from_slice(&src);
        pseudo.extend_from_slice(&dst);
        pseudo.extend_from_slice(&(payload_len as u32).to_be_bytes());
        pseudo.extend_from_slice(&[0, 0, 0, proto]);
        if proto == 6 { l4_off + 16 } else { l4_off + 6 }
    } else {
        // IPv4 TCP/UDP pseudo-header: src(4) + dst(4) + zero + proto + length.
        pseudo.extend_from_slice(&src[..4]);
        pseudo.extend_from_slice(&dst[..4]);
        pseudo.extend_from_slice(&[0, proto]);
        pseudo.extend_from_slice(&(payload_len as u16).to_be_bytes());
        if proto == 6 { l4_off + 16 } else { l4_off + 6 }
    };

    let saved = (packet[cksum_off], packet[cksum_off + 1]);
    packet[cksum_off] = 0;
    packet[cksum_off + 1] = 0;
    let mut sum_bytes = pseudo;
    sum_bytes.extend_from_slice(&packet[l4_off..l4_off + payload_len]);
    let mut cksum = ones_complement_sum(&sum_bytes);
    if proto == 17 && cksum == 0 {
        cksum = 0xFFFF; // UDP-on-IPv4 zero-cksum special case.
    }
    packet[cksum_off] = saved.0;
    packet[cksum_off + 1] = saved.1;
    Some(cksum)
}

/// Whether a full IP header fits at `off` — `rewrite_ip_layer` indexes the
/// fixed 20-byte IPv4 / 40-byte IPv6 header without further bounds checks, so a
/// candidate offset is only usable when the packet is long enough. A truncated
/// capture record (e.g. an Ethernet frame with an IPv4 ethertype but no
/// 20-byte IP header) is therefore reported as "no IP layer" and skipped rather
/// than triggering an out-of-bounds slice panic.
///
/// On such a truncated record the malformed packet is skipped (passed through
/// unmodified) rather than risking an out-of-bounds slice; well-formed captures
/// (the only shape any real device produces) carry full headers and are
/// unaffected.
fn ip_header_fits(packet: &[u8], off: usize, is_v6: bool) -> bool {
    let need = if is_v6 { 40 } else { 20 };
    off.checked_add(need).is_some_and(|end| end <= packet.len())
}

/// Return `(offset, is_v6)` for the IP header inside `packet`, or `None`.
///
/// A candidate offset is only returned when a full IP header fits at it (see
/// [`ip_header_fits`]); truncated packets yield `None`.
#[must_use]
pub fn find_ip_offset(packet: &[u8], linktype: u16) -> Option<(usize, bool)> {
    // Funnel every candidate through the fixed-header length guard so callers
    // never index past a truncated capture record.
    let fit = |off: usize, is_v6: bool| ip_header_fits(packet, off, is_v6).then_some((off, is_v6));

    if linktype == LINKTYPE_RAW || linktype == LINKTYPE_RAW_IPV4 {
        return if !packet.is_empty() && (packet[0] >> 4) == 4 {
            fit(0, false)
        } else {
            None
        };
    }
    if linktype == LINKTYPE_RAW_IPV6 {
        return if !packet.is_empty() && (packet[0] >> 4) == 6 {
            fit(0, true)
        } else {
            None
        };
    }

    if (linktype == LINKTYPE_ETHERNET || linktype == LINKTYPE_F5_USER0) && packet.len() >= 14 {
        let ethertype = u16_at(packet, 12);
        if ethertype == 0x0800 {
            return fit(14, false);
        }
        if ethertype == 0x86DD {
            return fit(14, true);
        }
        if ethertype == 0x8100 && packet.len() >= 18 {
            let inner = u16_at(packet, 16);
            if inner == 0x0800 {
                return fit(18, false);
            }
            if inner == 0x86DD {
                return fit(18, true);
            }
        }
    }

    if linktype == LINKTYPE_LINUX_SLL && packet.len() >= 16 {
        let proto = u16_at(packet, 14);
        if proto == 0x0800 {
            return fit(16, false);
        }
        if proto == 0x86DD {
            return fit(16, true);
        }
    }

    if linktype == LINKTYPE_LINUX_SLL2 && packet.len() >= 20 {
        let proto = u16_at(packet, 0);
        if proto == 0x0800 {
            return fit(20, false);
        }
        if proto == 0x86DD {
            return fit(20, true);
        }
    }

    if linktype == LINKTYPE_F5_USER0 {
        let limit = 64.min(packet.len().saturating_sub(1));
        for (off, &byte) in packet.iter().enumerate().take(limit) {
            let ver = byte >> 4;
            if (ver == 4 || ver == 6)
                && let Some(hit) = fit(off, ver == 6)
            {
                return Some(hit);
            }
        }
    }

    None
}

fn convert(rm: &mut RedactionMap, addr: &str, reverse: bool) -> String {
    if reverse {
        unmap_address(rm, addr)
    } else {
        map_address(rm, addr)
    }
}

/// Rewrite IP src/dst at `ip_off`; return number of addresses changed.
fn rewrite_ip_layer(
    packet: &mut [u8],
    ip_off: usize,
    is_v6: bool,
    rm: &mut RedactionMap,
    reverse: bool,
) -> usize {
    let mut changed = 0usize;
    let mut v6_l4_off: Option<usize> = None;
    let mut v6_l4_len: Option<usize> = None;
    let ihl_bytes;
    let proto;

    if is_v6 {
        let mut src_b = [0u8; 16];
        let mut dst_b = [0u8; 16];
        src_b.copy_from_slice(&packet[ip_off + 8..ip_off + 24]);
        dst_b.copy_from_slice(&packet[ip_off + 24..ip_off + 40]);
        let src_str = Ipv6Addr::from(src_b).to_string();
        let dst_str = Ipv6Addr::from(dst_b).to_string();
        let new_src = convert(rm, &src_str, reverse);
        let new_dst = convert(rm, &dst_str, reverse);
        if new_src != src_str
            && let Ok(a) = new_src.parse::<Ipv6Addr>()
        {
            packet[ip_off + 8..ip_off + 24].copy_from_slice(&a.octets());
            changed += 1;
        }
        if new_dst != dst_str
            && let Ok(a) = new_dst.parse::<Ipv6Addr>()
        {
            packet[ip_off + 24..ip_off + 40].copy_from_slice(&a.octets());
            changed += 1;
        }
        ihl_bytes = 40;
        if let Some((p, off, len)) = ipv6_l4_locator(packet, ip_off) {
            proto = p;
            v6_l4_off = Some(off);
            v6_l4_len = Some(len);
        } else {
            proto = packet[ip_off + 6];
        }
    } else {
        let src_b: [u8; 4] = packet[ip_off + 12..ip_off + 16].try_into().unwrap();
        let dst_b: [u8; 4] = packet[ip_off + 16..ip_off + 20].try_into().unwrap();
        let src_str = Ipv4Addr::from(src_b).to_string();
        let dst_str = Ipv4Addr::from(dst_b).to_string();
        let new_src = convert(rm, &src_str, reverse);
        let new_dst = convert(rm, &dst_str, reverse);
        if new_src != src_str
            && let Ok(a) = new_src.parse::<Ipv4Addr>()
        {
            packet[ip_off + 12..ip_off + 16].copy_from_slice(&a.octets());
            changed += 1;
        }
        if new_dst != dst_str
            && let Ok(a) = new_dst.parse::<Ipv4Addr>()
        {
            packet[ip_off + 16..ip_off + 20].copy_from_slice(&a.octets());
            changed += 1;
        }
        ihl_bytes = (packet[ip_off] & 0x0F) as usize * 4;
        proto = packet[ip_off + 9];
    }

    if changed > 0 {
        if !is_v6 {
            let new_cksum = ipv4_header_checksum(packet, ip_off, ihl_bytes);
            set_u16(packet, ip_off + 10, new_cksum);
        }
        if let Some(new_l4) = l4_checksum(
            packet,
            &L4Ctx {
                ip_off,
                ihl_bytes,
                proto,
                is_v6,
                v6_l4_off,
                v6_l4_len,
            },
        ) {
            let l4_base = if is_v6 {
                v6_l4_off.unwrap_or(ip_off + ihl_bytes)
            } else {
                ip_off + ihl_bytes
            };
            let cksum_off = match proto {
                6 => l4_base + 16,     // TCP
                17 => l4_base + 6,     // UDP
                1 | 58 => l4_base + 2, // ICMPv4 / ICMPv6
                _ => usize::MAX,
            };
            if cksum_off != usize::MAX {
                set_u16(packet, cksum_off, new_l4);
            }
        }
    }

    changed
}

/// Build the packed v4/v6 lookup tables for the sweep fallback.
fn packed_lookup_for(rm: &RedactionMap, reverse: bool) -> (V4Lookup, V6Lookup) {
    let table = if reverse { &rm.reverse } else { &rm.forward };
    let mut v4: V4Lookup = HashMap::new();
    let mut v6: V6Lookup = HashMap::new();
    for (src, dst) in table {
        if let (Ok(a), Ok(b)) = (src.parse::<Ipv4Addr>(), dst.parse::<Ipv4Addr>()) {
            v4.insert(a.octets(), b.octets());
        } else if let (Ok(a), Ok(b)) = (src.parse::<Ipv6Addr>(), dst.parse::<Ipv6Addr>()) {
            v6.insert(a.octets(), b.octets());
        }
    }
    (v4, v6)
}

/// Replace exact-match IP byte sequences in `packet[start..end]`.
fn byte_replace_known_ips_in_range(
    packet: &mut [u8],
    rm: &RedactionMap,
    reverse: bool,
    start: usize,
    end: usize,
) -> usize {
    let table = if reverse { &rm.reverse } else { &rm.forward };
    if table.is_empty() {
        return 0;
    }
    let (v4_table, v6_table) = packed_lookup_for(rm, reverse);
    let mut changed = 0usize;

    // width-4 (v4) then width-16 (v6).
    for &(width, is_v4) in &[(4usize, true), (16usize, false)] {
        if (is_v4 && v4_table.is_empty()) || (!is_v4 && v6_table.is_empty()) {
            continue;
        }
        let mut i = start;
        let last = end.saturating_sub(width);
        while i <= last {
            let mut replaced = false;
            if is_v4 {
                let chunk: [u8; 4] = packet[i..i + 4].try_into().unwrap();
                if let Some(rep) = v4_table.get(&chunk) {
                    packet[i..i + 4].copy_from_slice(rep);
                    replaced = true;
                }
            } else {
                let chunk: [u8; 16] = packet[i..i + 16].try_into().unwrap();
                if let Some(rep) = v6_table.get(&chunk) {
                    packet[i..i + 16].copy_from_slice(rep);
                    replaced = true;
                }
            }
            if replaced {
                changed += 1;
                i += width;
            } else {
                i += 1;
            }
        }
    }
    changed
}

/// Rewrite a single IP field at `abs_off`; return 1 if changed else 0.
fn rewrite_trailer_field(
    packet: &mut [u8],
    abs_off: usize,
    kind: IpKind,
    rm: &mut RedactionMap,
    reverse: bool,
) -> usize {
    match kind {
        IpKind::V4 => {
            if abs_off + 4 > packet.len() {
                return 0;
            }
            let b: [u8; 4] = packet[abs_off..abs_off + 4].try_into().unwrap();
            let addr_str = Ipv4Addr::from(b).to_string();
            let new = convert(rm, &addr_str, reverse);
            if new != addr_str
                && let Ok(a) = new.parse::<Ipv4Addr>()
            {
                packet[abs_off..abs_off + 4].copy_from_slice(&a.octets());
                return 1;
            }
            0
        }
        IpKind::V6 => {
            if abs_off + 16 > packet.len() {
                return 0;
            }
            let b: [u8; 16] = packet[abs_off..abs_off + 16].try_into().unwrap();
            let addr_str = Ipv6Addr::from(b).to_string();
            let new = convert(rm, &addr_str, reverse);
            if new != addr_str
                && let Ok(a) = new.parse::<Ipv6Addr>()
            {
                packet[abs_off..abs_off + 16].copy_from_slice(&a.octets());
                return 1;
            }
            0
        }
        IpKind::V6OrV4Mapped => {
            if abs_off + 16 > packet.len() {
                return 0;
            }
            let sixteen: [u8; 16] = packet[abs_off..abs_off + 16].try_into().unwrap();
            let (actual_kind, v4_off) = classify_v6_or_v4mapped(&sixteen);
            if actual_kind == IpKind::V4 {
                rewrite_trailer_field(packet, abs_off + v4_off, IpKind::V4, rm, reverse)
            } else {
                rewrite_trailer_field(packet, abs_off, IpKind::V6, rm, reverse)
            }
        }
    }
}

fn tlv_header_len(fmt: TrailerFmt) -> usize {
    match fmt {
        TrailerFmt::Dpt => DPT_TLV_HDR_LEN,
        TrailerFmt::Legacy => 3,
    }
}

/// Parse and rewrite the F5 trailer in `packet[trailer_off..]`. Returns
/// `(addresses_changed, tlvs_total, tlvs_rewritten, tlvs_unknown)`.
fn rewrite_trailer(
    packet: &mut [u8],
    trailer_off: usize,
    rm: &mut RedactionMap,
    reverse: bool,
    unknown_policy: UnknownPolicy,
    overlay: &SchemaOverlay,
) -> Result<(usize, usize, usize, usize), PcapError> {
    if trailer_off >= packet.len() {
        return Ok((0, 0, 0, 0));
    }
    let trailer_bytes = packet[trailer_off..].to_vec();
    let parse = parse_trailer_with(&trailer_bytes, overlay);
    let Some(fmt) = parse.fmt else {
        return Ok((0, 0, 0, 0));
    };

    let mut addrs_changed = 0usize;
    let mut tlvs_rewritten = 0usize;
    let mut unknown = 0usize;
    let tlvs_total = parse.tlvs.len();

    for tlv in &parse.tlvs {
        if !tlv.schema_known {
            unknown += 1;
            match unknown_policy {
                UnknownPolicy::Error => {
                    return Err(PcapError::UnknownTrailer {
                        fmt: match fmt {
                            TrailerFmt::Legacy => "legacy".to_owned(),
                            TrailerFmt::Dpt => "dpt".to_owned(),
                        },
                        type_: tlv.type_,
                        version: tlv.version,
                        length: tlv.length,
                    });
                }
                UnknownPolicy::Sweep => {
                    let start = trailer_off + tlv.offset + tlv_header_len(fmt);
                    let end = trailer_off + tlv.offset + tlv.length;
                    let tlv_changes =
                        byte_replace_known_ips_in_range(packet, rm, reverse, start, end);
                    if tlv_changes > 0 {
                        addrs_changed += tlv_changes;
                        tlvs_rewritten += 1;
                    }
                }
                UnknownPolicy::Preserve => {}
            }
            continue;
        }

        let mut tlv_changes = 0usize;
        for ip_field in &tlv.ip_fields {
            tlv_changes += rewrite_trailer_field(
                packet,
                trailer_off + ip_field.offset,
                ip_field.kind,
                rm,
                reverse,
            );
        }
        if tlv_changes > 0 {
            addrs_changed += tlv_changes;
            tlvs_rewritten += 1;
        }
    }

    Ok((addrs_changed, tlvs_total, tlvs_rewritten, unknown))
}

// F5 FILEINFO pseudo-packet magic.
fn fileinfo_magic() -> Vec<u8> {
    let mut v = vec![0u8; 12];
    v.extend_from_slice(&[0x05, 0xff]);
    v.extend_from_slice(b"F5-Pseudo-pkt\x00");
    v
}

fn rewrite_fileinfo_packet(packet: &mut [u8], rm: &mut RedactionMap, reverse: bool) -> usize {
    let magic = fileinfo_magic();
    if !packet.starts_with(&magic) {
        return 0;
    }
    let body_off = magic.len();
    let body_text = String::from_utf8_lossy(&packet[body_off..]).into_owned();
    let original_len = body_text.len();
    let (rewritten, count) = apply_map(rm, &body_text, reverse);
    if count == 0 {
        return 0;
    }
    let rewritten_bytes = rewritten.into_bytes();
    if rewritten_bytes.len() != original_len {
        return 0;
    }
    packet[body_off..body_off + rewritten_bytes.len()].copy_from_slice(&rewritten_bytes);
    count
}

#[derive(Default)]
struct PerPacketCounts {
    addresses_rewritten: usize,
    rewrote_anything: bool,
    tlvs_total: usize,
    tlvs_rewritten: usize,
    tlvs_unknown: usize,
}

/// Rewrite one packet in place; return counts. Shared by libpcap + pcapng.
fn rewrite_one_packet(
    packet: &mut [u8],
    linktype: u16,
    rm: &mut RedactionMap,
    reverse: bool,
    unknown_policy: UnknownPolicy,
    overlay: &SchemaOverlay,
) -> Result<PerPacketCounts, PcapError> {
    let mut counts = PerPacketCounts::default();

    if packet.starts_with(&fileinfo_magic()) {
        let n = rewrite_fileinfo_packet(packet, rm, reverse);
        counts.addresses_rewritten = n;
        counts.rewrote_anything = n != 0;
        return Ok(counts);
    }

    let Some((ip_off, is_v6)) = find_ip_offset(packet, linktype) else {
        return Ok(counts);
    };

    let ip_changed = rewrite_ip_layer(packet, ip_off, is_v6, rm, reverse);

    let trailer_off = if is_v6 {
        let payload_len = u16_at(packet, ip_off + 4) as usize;
        ip_off + 40 + payload_len
    } else {
        let total_len = u16_at(packet, ip_off + 2) as usize;
        ip_off + total_len
    };

    let mut trailer_changed = 0usize;
    if trailer_off > 0 && trailer_off < packet.len() {
        let (tc, total, rew, unk) =
            rewrite_trailer(packet, trailer_off, rm, reverse, unknown_policy, overlay)?;
        trailer_changed = tc;
        counts.tlvs_total = total;
        counts.tlvs_rewritten = rew;
        counts.tlvs_unknown = unk;
    }

    counts.addresses_rewritten = ip_changed + trailer_changed;
    counts.rewrote_anything = ip_changed > 0 || trailer_changed > 0;
    Ok(counts)
}

// top-level entry

// Classic libpcap magics -> (big_endian?).
fn pcap_magic_endian(magic: u32) -> Option<bool> {
    match magic {
        0xA1B2_C3D4 | 0xA1B2_3C4D => Some(false), // little-endian
        0xD4C3_B2A1 | 0x4D3C_B2A1 => Some(true),  // big-endian
        _ => None,
    }
}

/// Stream-rewrite a libpcap *or* PCAPNG buffer. Returns the output bytes and
/// the summary counts.
///
/// # Errors
/// Returns [`PcapError::UnknownTrailer`] under [`UnknownPolicy::Error`], or
/// [`PcapError::Value`] for malformed input.
pub fn remap_pcap(
    input: &[u8],
    rm: &mut RedactionMap,
    reverse: bool,
    unknown_policy: UnknownPolicy,
) -> Result<(Vec<u8>, PcapRemapResult), PcapError> {
    remap_pcap_with(
        input,
        rm,
        reverse,
        unknown_policy,
        &SchemaOverlay::default(),
    )
}

/// Like [`remap_pcap`] but consulting *overlay* trailer schemas (from a
/// `--schema` TOML) before the built-ins. [`remap_pcap`] is the
/// `overlay = default` case.
///
/// # Errors
/// Same as [`remap_pcap`].
pub fn remap_pcap_with(
    input: &[u8],
    rm: &mut RedactionMap,
    reverse: bool,
    unknown_policy: UnknownPolicy,
    overlay: &SchemaOverlay,
) -> Result<(Vec<u8>, PcapRemapResult), PcapError> {
    if input.len() < 4 {
        return Err(PcapError::Value("pcap: file too short".to_owned()));
    }
    if pcapng::is_pcapng_magic(&input[..4]) {
        return remap_pcapng(input, rm, reverse, unknown_policy, overlay);
    }
    remap_libpcap(input, rm, reverse, unknown_policy, overlay)
}

fn remap_libpcap(
    input: &[u8],
    rm: &mut RedactionMap,
    reverse: bool,
    unknown_policy: UnknownPolicy,
    overlay: &SchemaOverlay,
) -> Result<(Vec<u8>, PcapRemapResult), PcapError> {
    if input.len() < 24 {
        return Err(PcapError::Value(
            "pcap: file too short to contain a global header".to_owned(),
        ));
    }
    let magic = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let Some(big_endian) = pcap_magic_endian(magic) else {
        return Err(PcapError::Value(format!(
            "pcap: unrecognised magic 0x{magic:08x}"
        )));
    };
    let read_u32 = |b: &[u8], off: usize| -> u32 {
        let arr = [b[off], b[off + 1], b[off + 2], b[off + 3]];
        if big_endian {
            u32::from_be_bytes(arr)
        } else {
            u32::from_le_bytes(arr)
        }
    };

    let mut out: Vec<u8> = Vec::with_capacity(input.len());
    out.extend_from_slice(&input[..24]);
    let linktype = (read_u32(input, 20) & 0xFFFF) as u16;

    let mut result = PcapRemapResult::default();
    let mut cursor = 24usize;
    while cursor < input.len() {
        if input.len() - cursor < 16 {
            return Err(PcapError::Value("pcap: truncated record header".to_owned()));
        }
        let incl_len = read_u32(input, cursor + 8) as usize;
        let body_start = cursor + 16;
        if input.len() - body_start < incl_len {
            let have = input.len() - body_start;
            return Err(PcapError::Value(format!(
                "pcap: truncated packet body ({have}/{incl_len})"
            )));
        }
        result.packets_total += 1;
        let mut packet = input[body_start..body_start + incl_len].to_vec();
        let counts =
            rewrite_one_packet(&mut packet, linktype, rm, reverse, unknown_policy, overlay)?;
        if counts.rewrote_anything {
            result.packets_rewritten += 1;
        }
        result.addresses_rewritten += counts.addresses_rewritten;
        result.trailer_tlvs_total += counts.tlvs_total;
        result.trailer_tlvs_rewritten += counts.tlvs_rewritten;
        result.trailer_tlvs_unknown += counts.tlvs_unknown;

        out.extend_from_slice(&input[cursor..cursor + 16]);
        out.extend_from_slice(&packet);
        cursor = body_start + incl_len;
    }

    Ok((out, result))
}

fn remap_pcapng(
    input: &[u8],
    rm: &mut RedactionMap,
    reverse: bool,
    unknown_policy: UnknownPolicy,
    overlay: &SchemaOverlay,
) -> Result<(Vec<u8>, PcapRemapResult), PcapError> {
    let mut blocks = pcapng::read_blocks(input).map_err(|e| PcapError::Value(e.to_string()))?;
    let mut result = PcapRemapResult::default();
    let mut interface_linktypes: Vec<u16> = Vec::new();
    let mut out: Vec<u8> = Vec::with_capacity(input.len());

    for block in &mut blocks {
        if block.block_type == pcapng::BLOCK_TYPE_SHB {
            interface_linktypes = Vec::new();
        } else if block.block_type == pcapng::BLOCK_TYPE_IDB {
            interface_linktypes.push(block.linktype.unwrap_or(0));
        } else if block.block_type == pcapng::BLOCK_TYPE_EPB && block.packet_data.is_some() {
            result.packets_total += 1;
            let iface_idx = block.interface_id.unwrap_or(0) as usize;
            if iface_idx >= interface_linktypes.len() {
                pcapng::write_block(&mut out, block)
                    .map_err(|e| PcapError::Value(e.to_string()))?;
                continue;
            }
            let linktype = interface_linktypes[iface_idx];
            let mut packet = block.packet_data.take().unwrap();
            let counts =
                rewrite_one_packet(&mut packet, linktype, rm, reverse, unknown_policy, overlay)?;
            if counts.rewrote_anything {
                result.packets_rewritten += 1;
            }
            result.addresses_rewritten += counts.addresses_rewritten;
            result.trailer_tlvs_total += counts.tlvs_total;
            result.trailer_tlvs_rewritten += counts.tlvs_rewritten;
            result.trailer_tlvs_unknown += counts.tlvs_unknown;
            block.packet_data = Some(packet);
        }

        pcapng::write_block(&mut out, block).map_err(|e| PcapError::Value(e.to_string()))?;
    }

    Ok((out, result))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(pairs: &[(&str, &str)]) -> RedactionMap {
        let mut rm = RedactionMap::default();
        for (a, b) in pairs {
            rm.forward.insert((*a).to_owned(), (*b).to_owned());
            rm.reverse.insert((*b).to_owned(), (*a).to_owned());
        }
        rm
    }

    /// RFC 1071 checksum sanity: 16-bit words `0x0001 0xf203 0xf4f5 0xf6f7`
    /// have the well-known one's-complement sum `0x220d`.
    #[test]
    fn ones_complement_known_vector() {
        let data = [0x00u8, 0x01, 0xf2, 0x03, 0xf4, 0xf5, 0xf6, 0xf7];
        assert_eq!(ones_complement_sum(&data), 0x220d);
    }

    /// An odd-length buffer pads with a trailing zero byte (high byte of the
    /// final word).
    #[test]
    fn ones_complement_odd_length() {
        let even = [0x12u8, 0x34, 0x56, 0x00];
        let odd = [0x12u8, 0x34, 0x56];
        assert_eq!(ones_complement_sum(&odd), ones_complement_sum(&even));
    }

    /// Build a minimal Ethernet/IPv4/TCP libpcap with a recomputable checksum,
    /// remap it, and confirm the resulting IPv4 + TCP checksums are valid
    /// (a correct Internet checksum over the whole header verifies to zero).
    #[test]
    fn ipv4_tcp_checksums_valid_after_remap() {
        // src 93.184.216.34 -> 10.0.0.34 ; dst 8.8.8.8 -> 10.0.1.8
        let mut rm = map_with(&[("93.184.216.34", "10.0.0.34"), ("8.8.8.8", "10.0.1.8")]);

        // Hand-build the packet: Eth(14) + IPv4(20) + TCP(20) + payload.
        let mut tcp = vec![
            0x30, 0x39, 0x00, 0x50, // sport 12345 dport 80
            0, 0, 0, 1, 0, 0, 0, 2, // seq / ack
            0x50, 0x18, 0x72, 0x10, // offset+flags, window
            0, 0, 0, 0, // cksum placeholder, urgent
        ];
        tcp.extend_from_slice(b"hi");
        let src = [93u8, 184, 216, 34];
        let dst = [8u8, 8, 8, 8];
        // TCP checksum.
        let mut pseudo = Vec::new();
        pseudo.extend_from_slice(&src);
        pseudo.extend_from_slice(&dst);
        pseudo.extend_from_slice(&[0, 6]);
        pseudo.extend_from_slice(&(tcp.len() as u16).to_be_bytes());
        let mut s = pseudo.clone();
        s.extend_from_slice(&tcp);
        let ck = ones_complement_sum(&s);
        tcp[16] = (ck >> 8) as u8;
        tcp[17] = (ck & 0xFF) as u8;

        let total_len = (20 + tcp.len()) as u16;
        let mut ip = vec![0x45, 0x00];
        ip.extend_from_slice(&total_len.to_be_bytes());
        ip.extend_from_slice(&[0x12, 0x34, 0x40, 0x00, 64, 6, 0, 0]);
        ip.extend_from_slice(&src);
        ip.extend_from_slice(&dst);
        let ick = ones_complement_sum(&ip);
        ip[10] = (ick >> 8) as u8;
        ip[11] = (ick & 0xFF) as u8;

        let mut eth = vec![2u8, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 1, 0x08, 0x00];
        eth.extend_from_slice(&ip);
        eth.extend_from_slice(&tcp);

        // libpcap global header (LE, microsecond) + one record.
        let mut input = Vec::new();
        input.extend_from_slice(&0xa1b2_c3d4u32.to_le_bytes());
        input.extend_from_slice(&2u16.to_le_bytes());
        input.extend_from_slice(&4u16.to_le_bytes());
        input.extend_from_slice(&0i32.to_le_bytes());
        input.extend_from_slice(&0u32.to_le_bytes());
        input.extend_from_slice(&0x40000u32.to_le_bytes());
        input.extend_from_slice(&1u32.to_le_bytes()); // linktype Ethernet
        input.extend_from_slice(&1u32.to_le_bytes());
        input.extend_from_slice(&2u32.to_le_bytes());
        input.extend_from_slice(&(eth.len() as u32).to_le_bytes());
        input.extend_from_slice(&(eth.len() as u32).to_le_bytes());
        input.extend_from_slice(&eth);

        let (out, result) = remap_pcap(&input, &mut rm, false, UnknownPolicy::Error).unwrap();
        assert_eq!(result.packets_total, 1);
        assert_eq!(result.packets_rewritten, 1);
        assert_eq!(result.addresses_rewritten, 2);

        // Locate the IP header in the output record (24 global + 16 record).
        let ip_off = 24 + 16 + 14;
        let new_ip = &out[ip_off..ip_off + 20];
        assert_eq!(&new_ip[12..16], &[10, 0, 0, 34]); // src remapped
        assert_eq!(&new_ip[16..20], &[10, 0, 1, 8]); // dst remapped
        // IPv4 header checksum verifies to zero.
        assert_eq!(ones_complement_sum(new_ip), 0);

        // TCP checksum verifies to zero over pseudo + segment.
        let l4_off = ip_off + 20;
        let new_tcp = &out[l4_off..l4_off + tcp.len()];
        let mut v = Vec::new();
        v.extend_from_slice(&new_ip[12..16]);
        v.extend_from_slice(&new_ip[16..20]);
        v.extend_from_slice(&[0, 6]);
        v.extend_from_slice(&(new_tcp.len() as u16).to_be_bytes());
        v.extend_from_slice(new_tcp);
        assert_eq!(ones_complement_sum(&v), 0);
    }

    /// A forward remap followed by a reverse remap restores the original
    /// bytes exactly (round-trip identity).
    #[test]
    fn forward_reverse_round_trip() {
        let mut rm = map_with(&[("93.184.216.34", "10.0.0.34"), ("8.8.8.8", "10.0.1.8")]);
        // Reuse the builder above by recomputing a minimal capture inline.
        let mut tcp = vec![
            0x30, 0x39, 0x00, 0x50, 0, 0, 0, 1, 0, 0, 0, 2, 0x50, 0x18, 0x72, 0x10, 0, 0, 0, 0,
        ];
        tcp.extend_from_slice(b"hi");
        let src = [93u8, 184, 216, 34];
        let dst = [8u8, 8, 8, 8];
        let mut s = Vec::new();
        s.extend_from_slice(&src);
        s.extend_from_slice(&dst);
        s.extend_from_slice(&[0, 6]);
        s.extend_from_slice(&(tcp.len() as u16).to_be_bytes());
        s.extend_from_slice(&tcp);
        let ck = ones_complement_sum(&s);
        tcp[16] = (ck >> 8) as u8;
        tcp[17] = (ck & 0xFF) as u8;
        let total_len = (20 + tcp.len()) as u16;
        let mut ip = vec![0x45, 0x00];
        ip.extend_from_slice(&total_len.to_be_bytes());
        ip.extend_from_slice(&[0x12, 0x34, 0x40, 0x00, 64, 6, 0, 0]);
        ip.extend_from_slice(&src);
        ip.extend_from_slice(&dst);
        let ick = ones_complement_sum(&ip);
        ip[10] = (ick >> 8) as u8;
        ip[11] = (ick & 0xFF) as u8;
        let mut eth = vec![2u8, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 1, 0x08, 0x00];
        eth.extend_from_slice(&ip);
        eth.extend_from_slice(&tcp);
        let mut input = Vec::new();
        input.extend_from_slice(&0xa1b2_c3d4u32.to_le_bytes());
        input.extend_from_slice(&2u16.to_le_bytes());
        input.extend_from_slice(&4u16.to_le_bytes());
        input.extend_from_slice(&0i32.to_le_bytes());
        input.extend_from_slice(&0u32.to_le_bytes());
        input.extend_from_slice(&0x40000u32.to_le_bytes());
        input.extend_from_slice(&1u32.to_le_bytes());
        input.extend_from_slice(&1u32.to_le_bytes());
        input.extend_from_slice(&2u32.to_le_bytes());
        input.extend_from_slice(&(eth.len() as u32).to_le_bytes());
        input.extend_from_slice(&(eth.len() as u32).to_le_bytes());
        input.extend_from_slice(&eth);

        let (fwd, _) = remap_pcap(&input, &mut rm, false, UnknownPolicy::Error).unwrap();
        let (back, _) = remap_pcap(&fwd, &mut rm, true, UnknownPolicy::Error).unwrap();
        assert_eq!(back, input, "round-trip must restore the original");
    }

    #[test]
    fn unrecognised_magic_errors() {
        let input = [0u8; 24];
        let mut rm = RedactionMap::default();
        let err = remap_pcap(&input, &mut rm, false, UnknownPolicy::Error).unwrap_err();
        assert!(matches!(err, PcapError::Value(_)));
    }

    /// A truncated Ethernet/IPv4 record (IPv4 ethertype but fewer than 20
    /// trailing bytes) has no room for a full IP header, so `find_ip_offset`
    /// reports "no IP layer" instead of returning an offset the rewriter would
    /// index out of bounds. A full header at the same offset is accepted.
    #[test]
    fn find_ip_offset_rejects_truncated_header() {
        // Ethernet(14) + ethertype 0x0800 + only 4 IP bytes = 18 total.
        let mut trunc = vec![0u8; 12];
        trunc.extend_from_slice(&[0x08, 0x00]); // ethertype IPv4
        trunc.extend_from_slice(&[0x45, 0x00, 0x00, 0x14]); // partial IP header
        assert_eq!(find_ip_offset(&trunc, LINKTYPE_ETHERNET), None);

        // The same frame with a full 20-byte IPv4 header is accepted.
        let mut full = trunc.clone();
        full.extend_from_slice(&[0u8; 16]);
        assert_eq!(find_ip_offset(&full, LINKTYPE_ETHERNET), Some((14, false)));
    }

    /// A truncated IPv4 packet must not panic the whole-file rewriter: the
    /// malformed record is skipped (passed through unmodified) and the run
    /// completes cleanly. Regression guard for the out-of-bounds slice panic.
    #[test]
    fn remap_pcap_skips_truncated_packet() {
        let mut rm = map_with(&[("93.184.216.34", "10.0.0.34")]);
        // Ethernet(14) + ethertype 0x0800 + 4 IP bytes — no full IP header.
        let mut eth = vec![0u8; 12];
        eth.extend_from_slice(&[0x08, 0x00]);
        eth.extend_from_slice(&[0x45, 0x00, 0x00, 0x14]);
        let mut input = Vec::new();
        input.extend_from_slice(&0xa1b2_c3d4u32.to_le_bytes());
        input.extend_from_slice(&2u16.to_le_bytes());
        input.extend_from_slice(&4u16.to_le_bytes());
        input.extend_from_slice(&0i32.to_le_bytes());
        input.extend_from_slice(&0u32.to_le_bytes());
        input.extend_from_slice(&0x40000u32.to_le_bytes());
        input.extend_from_slice(&1u32.to_le_bytes()); // linktype = Ethernet
        input.extend_from_slice(&0u32.to_le_bytes());
        input.extend_from_slice(&0u32.to_le_bytes());
        input.extend_from_slice(&(eth.len() as u32).to_le_bytes());
        input.extend_from_slice(&(eth.len() as u32).to_le_bytes());
        input.extend_from_slice(&eth);

        let (out, result) = remap_pcap(&input, &mut rm, false, UnknownPolicy::Error)
            .expect("truncated packet must not panic or error");
        assert_eq!(out, input, "the malformed record passes through unmodified");
        assert_eq!(result.packets_total, 1);
        assert_eq!(result.packets_rewritten, 0);
    }
}
