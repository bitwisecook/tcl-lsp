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

//! Quick-fix safety-classification suite (issue #1195).
//!
//! Every [`CodeFix`](tcl_compiler::analyser::CodeFix) records how much its
//! rewrite changes behaviour.  "Fix All Safe Issues" applies only the
//! `SemanticsEquivalent` class, so a misclassification here is a silent,
//! unattended change to a user's program — which is exactly what the
//! diagnostic-code whitelist this replaced allowed.
//!
//! The four coverage classes, per the issue's acceptance criteria:
//!
//! * **TP** — a rewrite that really is equivalent is classified so, and is
//!   therefore bulk-applicable.
//! * **FP** — a rewrite that changes behaviour is *not* classified
//!   equivalent, so the bulk pass leaves it alone.
//! * **TN** — a diagnostic that carries no fix at all contributes nothing.
//! * **FN** — an equivalent rewrite is not over-classified into a stricter
//!   class than it needs, so genuinely safe work still gets done.

use tcl_compiler::analyser::{Analyser, FixSafety};
use tcl_core_types::DiagCode;

/// `(code, description, safety)` for every fix the analyser attaches to
/// `src` under `dialect`.
fn fixes(src: &str, dialect: &str) -> Vec<(DiagCode, String, FixSafety)> {
    let mut analyser = Analyser::new();
    analyser
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .flat_map(|diag| {
            diag.fixes
                .iter()
                .map(move |fix| (diag.code, fix.description.clone(), fix.safety))
        })
        .collect()
}

/// The safety classes recorded for `code`'s fixes in `src`.
fn safety_for(src: &str, dialect: &str, code: DiagCode) -> Vec<FixSafety> {
    fixes(src, dialect)
        .into_iter()
        .filter(|(diag_code, _, _)| *diag_code == code)
        .map(|(_, _, safety)| safety)
        .collect()
}

// -- The taxonomy itself -------------------------------------------------

#[test]
fn only_the_equivalent_class_is_bulk_applicable() {
    assert!(FixSafety::SemanticsEquivalent.is_bulk_applicable());
    assert!(!FixSafety::BehaviourHardening.is_bulk_applicable());
    assert!(!FixSafety::StyleOnly.is_bulk_applicable());
    assert!(!FixSafety::RequiresReview.is_bulk_applicable());
}

#[test]
fn the_default_class_is_the_cautious_one() {
    // A new emitter that constructs a `CodeFix` without thinking about
    // safety must not silently opt into unattended bulk application.
    assert_eq!(FixSafety::default(), FixSafety::RequiresReview);
    assert!(!FixSafety::default().is_bulk_applicable());
}

// -- W100: the issue's headline case -------------------------------------

#[test]
fn fp_w100_brace_fix_over_a_substituted_operand_is_not_equivalent() {
    // C Tcl 9.0.3:
    //
    //   set a {$x}; set x 3; set b 2
    //   puts [expr $a + $b]     ;# 5   — `$a` substitutes to `$x`, then expr
    //                                    substitutes *that* to 3
    //   puts [expr {$a + $b}]   ;# error: `$a` is the string `$x`
    //
    // Bracing is the right advice and stays available as its own action; it
    // is not an unattended bulk rewrite.
    let src = "set a {$x}\nset x 3\nset b 2\nputs [expr $a + $b]\n";
    let classes = safety_for(src, "tcl9.0", DiagCode::W100);
    assert!(!classes.is_empty(), "expected a W100 fix for {src:?}");
    assert!(
        classes.iter().all(|s| *s == FixSafety::BehaviourHardening),
        "got {classes:?}"
    );
}

#[test]
fn tp_w100_brace_fix_over_a_substitution_free_operand_is_equivalent() {
    // Nothing substitutes in `abs(-2)`, so `expr` receives byte-identical
    // text either way — the delimiter repair the bulk pass exists for.  (A
    // pure-arithmetic `expr 1 + 2` never reaches here: the emitter already
    // treats a wholly literal arithmetic expression as safe and emits no
    // W100 at all.)
    let classes = safety_for("set n [expr abs(-2)]\n", "tcl9.0", DiagCode::W100);
    assert!(!classes.is_empty(), "expected a W100 fix");
    assert!(
        classes.iter().all(|s| s.is_bulk_applicable()),
        "got {classes:?}"
    );
}

