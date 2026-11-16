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

//! C-Tcl-observable behaviour of the command registry:
//!   - registry structure / switches / options / argument values / hover /
//!     subcommands / dialect scoping
//!   - per-dialect signature membership
//!   - the `is_irules_dialect` predicate
//!   - the dialect-name surface (see the NOTE below)
//!
//! These assert behaviour on the `tcl_registry` crate via `reg.get_for_surface`,
//! `spec.switch_names(Some(ds))`, the `spec.options` scan, `spec.arg_values_at`,
//! `reg.command_names()`, and per-dialect `reg.get_for_surface`.
//!
//! NOTE on dialect detection: the dialect-detection function
//! (`detect_dialect_from_source`) does NOT live in the `tcl-registry` crate —
//! it is implemented in `tcl-compiler` / `tcl-lsp-core`, which are out of
//! scope for this crate's tests. The
//! dialect-name surface that *is* in `tcl-registry` (`DialectProfile::find`,
//! `tcl_dialect::DialectProfile::name_is_irules`) is covered below instead.
//!
//! ## C-Tcl proof
//! Facts observable in real Tcl (core-command existence, which switches a
//! command accepts, the subcommand set of `string`/`dict`/`info`, the
//! `socket -server` option, the 9.0-only `const`) were verified with
//! `scripts/dev/tclsh_check.sh` against tclsh8.6 and tclsh9.0 and are recorded
//! in `// tclsh:` comments. F5/iRules commands and the `f5-irules` dialect are
//! not real Tcl — they are marked `// f5-dialect`. Hover summary text and
//! arg-role classification are registry-internal metadata, marked
//! `// registry-metadata`.

use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::{Family, PackageFloor, SurfaceLayer};
use tcl_registry::arity::Arity;
use tcl_registry::events::EventRegistry;
use tcl_registry::model::ingress::{static_context_for, static_document_context_for};
use tcl_registry::profiles::ProfileRegistry;
use tcl_registry::{
    ArgRole, CommandRegistry, DataCollectionAction, MethodDispatchKind,
    PayloadCollectionRequirement, SideSwitchTarget, Traits,
};

mod dialect_membership;
mod irules;
mod object_model;
mod spec_structure;
mod tcloo_method_context;
mod trait_membership;

// Helpers

/// `static_context_for(name).commands()` returns a registry with that dialect loaded;
/// pair it with the parsed `SpecSurface` for `get_for_surface`.
fn reg_and_set(dialect: &str) -> (&'static CommandRegistry, Option<SurfaceQuery<'static>>) {
    // "Available under dialect D" is membership in D's *profile availability
    // point*, not the bare single dialect bit. For the additive vendor
    // dialects the mask is `(base Tcl version | vendor bit)` — e.g. expect is
    // `TCL86 | EXPECT` — so a core command (an explicit `ALL_TCL` surface)
    // resolves through the version half. Using the
    // bare bit here would wrongly exclude the whole Tcl core from every vendor
    // dialect.
    (
        static_context_for(dialect).commands(),
        Some(
            tcl_registry::model::ingress::resolve_environment(dialect)
                .analyser_profile()
                .surface_query(),
        ),
    )
}

/// Whether `cmd` resolves in `dialect`.
fn present_in(dialect: &str, cmd: &str) -> bool {
    let (reg, ds) = reg_and_set(dialect);
    reg.get_for_surface(cmd, ds).is_some()
}

