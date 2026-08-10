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

//! `expr` math functions as a **registry query** — the bare-name ⇄ qualified
//! `::tcl::mathfunc::NAME` mapping, plus the two availability axes a consumer
//! has to keep apart.
//!
//! Inside an `expr` expression a function-call word `NAME(` is *not* an
//! ordinary command lookup: C Tcl compiles it to a dispatch on
//! `tcl::mathfunc::NAME` resolved **relative to the calling namespace, then
//! globally** (`tclCompExpr.c`'s `CompileMathFuncCall` / `tclExecute.c`'s
//! `INST_INVOKE` path; TIP 232). Verified against tclsh 8.6.16 and 9.0.4:
//!
//! ```tcl
//! namespace eval ::tcl::mathfunc { proc g {x} { expr {$x + 1000} } }
//! namespace eval ::foo {
//!     namespace eval tcl::mathfunc { proc f {x} { expr {$x * 100} } }
//!     proc use  {} { expr {f(2)} }   ;# 200  — ::foo::tcl::mathfunc::f
//!     proc useg {} { expr {g(2)} }   ;# 1002 — ::tcl::mathfunc::g (global)
//! }
//! proc f {x} { return GLOBAL-PROC-f }
//! ```
//!
//! A plain global `proc f` never enters that resolution, and conversely a
//! `proc` living in a `tcl::mathfunc` namespace is *not* reachable as a bare
//! command (`li {10 20 30} 1` → `invalid command name "li"` on both oracles) —
//! [`is_in_mathfunc_namespace`] is what lets a consumer keep the two apart
//! without naming a namespace itself.
//!
//! # The two availability axes
//!
//! | axis | question | helper |
//! |---|---|---|
//! | `expr` grammar | is `NAME(…)` a built-in function here? | [`available_in_expr`] |
//! | command table | does the command `::tcl::mathfunc::NAME` exist here? | [`command_wrappers_available`] + the spec's own `dialects` |
//!
//! They genuinely differ: `expr {abs(1)}` works in 8.4, but the *command*
//! `::tcl::mathfunc::abs` only exists from 8.5 (TIP 232 created the table).
//! So the [`CommandSpec`](crate::spec::CommandSpec) gate that
//! `mathfunc_generated.rs` writes (never looser than
//! [`DialectSet::TCL85_PLUS`](crate::dialects::DialectSet)) is the right gate
//! for the *qualified command* spelling and the **wrong** one for the bare
//! in-`expr` spelling — [`CommandRegistry::math_function_spec`] exists so a
//! consumer rendering the in-`expr` spelling gets the registry's hover data
//! under the `expr`-grammar gate instead of the command-table gate.
//!
//! [`tcl_syntax::expr::mathfunc`] stays the layer-1 fact table (which names
//! exist, and since when); this module is the single place the *registry*
//! answers mathfunc questions, so no consumer re-derives the
//! `tcl::mathfunc` prefix or the version ceiling for itself.

use tcl_dialect::{DialectProfile, TclVersion};
use tcl_syntax::expr::mathfunc::MathFuncSince;

use crate::registry::CommandRegistry;
use crate::spec::CommandSpec;

/// The absolute namespace every `expr` math function dispatches through.
pub const MATHFUNC_NAMESPACE: &str = "::tcl::mathfunc";

/// The relative spelling of [`MATHFUNC_NAMESPACE`] — what a namespace-local
/// override is written as (`namespace eval tcl::mathfunc { proc f … }`) and
/// the suffix a caller-relative candidate carries.
pub const MATHFUNC_NAMESPACE_RELATIVE: &str = "tcl::mathfunc";

/// The absolute command name a bare `expr` function word `bare` dispatches to
/// when it resolves globally.
#[must_use]
pub fn qualified_name(bare: &str) -> String {
    format!("{MATHFUNC_NAMESPACE}::{bare}")
}

/// The bare function name when `qualified` addresses a direct command in the
/// global [`MATHFUNC_NAMESPACE`], or `None` for every other command spelling.
///
/// This keeps the global command-table shape owned by the registry, so runtime
/// enumerators never need to reconstruct the namespace prefix themselves.
#[must_use]
pub fn global_command_bare_name(qualified: &str) -> Option<&str> {
    let bare = qualified
        .trim_start_matches("::")
        .strip_prefix(MATHFUNC_NAMESPACE_RELATIVE)?
        .strip_prefix("::")?;
    (!bare.is_empty() && !bare.contains("::")).then_some(bare)
}