#[test]
fn fp_w100_brace_fix_over_a_backslash_bearing_operand_is_not_equivalent() {
    // The outer parse decodes `\x41` before `expr` sees it; braced, `expr`
    // sees the four written characters instead.
    let classes = safety_for("set n [expr \\x41 + 1]\n", "tcl9.0", DiagCode::W100);
    if !classes.is_empty() {
        assert!(
            classes.iter().all(|s| !s.is_bulk_applicable()),
            "a backslash-bearing operand is not a byte-identical rewrite: {classes:?}"
        );
    }
}

// -- W110: numeric vs string comparison ----------------------------------

#[test]
fn fp_w110_eq_rewrite_is_never_equivalent() {
    // C Tcl 9.0.3: `expr {"1" == "01"}` is 1 (numeric), `expr {"1" eq "01"}`
    // is 0 (string).  The rewrite changes the answer in precisely the
    // coercion cases the diagnostic is about.
    let classes = safety_for("puts [expr {\"1\" == \"01\"}]\n", "tcl9.0", DiagCode::W110);
    assert!(!classes.is_empty(), "expected a W110 fix");
    assert!(
        classes.iter().all(|s| !s.is_bulk_applicable()),
        "got {classes:?}"
    );
}

// -- W105: unbraced code block -------------------------------------------

#[test]
fn fp_w105_brace_fix_over_a_substituted_body_is_not_equivalent() {
    let classes = safety_for("while 1 \"puts $x\"\n", "tcl9.0", DiagCode::W105);
    assert!(!classes.is_empty(), "expected a W105 fix");
    assert!(
        classes.iter().all(|s| !s.is_bulk_applicable()),
        "got {classes:?}"
    );
}

// -- W120 / W213 / W304: hardening, not equivalence ----------------------

#[test]
fn fp_w213_nocomplain_fix_is_hardening() {
    // `unset x` raises when `x` does not exist; `unset -nocomplain x` does
    // not.  Suppressing that error is the fix, and a `catch`-guarded probe
    // observes the change.
    let classes = safety_for("proc f {} {\n    unset x\n}\n", "tcl9.0", DiagCode::W213);
    assert!(!classes.is_empty(), "expected a W213 fix");
    assert!(
        classes.iter().all(|s| !s.is_bulk_applicable()),
        "got {classes:?}"
    );
}

#[test]
fn fp_w304_option_terminator_fix_is_hardening() {
    // `--` changes how the following word is read.  That is the point.
    let classes = safety_for("regexp $pattern $text\n", "tcl9.0", DiagCode::W304);
    assert!(!classes.is_empty(), "expected a W304 fix");
    assert!(
        classes.iter().all(|s| !s.is_bulk_applicable()),
        "got {classes:?}"
    );
}

// -- "Did you mean …?" suggestions ---------------------------------------

#[test]
fn fp_did_you_mean_suggestions_require_review() {
    // An edit-distance guess is a suggestion, never a proof.  W001's
    // subcommand suggestion is the representative case.
    let classes = safety_for("string lenght $x\n", "tcl9.0", DiagCode::W001);
    assert!(!classes.is_empty(), "expected a W001 suggestion fix");
    assert!(
        classes.iter().all(|s| *s == FixSafety::RequiresReview),
        "got {classes:?}"
    );
}

// -- TN: diagnostics with no fix -----------------------------------------

#[test]
fn tn_clean_source_carries_no_fixes_at_all() {
    assert!(fixes("set x 1\nputs $x\n", "tcl9.0").is_empty());
}

#[test]
fn tn_a_diagnostic_without_a_fix_contributes_nothing() {
    // E002 (too few arguments) has no surplus word to delete and carries no
    // fix, so it can never reach the bulk pass regardless of its class.
    let classes = safety_for("string\n", "tcl9.0", DiagCode::E002);
    assert!(classes.is_empty(), "got {classes:?}");
}

// -- FN: safe work still gets done ---------------------------------------

#[test]
fn fn_the_equivalent_class_is_actually_reachable() {
    // A guard against over-classification emptying the bulk pass entirely:
    // at least one real emitter must still produce a bulk-applicable fix,
    // or "Fix All Safe Issues" would be a no-op by construction.
    let classes = safety_for("set n [expr abs(-2)]\n", "tcl9.0", DiagCode::W100);
    assert!(
        classes.iter().any(|s| s.is_bulk_applicable()),
        "no emitter produces a bulk-applicable fix: {classes:?}"
    );
}

#[test]
fn every_fix_reports_a_label() {
    // The `applied` report names the class, so every variant needs one.
    for safety in [
        FixSafety::SemanticsEquivalent,
        FixSafety::BehaviourHardening,
        FixSafety::StyleOnly,
        FixSafety::RequiresReview,
    ] {
        assert!(!safety.as_str().is_empty());
    }
}
