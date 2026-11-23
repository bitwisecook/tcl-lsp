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

//! Query argument coercion for the shared BIG-IP address readers.

use tcl_bigip::value::{Address, Destination, Network, try_parse_address};

use super::{Net, parse_network};
use crate::builtins::as_str;
use crate::errors::QueryError;
use crate::value::Value;

/// Project the host, route domain and service for scalar query builtins.
pub(super) fn destination_fields(text: &str) -> (String, String, String) {
    if let Some(destination) = Destination::try_parse(text) {
        let host = destination.address.to_string();
        let domain = destination
            .route_domain
            .map_or_else(String::new, |rd| rd.id.to_string());
        let service = if destination.port.is_any() {
            destination.port.spelling
        } else {
            destination.port.port.to_string()
        };
        return (host, domain, service);
    }
    if let Some(network) = Network::try_parse(text) {
        let domain = network
            .route_domain
            .map_or_else(String::new, |rd| rd.to_string());
        return (text.to_owned(), domain, String::new());
    }
    (text.to_owned(), String::new(), String::new())
}

/// Coerce to a typed `Address`.
pub(super) fn typed_address(value: &Value, name: &str) -> Result<Option<Address>, QueryError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let s = as_str(value, name, 1)?;
    if s.is_empty() {
        return Ok(None);
    }
    if let Some(dest) = Destination::try_parse(&s) {
        return Ok(Some(dest.address));
    }
    Ok(try_parse_address(&s))
}

/// Coerce to a typed `Network`.
pub(super) fn typed_network(
    value: &Value,
    name: &str,
    arg: usize,
) -> Result<Option<Net>, QueryError> {
    if matches!(value, Value::Null) {
        return Ok(None);
    }
    let s = as_str(value, name, arg)?;
    if s.is_empty() {
        return Ok(None);
    }
    Ok(parse_network(&s).ok())
}
