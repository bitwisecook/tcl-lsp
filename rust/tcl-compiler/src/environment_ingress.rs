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

//! The compiler's dialect-name ingress — a thin delegation to the shared
//! seam in [`tcl_registry::model::ingress`] (centralisation contract R-a,
//! ledger row C2 — the compiler half of P1-F).
//!
//! Wave 1 landed this module here; wave 2 moved the implementation into
//! the registry model so `tcl-lsp-core`, `tcl-lsp-db`, and
//! `tcl-lsp-server` resolve names through the *same* seam rather than a
//! second copy. The re-exports below keep every compiler call site (and
//! this module's wave-1 tests) unchanged; see the shared module's docs
//! for the resolution rules and the three accepted micro-unifications.

use std::sync::{Mutex, OnceLock};

pub(crate) use tcl_registry::model::ingress::{
    DocumentEnvironment, context_for_profile, irules_context, resolve_environment,
};

/// Intern `name` as a `&'static str` — transitional plumbing for the
/// version-gate axis, whose `Package` arm predates the model's
/// `Arc<str>` package names. Bounded by the compiled placement
/// vocabulary (each distinct name leaks once). Retired with the axis's
/// re-typing in P1-G. Compiler-local: it is not a dialect ingress, so it
/// stays out of the shared seam.
#[must_use]
pub(crate) fn interned_package_name(name: &str) -> &'static str {
    static CELL: OnceLock<Mutex<Vec<&'static str>>> = OnceLock::new();
    let interned = CELL.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = interned.lock().expect("package-name intern mutex");
    if let Some(&existing) = guard.iter().find(|&&existing| existing == name) {
        return existing;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    guard.push(leaked);
    leaked
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tcl_registry::model::KeyedVersions;

    #[test]
    fn names_resolve_as_the_old_ingress_did() {
        // Catalogue names and aliases → their same-named profile.
        for (name, profile) in [
            ("tcl8.6", "tcl8.6"),
            ("f5-irules", "f5-irules"),
            ("irules", "f5-irules"),
            ("tcl-irule", "f5-irules"),
            ("f5-iapps", "f5-iapps"),
            ("xilinx-eda-tcl", "xilinx-eda-tcl"),
        ] {
            let environment = resolve_environment(name);
            assert_eq!(environment.definition.id.as_str(), profile, "{name}");
            assert_eq!(environment.analyser_profile().name, profile, "{name}");
            assert_eq!(environment.unit_profile().name, profile, "{name}");
            assert!(!environment.is_tk(), "{name}");
        }
        // Unknown names and the bare `tcl` land on the lenient
        // environment and the permissive fallback profile.
        for name in ["", "tcl", "no-such-dialect"] {
            let environment = resolve_environment(name);
            assert_eq!(environment.definition.id.as_str(), "tcl", "{name}");
            assert!(environment.analyser_profile().is_fallback(), "{name}");
            assert!(environment.unit_profile().is_fallback(), "{name}");
            assert!(!environment.is_tk(), "{name}");
        }
        // The `tk` ingress: permissive analyser profile (the old
        // `by_name` answer), typed additive unit profile (the old
        // `resolve_known` answer), and the Tk-environment fact set.
        let tk = resolve_environment("tk");
        assert!(tk.is_tk());
        assert!(tk.analyser_profile().is_fallback());
        assert_eq!(tk.unit_profile().name, "tk");
    }

    #[test]
    fn context_registries_carry_the_expected_stores() {
        let environment = resolve_environment("tcl8.5");
        let generation = environment.context_registry(&KeyedVersions::default(), 0);
        assert_eq!(
            generation.context().environment.id.as_str(),
            "tcl8.5",
            "the generation answers under the resolved environment"
        );
        // An uninstalled pack overlay falls back to the un-overlaid
        // generation, exactly as the old if-built door did.
        let fallback = environment.context_registry(&KeyedVersions::default(), 0xDEAD);
        assert!(Arc::ptr_eq(generation.commands(), fallback.commands()));
    }

    #[test]
    fn package_names_intern_stably() {
        let first = interned_package_name("f5-irules-cmds");
        let second = interned_package_name("f5-irules-cmds");
        assert!(std::ptr::eq(first, second));
        assert_eq!(first, "f5-irules-cmds");
    }
}
