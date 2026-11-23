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

//! Pool member keys in TMSH configuration.

use super::{IPAddress, Port};

/// A named or literal node and its service in a TMSH pool member key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberKey<'a> {
    /// Original node reference, including its folders and any route domain.
    pub node: &'a str,
    /// Numeric or named service, with its original spelling.
    pub port: Port,
}

impl<'a> MemberKey<'a> {
    /// Read a service after a colon on a named node or IPv4 address, or
    /// after a dot on a literal IPv6 address.
    #[must_use]
    pub fn try_parse(text: &'a str) -> Option<Self> {
        let leaf = text.rsplit('/').next()?;
        if text.chars().any(char::is_whitespace) || leaf.is_empty() {
            return None;
        }
        let separator = if leaf.bytes().filter(|byte| *byte == b':').count() > 1 {
            let (host, _) = leaf.rsplit_once('.')?;
            IPAddress::try_parse(host).filter(IPAddress::is_ipv6)?;
            '.'
        } else {
            ':'
        };
        let (host, service) = leaf.rsplit_once(separator)?;
        if host.is_empty() {
            return None;
        }
        let node = &text[..text.len() - service.len() - 1];
        Some(Self {
            node,
            port: Port::try_parse_service(service)?,
        })
    }
}

pub(crate) fn split_numeric_service(text: &str, empty: EmptyServiceSuffix) -> Option<(&str, &str)> {
    let (host, service) = text.split_once(':')?;
    ((empty == EmptyServiceSuffix::Strip || !service.is_empty())
        && service.bytes().all(|byte| byte.is_ascii_digit()))
    .then_some((host, service))
}

/// Treatment of an empty service suffix in an authored object reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmptyServiceSuffix {
    /// Leave a trailing colon in the object key.
    Retain,
    /// Remove a trailing colon as well as a numeric service suffix.
    Strip,
}

/// Remove one numeric colon suffix while preserving IPv6 and nonnumeric keys.
#[must_use]
pub fn reference_without_numeric_service(reference: &str, empty: EmptyServiceSuffix) -> &str {
    split_numeric_service(reference, empty).map_or(reference, |(host, _)| host)
}
