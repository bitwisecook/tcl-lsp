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

//! Port-set and IP-range builtins.

use super::address_builtins::ip_to_u128;
use super::cidr_builtins::{count_value, format_ip_int, network_at, summarize_range};
use super::parse_ip_address;
use crate::builtins::{as_int, as_str};
use crate::errors::QueryError;
use crate::value::Value;
use std::net::IpAddr;

// port sets

#[derive(Clone, Copy)]
struct PortSeg {
    low: u32,
    high: u32,
}

/// Parse a port set into normalised segments, or `None`.
fn port_set_try_parse(text: &str) -> Option<Vec<PortSeg>> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_lowercase();
    if lower == "any" || lower == "*" || lower == "0" {
        return Some(vec![PortSeg {
            low: 0,
            high: 65535,
        }]);
    }
    let mut raw: Vec<PortSeg> = Vec::new();
    for chunk in text.split(',') {
        let piece = chunk.trim();
        if piece.is_empty() {
            continue;
        }
        if let Some((lo_text, hi_text)) = piece.split_once('-') {
            let lo: i64 = lo_text.trim().parse().ok()?;
            let hi: i64 = hi_text.trim().parse().ok()?;
            // PortSegment validation: 0 <= low, high <= 65535, low <= high.
            if lo < 0 || hi > 65535 || lo > hi {
                return None;
            }
            raw.push(PortSeg {
                low: lo as u32,
                high: hi as u32,
            });
        } else {
            let n: i64 = piece.parse().ok()?;
            if !(0..=65535).contains(&n) {
                return None;
            }
            raw.push(PortSeg {
                low: n as u32,
                high: n as u32,
            });
        }
    }
    if raw.is_empty() {
        return None;
    }
    Some(collapse_port_segments(raw))
}

/// Collapse adjacent / overlapping port segments.
fn collapse_port_segments(mut raw: Vec<PortSeg>) -> Vec<PortSeg> {
    raw.sort_by_key(|s| (s.low, s.high));
    let mut out: Vec<PortSeg> = vec![raw[0]];
    for seg in &raw[1..] {
        let last = out.last_mut().unwrap();
        if seg.low <= last.high + 1 {
            last.high = last.high.max(seg.high);
        } else {
            out.push(*seg);
        }
    }
    out
}

pub(super) fn bi_port_set_contains(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "port_set_contains", 1)?;
    let p = as_int(&args[1], "port_set_contains", 2)?;
    let Some(segs) = port_set_try_parse(&s) else {
        return Ok(Value::Bool(false));
    };
    let result = segs
        .iter()
        .any(|seg| i64::from(seg.low) <= p && p <= i64::from(seg.high));
    Ok(Value::Bool(result))
}

pub(super) fn bi_port_set_count(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "port_set_count", 1)?;
    match port_set_try_parse(&s) {
        None => Ok(Value::Null),
        Some(segs) => {
            let count: i64 = segs
                .iter()
                .map(|seg| i64::from(seg.high) - i64::from(seg.low) + 1)
                .sum();
            Ok(Value::Int(count))
        }
    }
}

pub(super) fn bi_port_set_overlaps(args: &[Value]) -> Result<Value, QueryError> {
    let sa = as_str(&args[0], "port_set_overlaps", 1)?;
    let sb = as_str(&args[1], "port_set_overlaps", 2)?;
    let (Some(a), Some(b)) = (port_set_try_parse(&sa), port_set_try_parse(&sb)) else {
        return Ok(Value::Bool(false));
    };
    let result = a
        .iter()
        .any(|s| b.iter().any(|o| !(s.high < o.low || o.high < s.low)));
    Ok(Value::Bool(result))
}

// IP ranges

/// Parse an IP range into `(first, last)` as same-family `IpAddr`, or `None`.
fn ip_range_try_parse(text: &str) -> Option<(IpAddr, IpAddr)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if !text.contains('-') {
        let single = parse_ip_address(text)?;
        return Some((single, single));
    }
    let (lo_text, hi_text) = text.rsplit_once('-')?;
    let lo = parse_ip_address(lo_text.trim())?;
    let hi = parse_ip_address(hi_text.trim())?;
    // __post_init__: same family, first <= last.
    match (lo, hi) {
        (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {}
        _ => return None,
    }
    if ip_to_u128(lo) > ip_to_u128(hi) {
        return None;
    }
    Some((lo, hi))
}

pub(super) fn bi_ip_range_to_cidrs(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_to_cidrs", 1)?;
    let Some((first, last)) = ip_range_try_parse(&s) else {
        return Ok(Value::Null);
    };
    let is_v4 = first.is_ipv4();
    let max_prefix: u8 = if is_v4 { 32 } else { 128 };
    let mut cidrs = Vec::new();
    summarize_range(ip_to_u128(first), ip_to_u128(last), max_prefix, &mut cidrs);
    let out: Vec<Value> = cidrs
        .into_iter()
        .map(|(addr, prefix)| Value::Str(format!("{}/{}", format_ip_int(addr, is_v4), prefix)))
        .collect();
    Ok(Value::List(out))
}

pub(super) fn bi_ip_range_supernet(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_supernet", 1)?;
    let Some((first, last)) = ip_range_try_parse(&s) else {
        return Ok(Value::Null);
    };
    let is_v4 = first.is_ipv4();
    let max_prefix: u8 = if is_v4 { 32 } else { 128 };
    let lo = ip_to_u128(first);
    let hi = ip_to_u128(last);
    let mut cidrs = Vec::new();
    summarize_range(lo, hi, max_prefix, &mut cidrs);
    if cidrs.len() == 1 {
        let (addr, prefix) = cidrs[0];
        return Ok(Value::Str(format!(
            "{}/{}",
            format_ip_int(addr, is_v4),
            prefix
        )));
    }
    // Iteratively widen.
    for prefix in (0..=max_prefix).rev() {
        let (net, bcast) = network_at(lo, prefix, max_prefix);
        if net <= lo && bcast >= hi {
            return Ok(Value::Str(format!(
                "{}/{}",
                format_ip_int(net, is_v4),
                prefix
            )));
        }
    }
    Ok(Value::Str(if is_v4 {
        "0.0.0.0/0".to_string()
    } else {
        "::/0".to_string()
    }))
}

pub(super) fn bi_ip_range_count(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_count", 1)?;
    match ip_range_try_parse(&s) {
        None => Ok(Value::Null),
        Some((first, last)) => {
            let count = ip_to_u128(last) - ip_to_u128(first) + 1;
            Ok(count_value(count))
        }
    }
}

pub(super) fn bi_ip_range_contains(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ip_range_contains", 1)?;
    let a = as_str(&args[1], "ip_range_contains", 2)?;
    let Some((first, last)) = ip_range_try_parse(&s) else {
        return Ok(Value::Bool(false));
    };
    let Some(candidate) = parse_ip_address(&a) else {
        return Ok(Value::Bool(false));
    };
    if candidate.is_ipv4() != first.is_ipv4() {
        return Ok(Value::Bool(false));
    }
    let c = ip_to_u128(candidate);
    Ok(Value::Bool(ip_to_u128(first) <= c && c <= ip_to_u128(last)))
}
