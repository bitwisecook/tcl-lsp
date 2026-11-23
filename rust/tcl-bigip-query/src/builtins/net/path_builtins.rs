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

//! Builtins that edit a destination or an object path: port, host, folder, name and partition.

use tcl_bigip::object_names::BigIpName;
use tcl_bigip::value::{Address, Destination, Folder, Partition, Port, try_parse_address};

use super::{coerce_pathlike, py_str_repr};
use crate::builtins::{as_str, type_name};
use crate::errors::QueryError;
use crate::value::Value;

// destination edit

pub(super) fn bi_with_port(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_port", 1)?;
    let parsed = Destination::try_parse(&s).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_port: cannot parse destination {}",
            py_str_repr(&s)
        ))
    })?;
    let new_port = match &args[1] {
        Value::Null => Port {
            port: 0,
            spelling: String::new(),
        },
        Value::Str(p) if p.is_empty() => Port {
            port: 0,
            spelling: String::new(),
        },
        Value::Bool(_) => {
            return Err(QueryError::builtin("with_port: port cannot be a boolean"));
        }
        Value::Int(i) => {
            if !(0..=65535).contains(i) {
                return Err(QueryError::builtin(format!(
                    "with_port: port out of range ({i})"
                )));
            }
            Port {
                port: *i as u16,
                spelling: i.to_string(),
            }
        }
        Value::Str(p) => Port::try_parse(p).ok_or_else(|| {
            // Port.parse raised ValueError — surfaced as the raw message.
            port_parse_error(p)
        })?,
        Value::PathRef(p) => {
            Port::try_parse(&p.full_path).ok_or_else(|| port_parse_error(&p.full_path))?
        }
        other => {
            return Err(QueryError::builtin(format!(
                "with_port: port must be a string, integer, or null, got {}",
                type_name(other)
            )));
        }
    };
    let dest = Destination {
        address: parsed.address,
        port: new_port,
        folder: parsed.folder,
        route_domain: parsed.route_domain,
        ipv6_brackets: parsed.ipv6_brackets,
        port_separator: parsed.port_separator,
        original: parsed.original,
    };
    Ok(Value::Str(dest.to_string()))
}

/// `Port.parse` `ValueError` text, surfaced verbatim when `with_port` is given
/// an unparseable port string.
fn port_parse_error(text: &str) -> QueryError {
    let t = text.trim();
    if t.is_empty() {
        return QueryError::builtin("Port: empty input");
    }
    let lower = t.to_lowercase();
    if lower == "any" || lower == "*" || lower == "0" {
        // would have parsed
        return QueryError::builtin("Port: empty input");
    }
    match t.parse::<i64>() {
        Err(_) => QueryError::builtin(format!("Port: not a number ({})", py_str_repr(t))),
        Ok(v) => QueryError::builtin(format!("Port: out of range ({v})")),
    }
}

pub(super) fn bi_with_host(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "with_host", 1)?;
    let h = as_str(&args[1], "with_host", 2)?;
    let parsed = Destination::try_parse(&s).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_host: cannot parse destination {}",
            py_str_repr(&s)
        ))
    })?;
    let new_addr = try_parse_address(&h).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_host: cannot parse host {}: {}",
            py_str_repr(&h),
            // parse_address raises FQDN's ValueError; the typical failure is a
            // value that's neither IP nor FQDN. The FQDN error text varies;
            // reproduce the most common shape.
            fqdn_value_error(&h)
        ))
    })?;
    let new_is_v6 = matches!(&new_addr, Address::Ip(ip) if ip.addr.is_ipv6());
    let (new_brackets, new_separator) = if new_is_v6 {
        (parsed.ipv6_brackets, parsed.port_separator)
    } else {
        (false, ":".to_owned())
    };
    let dest = Destination {
        address: new_addr,
        port: parsed.port,
        folder: parsed.folder,
        route_domain: parsed.route_domain,
        ipv6_brackets: new_brackets,
        port_separator: new_separator,
        original: parsed.original,
    };
    Ok(Value::Str(dest.to_string()))
}

