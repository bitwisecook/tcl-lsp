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

//! BIG-IP object specs for the `ltm` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("ltm")`).
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

mod alg_log_profile;
mod auth;
mod cipher;
mod classification;
mod clientssl;
mod clientssl_proxy;
mod data_group;
mod default_node_monitor;
mod dns;
mod eviction_policy;
mod global_settings;
mod ifile;
mod lsn_log_profile;
mod lsn_pool;
mod message_routing;
mod monitor;
mod nat;
mod node;
mod persistence;
mod policy;
mod pool;
mod profile;
mod rule;
mod rule_profiler;
mod snat;
mod snat_translation;
mod snatpool;
mod tacdb;
mod traffic_class;
mod traffic_matching_criteria;
mod urlcat_cloud_cache;
mod r#virtual;
mod virtual_address;

/// The spec tables of `ltm`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    [
        vec![
            alg_log_profile::SPECS,
            auth::SPECS,
            cipher::SPECS,
            classification::SPECS,
            clientssl::SPECS,
            clientssl_proxy::SPECS,
            data_group::SPECS,
            default_node_monitor::SPECS,
            dns::SPECS,
            eviction_policy::SPECS,
            global_settings::SPECS,
            ifile::SPECS,
            lsn_log_profile::SPECS,
            lsn_pool::SPECS,
        ],
        message_routing::tables(),
        monitor::tables(),
        vec![
            nat::SPECS,
            node::SPECS,
            persistence::SPECS,
            policy::SPECS,
            pool::SPECS,
        ],
        profile::tables(),
        vec![
            rule::SPECS,
            rule_profiler::SPECS,
            snat::SPECS,
            snat_translation::SPECS,
            snatpool::SPECS,
            tacdb::SPECS,
            traffic_class::SPECS,
            traffic_matching_criteria::SPECS,
            urlcat_cloud_cache::SPECS,
            r#virtual::SPECS,
            virtual_address::SPECS,
        ],
    ]
    .concat()
}
