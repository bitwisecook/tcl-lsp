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

//! BIG-IP object specs for the `sys` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("sys")`).
//!
//! `cargo xtask bigip-data-schema --check` enforces the internal
//! invariants a generator would otherwise have guaranteed: every `kind`
//! is globally unique, filed under the module file matching its own
//! `module` field, and every `references` target either names a real
//! kind or is on the documented known-gap list.
//!
//! One file per tmsh component (the word after the module); a large
//! component has one file per following word.

use crate::bigip::BigipObjectSpec;

mod appiq;
mod application;
mod autoscale_group;
mod clock;
mod cluster;
mod config;
mod connection;
mod console;
mod core;
mod crypto;
mod daemon_ha;
mod daemon_log_settings;
mod datastor;
mod db;
mod diags;
mod disk;
mod dns;
mod dynad;
mod ecm;
mod feature_module;
mod file;
mod folder;
mod fpga;
mod global_settings;
mod ha_group;
mod httpd;
mod iapp_restricted_key;
mod iapprestricted;
mod icall;
mod icontrol_soap;
mod internal_proxy;
mod ipfix;
mod log_config;
mod log_rotate;
mod management_dhcp;
mod management_ip;
mod management_ovsdb;
mod management_proxy_config;
mod management_route;
mod ntp;
mod outbound_smtp;
mod pfman;
mod provision;
mod raid;
mod scriptd;
mod service;
mod sflow;
mod smtp_server;
mod snmp;
mod software;
mod sshd;
mod state_mirroring;
mod syslog;
mod turboflex;
mod ucs;
mod url_db;

/// The spec tables of `sys`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        appiq::SPECS,
        application::SPECS,
        autoscale_group::SPECS,
        clock::SPECS,
        cluster::SPECS,
        config::SPECS,
        connection::SPECS,
        console::SPECS,
        core::SPECS,
        crypto::SPECS,
        daemon_ha::SPECS,
        daemon_log_settings::SPECS,
        datastor::SPECS,
        db::SPECS,
        diags::SPECS,
        disk::SPECS,
        dns::SPECS,
        dynad::SPECS,
        ecm::SPECS,
        feature_module::SPECS,
        file::SPECS,
        folder::SPECS,
        fpga::SPECS,
        global_settings::SPECS,
        ha_group::SPECS,
        httpd::SPECS,
        iapp_restricted_key::SPECS,
        iapprestricted::SPECS,
        icall::SPECS,
        icontrol_soap::SPECS,
        internal_proxy::SPECS,
        ipfix::SPECS,
        log_config::SPECS,
        log_rotate::SPECS,
        management_dhcp::SPECS,
        management_ip::SPECS,
        management_ovsdb::SPECS,
        management_proxy_config::SPECS,
        management_route::SPECS,
        ntp::SPECS,
        outbound_smtp::SPECS,
        pfman::SPECS,
        provision::SPECS,
        raid::SPECS,
        scriptd::SPECS,
        service::SPECS,
        sflow::SPECS,
        smtp_server::SPECS,
        snmp::SPECS,
        software::SPECS,
        sshd::SPECS,
        state_mirroring::SPECS,
        syslog::SPECS,
        turboflex::SPECS,
        ucs::SPECS,
        url_db::SPECS,
    ]
}