/// Whether `qualified` names a command inside *a* `tcl::mathfunc` namespace —
/// the global `::tcl::mathfunc::sin` or a namespace-local override
/// `::foo::tcl::mathfunc::f`.
///
/// A command there is reachable only through `expr`'s function-call
/// production or its own fully-qualified name, never as a bare command word
/// (oracle in the module docs), so a bareword-resolution fallback must not
/// reach into it.
#[must_use]
pub fn is_in_mathfunc_namespace(qualified: &str) -> bool {
    let Some((ns, _tail)) = qualified.rsplit_once("::") else {
        return false;
    };
    ns == MATHFUNC_NAMESPACE
        || ns.ends_with(&format!("::{MATHFUNC_NAMESPACE_RELATIVE}"))
        || ns == MATHFUNC_NAMESPACE_RELATIVE
}

/// The newest math-function release `profile`'s `expr` grammar exposes, or
/// `None` for a profile with no version ceiling (the permissive fallback).
///
/// Gates on the profile's *`expr` grammar* base version — the same axis the
/// relational operators (`in` / `lt` / …) use — so a vendor shell running on
/// an 8.5 core has the 8.5 set even though its dialect tag is not a plain Tcl
/// version.
#[must_use]
pub fn expr_grammar_ceiling(profile: &'static DialectProfile) -> Option<MathFuncSince> {
    Some(match profile.expr_grammar_base? {
        TclVersion::V8_4 => MathFuncSince::Tcl84,
        // TIP 232 landed in 8.5; 8.6 added no math functions over 8.5.
        TclVersion::V8_5 | TclVersion::V8_6 => MathFuncSince::Tcl85,
        TclVersion::V9_0 => MathFuncSince::Tcl90,
        TclVersion::V9_1 => MathFuncSince::Tcl91,
    })
}

/// Whether `bare` is a genuine built-in `expr` math function available under
/// `profile` — the axis that governs the *in-`expr`* spelling `bare(…)`.
#[must_use]
pub fn available_in_expr(bare: &str, profile: &'static DialectProfile) -> bool {
    let Some(since) = tcl_syntax::expr::mathfunc::added_in(bare) else {
        return false;
    };
    expr_grammar_ceiling(profile).is_none_or(|ceiling| since <= ceiling)
}

/// Whether `profile` exposes math functions as literal `::tcl::mathfunc::*`
/// **commands** — TIP 232's wrapper mechanism, landed in 8.5.
///
/// Coarser than [`available_in_expr`]: it does not vary per name, because
/// every wrapper command — even one backing an 8.4-vintage function like
/// `sin` — only exists from 8.5 onward.
#[must_use]
pub fn command_wrappers_available(profile: &'static DialectProfile) -> bool {
    expr_grammar_ceiling(profile).is_none_or(|ceiling| ceiling >= MathFuncSince::Tcl85)
}

impl CommandRegistry {
    /// The registry [`CommandSpec`] backing the **in-`expr`** spelling of the
    /// bare math-function word `bare` under `profile`, or `None` when `bare`
    /// is not a built-in `expr` function this profile has.
    ///
    /// The mapping bare → `::tcl::mathfunc::bare` is data-driven (the spec is
    /// looked up by name; `mathfunc_generated.rs` owns the entries), and the
    /// gate is the *`expr` grammar* one ([`available_in_expr`]) rather than
    /// the spec's own command-table `dialects` — see the module docs for why
    /// the two differ. Consumers rendering the *qualified command* spelling
    /// keep using [`CommandRegistry::get_for_dialect`] and get the tighter
    /// gate, as they should.
    #[must_use]
    pub fn math_function_spec(
        &self,
        bare: &str,
        profile: &'static DialectProfile,
    ) -> Option<&CommandSpec> {
        if !available_in_expr(bare, profile) {
            return None;
        }
        self.get(&qualified_name(bare))
    }

