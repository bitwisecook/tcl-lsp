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

//! BIG-IP object specs for the `pem` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("pem")`).
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

mod forwarding_endpoint;
mod global_settings;
mod interception_endpoint;
mod irule;
mod listener;
mod policy;
mod profile;
mod protocol;
mod quota_mgmt;
mod reporting;
mod service_chain_endpoint;
mod subscriber;
mod subscriber_attribute;

/// The spec tables of `pem`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        forwarding_endpoint::SPECS,
        global_settings::SPECS,
        interception_endpoint::SPECS,
        irule::SPECS,
        listener::SPECS,
        policy::SPECS,
        profile::SPECS,
        protocol::SPECS,
        quota_mgmt::SPECS,
        reporting::SPECS,
        service_chain_endpoint::SPECS,
        subscriber::SPECS,
        subscriber_attribute::SPECS,
    ]
}