/// FQDN parse `ValueError` text (best-effort).
fn fqdn_value_error(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return "FQDN: empty input".to_string();
    }
    if t.starts_with('.') || t.ends_with('.') {
        return format!("FQDN: leading/trailing dot in {}", py_str_repr(t));
    }
    let labels: Vec<&str> = t.split('.').collect();
    if labels.len() < 2 {
        return format!("FQDN: must have at least one dot ({})", py_str_repr(t));
    }
    for label in &labels {
        if label.is_empty() {
            return format!("FQDN: empty label in {}", py_str_repr(t));
        }
        if label.len() > 63 {
            return format!("FQDN: label longer than 63 chars in {}", py_str_repr(t));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return format!("FQDN: label leading/trailing hyphen in {}", py_str_repr(t));
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return format!("FQDN: invalid char in label of {}", py_str_repr(t));
        }
    }
    format!("FQDN: looks like an IPv4 address ({})", py_str_repr(t))
}

// path / folder

pub(super) fn bi_folder(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "folder", 1)?;
    match BigIpName::try_parse(&s) {
        None => Ok(Value::Str(String::new())),
        Some(obj) => Ok(Value::Str(obj.folder.to_string())),
    }
}

pub(super) fn bi_with_folder(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "with_folder", 1)?;
    let f = as_str(&args[1], "with_folder", 2)?;
    let obj = BigIpName::try_parse(&p).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_folder: cannot parse path {}",
            py_str_repr(&p)
        ))
    })?;
    let new_folder = Folder::try_parse(&f).ok_or_else(|| {
        QueryError::builtin(format!(
            "with_folder: cannot parse folder {}",
            py_str_repr(&f)
        ))
    })?;
    let result = BigIpName {
        folder: new_folder,
        name: obj.name,
    };
    Ok(Value::Str(result.to_string()))
}

pub(super) fn bi_with_name(args: &[Value]) -> Result<Value, QueryError> {
    let p = as_str(&args[0], "with_name", 1)?;
    let n = as_str(&args[1], "with_name", 2)?.trim().to_string();
    if n.is_empty() {
        return Err(QueryError::builtin("with_name: new name must not be empty"));
    }
    match BigIpName::try_parse(&p) {
        Some(obj) => {
            let result = BigIpName {
                folder: obj.folder,
                name: n,
            };
            Ok(Value::Str(result.to_string()))
        }
        None => {
            if !p.is_empty() && !p.contains('/') {
                Ok(Value::Str(n))
            } else {
                Err(QueryError::builtin(format!(
                    "with_name: cannot parse path {}",
                    py_str_repr(&p)
                )))
            }
        }
    }
}

pub(super) fn bi_in_partition(args: &[Value]) -> Result<Value, QueryError> {
    let p = coerce_pathlike(&args[0], "in_partition", 1)?;
    let part_text = as_str(&args[1], "in_partition", 2)?;
    let Some(obj) = BigIpName::try_parse(&p) else {
        return Ok(Value::Bool(false));
    };
    let Some(target) = Partition::try_parse(&part_text) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(*obj.partition() == target))
}

pub(super) fn bi_in_folder(args: &[Value]) -> Result<Value, QueryError> {
    let p = coerce_pathlike(&args[0], "in_folder", 1)?;
    let f = as_str(&args[1], "in_folder", 2)?;
    let obj = BigIpName::try_parse(&p);
    let target_folder = Folder::try_parse(&f);
    let (Some(obj), Some(target)) = (obj, target_folder) else {
        return Ok(Value::Bool(false));
    };
    if obj.folder.partition != target.partition {
        return Ok(Value::Bool(false));
    }
    if target.segments.len() > obj.folder.segments.len() {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(
        obj.folder.segments[..target.segments.len()] == target.segments[..],
    ))
}

pub(super) fn bi_can_see(args: &[Value]) -> Result<Value, QueryError> {
    let r_text = as_str(&args[0], "can_see", 1)?;
    let t_text = as_str(&args[1], "can_see", 2)?;
    let r = BigIpName::try_parse(&r_text);
    let t = BigIpName::try_parse(&t_text);
    let (Some(r), Some(t)) = (r, t) else {
        return Ok(Value::Bool(false));
    };
    Ok(Value::Bool(r.partition().can_see(t.partition())))
}