    /// Every built-in `expr` math function available under `profile`, by bare
    /// name, sourced from the registry's own `::tcl::mathfunc::*` entries
    /// (so a name with no registry data never surfaces) and gated on the
    /// `expr`-grammar axis. Sorted, deduplicated.
    #[must_use]
    pub fn math_function_names(&self, profile: &'static DialectProfile) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .command_names()
            .filter_map(|name| name.strip_prefix(MATHFUNC_NAMESPACE)?.strip_prefix("::"))
            .filter(|bare| !bare.contains("::") && available_in_expr(bare, profile))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str) -> &'static DialectProfile {
        DialectProfile::by_name(name)
    }

    #[test]
    fn qualified_name_uses_the_absolute_namespace() {
        assert_eq!(qualified_name("sin"), "::tcl::mathfunc::sin");
    }

    #[test]
    fn global_command_bare_name_accepts_only_direct_global_members() {
        assert_eq!(
            global_command_bare_name("::tcl::mathfunc::sin"),
            Some("sin")
        );
        assert_eq!(global_command_bare_name("tcl::mathfunc::sin"), Some("sin"));
        // TN: a nested command and an unrelated namespace are not global
        // math-function command-table entries.
        assert_eq!(
            global_command_bare_name("::tcl::mathfunc::nested::sin"),
            None
        );
        assert_eq!(global_command_bare_name("::tcl::mathop::sin"), None);
    }

    #[test]
    fn mathfunc_namespace_membership_covers_global_and_namespace_local() {
        assert!(is_in_mathfunc_namespace("::tcl::mathfunc::sin"));
        assert!(is_in_mathfunc_namespace("::foo::tcl::mathfunc::f"));
        assert!(is_in_mathfunc_namespace("tcl::mathfunc::f"));
        // TN: an ordinary command, and the sibling operator namespace.
        assert!(!is_in_mathfunc_namespace("::sin"));
        assert!(!is_in_mathfunc_namespace("::tcl::mathop::+"));
        assert!(!is_in_mathfunc_namespace("::mathfunc::sin"));
        assert!(!is_in_mathfunc_namespace("sin"));
    }

    /// The `expr`-grammar axis, not the command-table one: `abs(…)` is an 8.4
    /// function even though the command `::tcl::mathfunc::abs` is 8.5+.
    #[test]
    fn expr_availability_follows_the_expr_grammar_axis() {
        assert!(available_in_expr("abs", profile("tcl8.4")));
        assert!(available_in_expr("sin", profile("tcl8.4")));
        // TIP 232 names are 8.5+.
        assert!(!available_in_expr("max", profile("tcl8.4")));
        assert!(available_in_expr("max", profile("tcl8.5")));
        // TIP 521 names are 9.0+.
        assert!(!available_in_expr("isnan", profile("tcl8.6")));
        assert!(available_in_expr("isnan", profile("tcl9.0")));
        // TIP 745 names are 9.1+.
        assert!(!available_in_expr("gamma", profile("tcl9.0")));
        assert!(available_in_expr("gamma", profile("tcl9.1")));
        // TN: not a math function at all.
        assert!(!available_in_expr("zzznope", profile("tcl9.1")));
    }

    #[test]
    fn command_wrappers_start_at_tcl85() {
        assert!(!command_wrappers_available(profile("tcl8.4")));
        assert!(command_wrappers_available(profile("tcl8.5")));
        assert!(command_wrappers_available(profile("tcl9.0")));
    }

    #[test]
    fn math_function_spec_serves_registry_hover_data_under_the_expr_gate() {
        let reg = crate::registry_for_dialect("tcl8.6");
        let spec = reg
            .math_function_spec("sin", profile("tcl8.6"))
            .expect("sin has registry data under 8.6");
        assert_eq!(spec.name, "::tcl::mathfunc::sin");
        assert!(spec.hover.is_some(), "hover data must come from the spec");
        // The 8.4 profile still gets `sin`'s spec for the in-expr spelling,
        // even though the spec's own command-table gate is TCL85_PLUS.
        assert!(reg.math_function_spec("sin", profile("tcl8.4")).is_some());
        // Version gate: `isnan` is 9.0+.
        let reg9 = crate::registry_for_dialect("tcl9.0");
        assert!(
            reg9.math_function_spec("isnan", profile("tcl9.0"))
                .is_some()
        );
        assert!(reg.math_function_spec("isnan", profile("tcl8.6")).is_none());
        // TN: not a math function.
        assert!(reg.math_function_spec("puts", profile("tcl8.6")).is_none());
    }

    #[test]
    fn math_function_names_are_bare_sorted_and_gated() {
        let reg = crate::registry_for_dialect("tcl9.0");
        let names = reg.math_function_names(profile("tcl9.0"));
        assert!(names.contains(&"sin"), "{names:?}");
        assert!(names.contains(&"max"), "{names:?}");
        assert!(names.contains(&"isnan"), "{names:?}");
        assert!(!names.contains(&"gamma"), "9.1-only: {names:?}");
        assert!(
            names.windows(2).all(|w| w[0] < w[1]),
            "sorted + deduped: {names:?}"
        );
        assert!(
            names.iter().all(|n| !n.contains("::")),
            "bare names only: {names:?}"
        );
        let names84 = reg.math_function_names(profile("tcl8.4"));
        assert!(names84.contains(&"sin"));
        assert!(!names84.contains(&"max"), "8.5-only: {names84:?}");
    }
}
