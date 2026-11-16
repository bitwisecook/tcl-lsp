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

//! BIG-IP object specs for the `analytics` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("analytics")`).
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

mod afm_sweeper;
mod application_security;
mod application_security_anomalies;
mod application_security_network;
mod asm_bypass;
mod asm_cpu;
mod asm_memory;
mod asm_violation;
mod cpu;
mod device_traffic;
mod disk_info;
mod dns;
mod dns_protocol;
mod dos_l3;
mod fw_nat;
mod global_settings;
mod http;
mod ip_intelligence;
mod ip_layer;
mod lsn_pool;
mod memory;
mod network;
mod pem;
mod pool_traffic;
mod proc_cpu;
mod protocol_security;
mod protocol_security_http;
mod sip;
mod sip_dos;
mod ssl_orchestrator;
mod ssl_orchestrator_service_virtual;
mod swg;
mod swg_blocked;
mod tcp;
mod tcp_analytics;
mod traffic_classification;
mod udp;
mod uri_type;
mod vcmp;
mod r#virtual;

/// The spec tables of `analytics`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        afm_sweeper::SPECS,
        application_security_anomalies::SPECS,
        application_security_network::SPECS,
        application_security::SPECS,
        asm_bypass::SPECS,
        asm_cpu::SPECS,
        asm_memory::SPECS,
        asm_violation::SPECS,
        cpu::SPECS,
        device_traffic::SPECS,
        disk_info::SPECS,
        dns_protocol::SPECS,
        dns::SPECS,
        dos_l3::SPECS,
        fw_nat::SPECS,
        global_settings::SPECS,
        http::SPECS,
        ip_intelligence::SPECS,
        ip_layer::SPECS,
        lsn_pool::SPECS,
        memory::SPECS,
        network::SPECS,
        pem::SPECS,
        pool_traffic::SPECS,
        proc_cpu::SPECS,
        protocol_security_http::SPECS,
        protocol_security::SPECS,
        sip_dos::SPECS,
        sip::SPECS,
        ssl_orchestrator::SPECS,
        ssl_orchestrator_service_virtual::SPECS,
        swg_blocked::SPECS,
        swg::SPECS,
        tcp_analytics::SPECS,
        tcp::SPECS,
        traffic_classification::SPECS,
        udp::SPECS,
        uri_type::SPECS,
        vcmp::SPECS,
        r#virtual::SPECS,
    ]
}
