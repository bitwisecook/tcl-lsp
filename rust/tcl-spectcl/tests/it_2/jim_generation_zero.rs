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

//! The compiled-in core surfaces are in the registry from the first
//! constructor a consumer calls, before any pack set has been loaded or
//! published.
//!
//! Nothing here calls `core_surfaces::ensure` or `bundled::set_active`: each
//! test goes through one constructor a consumer already uses to obtain a
//! registry or a pack set, and then asks a `jim` context the question a
//! document analysis asks — does this head resolve? — of the assembled
//! generation.

use tcl_registry::model::resolve_environment;

/// Heads only Jim has: a loop command, a timer and the class definer.
const JIM_ONLY: &[&str] = &["loop", "sleep", "class"];

/// A head Tcl 8.6 has and no `jimsh` does, so a `jim` context refuses it once
/// the compiled-in roster is registered.
const NOT_IN_JIM: &str = "coroutine";

fn resolves(environment: &str, head: &str) -> bool {
    resolve_environment(environment)
        .default_context_registry()
        .resolve_command(head)
        .is_some()
}

/// What a `jim` context holds after a constructor has run, with no pack set
/// published behind it.
fn assert_jim_starts_with_its_surface() {
    assert!(
        tcl_spectcl::bundled::active().is_none(),
        "no pack set has been published to the process"
    );
    for head in JIM_ONLY {
        assert!(resolves("jim", head), "`{head}` resolves for a jim context");
        assert!(
            !resolves("tcl8.6", head),
            "`{head}` is Jim's own and does not reach a Tcl 8.6 context"
        );
    }
    assert!(
        !resolves("jim", NOT_IN_JIM),
        "the compiled-in roster narrows what a jim context inherits"
    );
    assert!(
        resolves("tcl8.6", NOT_IN_JIM),
        "the roster does not narrow Tcl 8.6 itself"
    );
}

/// The registry a consumer asks for by dialect name.
#[test]
fn asking_for_a_registry_by_dialect_seeds_the_jim_surface() {
    let _ = tcl_spectcl::bundled::registry_for_dialect("tcl8.6");
    assert_jim_starts_with_its_surface();
}

/// The shipped loadables, which a consumer reads for their key without
/// building a registry.
#[test]
fn loading_the_bundled_packs_seeds_the_jim_surface() {
    let _ = tcl_spectcl::bundled::packs();
    assert_jim_starts_with_its_surface();
}

/// The installer every pack-carrying registry goes through, given no packs.
#[test]
fn installing_no_packs_seeds_the_jim_surface() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").expect("catalogue profile");
    let _ = tcl_spectcl::install::registry_with_packs(profile, &tcl_spectcl::PackSet::default());
    assert_jim_starts_with_its_surface();
}