// tcllib `textutil::adjust` submodule vs the `textutil`
// umbrella package's flattened re-export aliases.
//
// Ground truth (tclsh 9.0.4 + real tcllib-2.0 `modules/textutil/`, verified
// directly — see the PR's commit message for the exact transcripts):
//   - `package require textutil::adjust` alone creates only the three-segment
//     `::textutil::adjust::adjust` / `::textutil::adjust::indent` /
//     `::textutil::adjust::undent` (plus `readPatterns`/`listPredefined`/
//     `getPredefined`) — no bare `::textutil::adjust` command exists.
//   - `package require textutil` (the umbrella) additionally creates the
//     flattened `::textutil::adjust` / `::textutil::indent` / `::textutil::undent`
//     aliases via `namespace import -force adjust::adjust adjust::indent
//     adjust::undent` — real and callable, but ONLY after requiring the
//     umbrella, never after requiring the submodule alone.
// The same shape recurs in `textutil::trim` (trim/trimleft/trimright) and
// `textutil::tabify` (tabify/untabify/tabify2/untabify2).
mod textutil_submodule_vs_umbrella {
    use super::*;

    /// TP — the real submodule-qualified commands the common `package
    /// require textutil::adjust; namespace import textutil::adjust::*`
    /// idiom (tcllib-consuming corpora like `georgtree_argparse`) actually
    /// resolves to must be registered, gated on the submodule package.
    #[test]
    fn textutil_adjust_submodule_commands_registered() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for (name, pkg) in [
            ("textutil::adjust::adjust", "textutil::adjust"),
            ("textutil::adjust::indent", "textutil::adjust"),
            ("textutil::adjust::undent", "textutil::adjust"),
        ] {
            let spec = reg
                .get_for_surface(name, ds)
                .unwrap_or_else(|| panic!("{name} must be registered"));
            assert_eq!(
                spec.required_package,
                Some(pkg),
                "{name} must be gated on the submodule package, not the umbrella"
            );
        }
    }

    /// TP — the umbrella package's flattened re-export aliases are also
    /// registered, distinctly gated on `textutil` (not `textutil::adjust`).
    #[test]
    fn textutil_umbrella_aliases_registered_distinctly() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for name in ["textutil::adjust", "textutil::indent"] {
            let spec = reg
                .get_for_surface(name, ds)
                .unwrap_or_else(|| panic!("{name} (umbrella alias) must be registered"));
            assert_eq!(
                spec.required_package,
                Some("textutil"),
                "{name} is the umbrella's re-export, gated on requiring `textutil` itself"
            );
        }
    }

    /// TN — the umbrella-vs-submodule distinction is not collapsed: the
    /// 2-segment alias and the 3-segment submodule command are two distinct
    /// registry entries with two distinct package gates, so requiring
    /// `textutil::adjust` alone must not read as granting the umbrella's
    /// `textutil::adjust` alias name (they carry different `required_package`
    /// values even though one's name is a prefix of the other's).
    #[test]
    fn submodule_and_umbrella_names_are_distinct_keys() {
        let (reg, ds) = reg_and_set("tcl8.6");
        let submodule = reg
            .get_for_surface("textutil::adjust::adjust", ds)
            .expect("submodule command registered");
        let umbrella = reg
            .get_for_surface("textutil::adjust", ds)
            .expect("umbrella alias registered");
        assert_ne!(submodule.required_package, umbrella.required_package);
        assert_eq!(umbrella.required_package, Some("textutil"));
        assert_eq!(submodule.required_package, Some("textutil::adjust"));
    }

    /// TN — a name shaped like the wrong guess this finding started from
    /// (`textutil::indent` treated as if it were its own *package*, i.e. a
    /// `textutil::indent::indent` submodule path) does not exist: `indent`
    /// lives inside the `textutil::adjust` package only, and there never was
    /// a separate `textutil::indent` package in real tcllib.
    #[test]
    fn no_fabricated_textutil_indent_package_submodule() {
        let (reg, ds) = reg_and_set("tcl8.6");
        assert!(
            reg.get_for_surface("textutil::indent::indent", ds)
                .is_none()
        );
    }

    /// TP/TN sibling sweep (verified against tclsh 9.0.4 the same way):
    /// `textutil::trim` and `textutil::tabify` show the exact same
    /// meta-package-flattening shape — the submodule-qualified commands must
    /// be registered too, distinctly from their umbrella aliases.
    #[test]
    fn sibling_submodules_trim_and_tabify_also_registered() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for name in [
            "textutil::trim::trim",
            "textutil::trim::trimleft",
            "textutil::trim::trimright",
            "textutil::tabify::tabify",
            "textutil::tabify::untabify",
            "textutil::tabify::tabify2",
            "textutil::tabify::untabify2",
            "textutil::string::longestCommonPrefix",
        ] {
            assert!(
                reg.get_for_surface(name, ds).is_some(),
                "{name} must be registered (tclsh 9.0.4-verified real tcllib command)"
            );
        }
        // Their umbrella aliases remain separately registered too.
        for name in [
            "textutil::trim",
            "textutil::trimleft",
            "textutil::trimright",
            "textutil::tabify",
            "textutil::untabify",
            "textutil::tabify2",
            "textutil::untabify2",
        ] {
            let spec = reg
                .get_for_surface(name, ds)
                .unwrap_or_else(|| panic!("{name} (umbrella alias) must be registered"));
            assert_eq!(spec.required_package, Some("textutil"));
        }
    }

    /// Contract test: the umbrella alias
    /// and the submodule-qualified canonical name are the *same real tcllib
    /// command* under two different call spellings, so every trait
    /// descriptor (`Traits::PURE` and any future one) must agree between
    /// them — a consumer doing purity-based GVN/DCE must not see a different
    /// answer purely because of which spelling a call site happens to use.
    /// `rust/tcl-registry/src/commands/tcllib/misc_ext.rs`'s
    /// `sync_textutil_submodule_traits` is what keeps this true (it derives
    /// the submodule spec's traits from the umbrella alias file rather than
    /// declaring them a second time); this test is the general guard that
    /// catches the *next* drift — a newly added re-exported command whose
    /// submodule row is never wired into that sync list — not just today's.
    #[test]
    fn umbrella_alias_and_submodule_canonical_name_agree_on_traits() {
        let (reg, ds) = reg_and_set("tcl8.6");
        for (umbrella, submodule) in [
            ("textutil::adjust", "textutil::adjust::adjust"),
            ("textutil::indent", "textutil::adjust::indent"),
            ("textutil::undent", "textutil::adjust::undent"),
            ("textutil::trim", "textutil::trim::trim"),
            ("textutil::trimleft", "textutil::trim::trimleft"),
            ("textutil::trimright", "textutil::trim::trimright"),
            ("textutil::tabify", "textutil::tabify::tabify"),
            ("textutil::untabify", "textutil::tabify::untabify"),
            ("textutil::tabify2", "textutil::tabify::tabify2"),
            ("textutil::untabify2", "textutil::tabify::untabify2"),
        ] {
            let umbrella_spec = reg
                .get_for_surface(umbrella, ds)
                .unwrap_or_else(|| panic!("{umbrella} (umbrella alias) must be registered"));
            let submodule_spec = reg.get_for_surface(submodule, ds).unwrap_or_else(|| {
                panic!("{submodule} (submodule canonical name) must be registered")
            });
            assert_eq!(
                umbrella_spec.traits, submodule_spec.traits,
                "{umbrella} and {submodule} are the same real tcllib command \
                 and must carry identical traits (got {:?} vs {:?})",
                umbrella_spec.traits, submodule_spec.traits
            );
        }
        // The `textutil::string` package has no bare `longestCommonPrefix`
        // 2-segment umbrella alias file of its own name (the umbrella only
        // re-exports it as `textutil::longestCommonPrefix`), covered above
        // by `textutil_umbrella_aliases_registered_distinctly`'s sibling
        // tests; check it here too for the same trait-parity property.
        let umbrella_spec = reg
            .get_for_surface("textutil::longestCommonPrefix", ds)
            .expect("textutil::longestCommonPrefix (umbrella alias) must be registered");
        let submodule_spec = reg
            .get_for_surface("textutil::string::longestCommonPrefix", ds)
            .expect("textutil::string::longestCommonPrefix must be registered");
        assert_eq!(umbrella_spec.traits, submodule_spec.traits);
    }
}
