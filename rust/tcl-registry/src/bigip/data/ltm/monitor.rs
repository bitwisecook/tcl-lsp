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

//! BIG-IP object specs for `ltm monitor`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod diameter;
mod dns;
mod external;
mod firepass;
mod ftp;
mod gateway_icmp;
mod http;
mod http2;
mod https;
mod icmp;
mod imap;
mod inband;
mod ldap;
mod module_score;
mod mqtt;
mod mssql;
mod mysql;
mod nntp;
mod oracle;
mod pop3;
mod postgresql;
mod radius;
mod radius_accounting;
mod real_server;
mod rpc;
mod sasp;
mod scripted;
mod sip;
mod smb;
mod smtp;
mod snmp_dca;
mod snmp_dca_base;
mod soap;
mod tcp;
mod tcp_echo;
mod tcp_half_open;
mod udp;
mod virtual_location;
mod wap;
mod wmi;

/// The spec tables of `ltm monitor`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        diameter::SPECS,
        dns::SPECS,
        external::SPECS,
        firepass::SPECS,
        ftp::SPECS,
        gateway_icmp::SPECS,
        http::SPECS,
        http2::SPECS,
        https::SPECS,
        icmp::SPECS,
        imap::SPECS,
        inband::SPECS,
        ldap::SPECS,
        module_score::SPECS,
        mqtt::SPECS,
        mssql::SPECS,
        mysql::SPECS,
        nntp::SPECS,
        oracle::SPECS,
        pop3::SPECS,
        postgresql::SPECS,
        radius::SPECS,
        radius_accounting::SPECS,
        real_server::SPECS,
        rpc::SPECS,
        sasp::SPECS,
        scripted::SPECS,
        sip::SPECS,
        smb::SPECS,
        smtp::SPECS,
        snmp_dca::SPECS,
        snmp_dca_base::SPECS,
        soap::SPECS,
        tcp::SPECS,
        tcp_echo::SPECS,
        tcp_half_open::SPECS,
        udp::SPECS,
        virtual_location::SPECS,
        wap::SPECS,
        wmi::SPECS,
    ]
}
