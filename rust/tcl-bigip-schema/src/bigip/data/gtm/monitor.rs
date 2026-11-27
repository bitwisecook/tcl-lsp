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

//! BIG-IP object specs for `gtm monitor`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod bigip;
mod bigip_link;
mod external;
mod firepass;
mod ftp;
mod gateway_icmp;
mod gtp;
mod http;
mod https;
mod imap;
mod ldap;
mod mssql;
mod mysql;
mod nntp;
mod oracle;
mod pop3;
mod postgresql;
mod radius;
mod radius_accounting;
mod real_server;
mod scripted;
mod sip;
mod smtp;
mod snmp;
mod snmp_link;
mod soap;
mod tcp;
mod tcp_half_open;
mod udp;
mod wap;
mod wmi;

/// The spec tables of `gtm monitor`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        bigip::SPECS,
        bigip_link::SPECS,
        external::SPECS,
        firepass::SPECS,
        ftp::SPECS,
        gateway_icmp::SPECS,
        gtp::SPECS,
        http::SPECS,
        https::SPECS,
        imap::SPECS,
        ldap::SPECS,
        mssql::SPECS,
        mysql::SPECS,
        nntp::SPECS,
        oracle::SPECS,
        pop3::SPECS,
        postgresql::SPECS,
        radius::SPECS,
        radius_accounting::SPECS,
        real_server::SPECS,
        scripted::SPECS,
        sip::SPECS,
        smtp::SPECS,
        snmp::SPECS,
        snmp_link::SPECS,
        soap::SPECS,
        tcp::SPECS,
        tcp_half_open::SPECS,
        udp::SPECS,
        wap::SPECS,
        wmi::SPECS,
    ]
}
