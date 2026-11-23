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

/// Select a default label for a port in the report.
/// A manifest `service-map -file` CSV can override these labels.
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
    fn default_service_labels() {
        assert_eq!(getservbyport(443), Some("https"));
        assert_eq!(getservbyport(80), Some("http"));
        assert_eq!(getservbyport(3389), Some("rdp"));
        assert_eq!(getservbyport(12345), None);
    }
}
