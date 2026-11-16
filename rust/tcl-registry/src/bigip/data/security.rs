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

//! BIG-IP object specs for the `security` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("security")`).
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

mod analytics;
mod anti_fraud;
mod blacklist_publisher;
mod bot_defense;
mod cloud_services;
mod datasync;
mod debug;
mod device;
mod device_id;
mod dos;
mod firewall;
mod flowspec_route_injector;
mod http;
mod ip_intelligence;
mod log;
mod nat;
mod packet_filter;
mod protected;
mod protocol_inspection;
mod scrubber;
mod ssh;
mod zone;

/// The spec tables of `security`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    [
        vec![analytics::SPECS],
        anti_fraud::tables(),
        vec![
            blacklist_publisher::SPECS,
            bot_defense::SPECS,
            cloud_services::SPECS,
            datasync::SPECS,
            debug::SPECS,
            device::SPECS,
            device_id::SPECS,
        ],
        dos::tables(),
        firewall::tables(),
        vec![
            flowspec_route_injector::SPECS,
            http::SPECS,
            ip_intelligence::SPECS,
            log::SPECS,
            nat::SPECS,
            packet_filter::SPECS,
            protected::SPECS,
            protocol_inspection::SPECS,
            scrubber::SPECS,
            ssh::SPECS,
            zone::SPECS,
        ],
    ]
    .concat()
}
