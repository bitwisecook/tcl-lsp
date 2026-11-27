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

//! BIG-IP object specs for `ltm profile`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod analytics;
mod certificate_authority;
mod classification;
mod client_ldap;
mod client_ssl;
mod connector;
mod dhcpv4;
mod dhcpv6;
mod diameter;
mod dns;
mod dns_logging;
mod doh_proxy;
mod doh_server;
mod fasthttp;
mod fastl4;
mod fix;
mod ftp;
mod georedundancy;
mod gtp;
mod html;
mod http;
mod http2;
mod http3;
mod http_compression;
mod httprouter;
mod icap;
mod iiop;
mod ilx;
mod imap;
mod ipother;
mod ipsecalg;
mod json;
mod mapt;
mod mblb;
mod mqtt;
mod mr_ratelimit;
mod mr_ratelimit_action;
mod mssql;
mod netflow;
mod ntlm;
mod ocsp;
mod one_connect;
mod pcp;
mod pop3;
mod pptp;
mod qoe;
mod quic;
mod radius;
mod ramcache;
mod request_adapt;
mod request_log;
mod response_adapt;
mod rewrite;
mod rtsp;
mod sctp;
mod server_ldap;
mod server_ssl;
mod sip;
mod smtp;
mod smtps;
mod socks;
mod splitsessionclient;
mod splitsessionserver;
mod sse;
mod statistics;
mod stream;
mod tcp;
mod tcp_analytics;
mod tdr;
mod tftp;
mod traffic_acceleration;
mod udp;
mod wa_cache;
mod web_acceleration;
mod web_security;
mod websocket;
mod xml;

/// The spec tables of `ltm profile`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        analytics::SPECS,
        certificate_authority::SPECS,
        classification::SPECS,
        client_ldap::SPECS,
        client_ssl::SPECS,
        connector::SPECS,
        dhcpv4::SPECS,
        dhcpv6::SPECS,
        diameter::SPECS,
        dns::SPECS,
        dns_logging::SPECS,
        doh_proxy::SPECS,
        doh_server::SPECS,
        fasthttp::SPECS,
        fastl4::SPECS,
        fix::SPECS,
        ftp::SPECS,
        georedundancy::SPECS,
        gtp::SPECS,
        html::SPECS,
        http::SPECS,
        http2::SPECS,
        http3::SPECS,
        http_compression::SPECS,
        httprouter::SPECS,
        icap::SPECS,
        iiop::SPECS,
        ilx::SPECS,
        imap::SPECS,
        ipother::SPECS,
        ipsecalg::SPECS,
        json::SPECS,
        mapt::SPECS,
        mblb::SPECS,
        mqtt::SPECS,
        mr_ratelimit::SPECS,
        mr_ratelimit_action::SPECS,
        mssql::SPECS,
        netflow::SPECS,
        ntlm::SPECS,
        ocsp::SPECS,
        one_connect::SPECS,
        pcp::SPECS,
        pop3::SPECS,
        pptp::SPECS,
        qoe::SPECS,
        quic::SPECS,
        radius::SPECS,
        ramcache::SPECS,
        request_adapt::SPECS,
        request_log::SPECS,
        response_adapt::SPECS,
        rewrite::SPECS,
        rtsp::SPECS,
        sctp::SPECS,
        server_ldap::SPECS,
        server_ssl::SPECS,
        sip::SPECS,
        smtp::SPECS,
        smtps::SPECS,
        socks::SPECS,
        splitsessionclient::SPECS,
        splitsessionserver::SPECS,
        sse::SPECS,
        statistics::SPECS,
        stream::SPECS,
        tcp::SPECS,
        tcp_analytics::SPECS,
        tdr::SPECS,
        tftp::SPECS,
        traffic_acceleration::SPECS,
        udp::SPECS,
        wa_cache::SPECS,
        web_acceleration::SPECS,
        web_security::SPECS,
        websocket::SPECS,
        xml::SPECS,
    ]
}
