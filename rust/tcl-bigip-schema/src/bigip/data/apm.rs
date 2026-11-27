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

//! BIG-IP object specs for the `apm` tmsh module.
//!
//! **Hand-maintained**, with no generator to produce these from — edit
//! them directly, in the same shape the other module files use.  Organised
//! by tmsh module name (this file's own module field, `Some("apm")`).
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

mod aaa;
mod acl;
mod apm_avr_config;
mod client;
mod configuration;
mod epsec;
mod log_setting;
mod ntlm;
mod oauth;
mod policy;
mod profile;
mod report;
mod resource;
mod saml;
mod session;
mod sso;
mod swg_scheme;
mod url_filter;

/// The spec tables of `apm`, in the order they are filed.
#[must_use]
pub fn tables() -> Vec<&'static [BigipObjectSpec]> {
    [
        aaa::tables(),
        vec![
            acl::SPECS,
            apm_avr_config::SPECS,
            client::SPECS,
            configuration::SPECS,
            epsec::SPECS,
            log_setting::SPECS,
            ntlm::SPECS,
            oauth::SPECS,
        ],
        policy::tables(),
        profile::tables(),
        vec![report::SPECS],
        resource::tables(),
        vec![
            saml::SPECS,
            session::SPECS,
            sso::SPECS,
            swg_scheme::SPECS,
            url_filter::SPECS,
        ],
    ]
    .concat()
}
