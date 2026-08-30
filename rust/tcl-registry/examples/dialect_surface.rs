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

//! Resolved command surface per dialect profile.
//!
//! Emits one `profile<TAB>command` line for every command that resolves
//! (is available) under each catalog [`DialectProfile`], profiles in
//! catalog order and commands sorted within each. This is the differential
//! guard for availability-preserving refactors (e.g. the EDA-as-packages
//! migration): capture it before a change, re-run after, and diff — the
//! surface must be identical except where a change is intended.
//!
//! Usage: `cargo run -q --example dialect_surface`

use tcl_dialect::DialectProfile;
use tcl_registry::model::ingress::{
    static_context_for, static_document_context_for_profile as ctx_for,
};

fn main() {
    let mut out = String::new();
    for profile in DialectProfile::all() {
        let registry = static_context_for(profile.name).commands();
        let mut names: Vec<&str> = registry
            .command_names()
            .filter(|name| ctx_for(profile).resolve_spec(registry, name).is_some())
            .collect();
        names.sort_unstable();
        names.dedup();
        for name in names {
            out.push_str(profile.name);
            out.push('\t');
            out.push_str(name);
            out.push('\n');
        }
    }
    print!("{out}");
}
