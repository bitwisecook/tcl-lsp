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

//! A tiny `/etc/services` stand-in for named-port resolution.
//!
//! `f5report.graph._port_num` falls back to `socket.getservbyname` when a
//! virtual's port survives projection as a service name rather than a number.
//! That is rare (BIG-IP destinations are almost always numeric), and there is
//! no `/etc/services` in a browser, so we resolve the handful of names that
//! actually appear on F5 listeners from a built-in table.

/// Resolve a lowercase service name to its well-known TCP/UDP port.
pub(crate) fn getservbyname(name: &str) -> Option<i64> {
    let n = name.to_ascii_lowercase();
    let port = match n.as_str() {
        "ftp-data" => 20,
        "ftp" => 21,
        "ssh" => 22,
        "telnet" => 23,
        "smtp" => 25,
        "domain" => 53,
        "http" | "www" => 80,
        "pop3" => 110,
        "ntp" => 123,
        "imap" | "imap2" => 143,
        "snmp" => 161,
        "ldap" => 389,
        "https" => 443,
        "smtps" => 465,
        "syslog" => 514,
        "ldaps" => 636,
        "imaps" => 993,
        "pop3s" => 995,
        "mysql" => 3306,
        "rdp" | "ms-wbt-server" => 3389,
        "sip" => 5060,
        "sips" => 5061,
        "postgresql" | "postgres" => 5432,
        "https-alt" | "http-alt" => 8080,
        _ => return None,
    };
    Some(port)
}
