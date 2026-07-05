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

/// Resolve a well-known TCP/UDP port to its canonical service name — the reverse
/// of [`getservbyname`], the built-in F5 default service map. Used to label a
/// destination/member port in the report (a manifest `service-map -file` CSV can
/// override this). Returns `None` for a port with no well-known name.
pub(crate) fn getservbyport(port: i64) -> Option<&'static str> {
    Some(match port {
        20 => "ftp-data",
        21 => "ftp",
        22 => "ssh",
        23 => "telnet",
        25 => "smtp",
        53 => "domain",
        80 => "http",
        110 => "pop3",
        123 => "ntp",
        143 => "imap",
        161 => "snmp",
        389 => "ldap",
        443 => "https",
        465 => "smtps",
        514 => "syslog",
        636 => "ldaps",
        993 => "imaps",
        995 => "pop3s",
        3306 => "mysql",
        3389 => "rdp",
        5060 => "sip",
        5061 => "sips",
        5432 => "postgresql",
        8080 => "http-alt",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_port_roundtrip() {
        assert_eq!(getservbyname("https"), Some(443));
        assert_eq!(getservbyport(443), Some("https"));
        assert_eq!(getservbyport(80), Some("http"));
        assert_eq!(getservbyport(3389), Some("rdp"));
        assert_eq!(getservbyport(12345), None);
    }
}
