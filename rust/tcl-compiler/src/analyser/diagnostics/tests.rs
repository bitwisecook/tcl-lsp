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

use super::*;
use crate::analyser::types::Diagnostic;
use tcl_core_types::DiagCode;
use tcl_lexer::Span;

mod const_dispatch;
mod dataflow;
mod definition_warnings;
mod dialect_profiles;
mod namespace_context;
mod original_source_subjects;
mod pipeline;
mod proven;
mod security;
mod unresolved;
mod usage;
mod validity;
mod var_command;
mod walked_bodies;

fn code_sevs(src: &str, code: &str) -> Vec<String> {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .map(|d| format!("{:?}", d.severity))
        .collect()
}

fn has_code(src: &str, dialect: &str, code: &str) -> bool {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .any(|d| d.code.as_str() == code)
}

fn codes_for_dialect(src: &str, dialect: &str) -> Vec<String> {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

fn w216_count(src: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W216)
        .count()
}

fn diag(code: DiagCode, span: Span, msg: &str) -> Diagnostic {
    crate::analyser::types::Diagnostic::new(code, span, msg.to_string(), Severity::Warning)
}

/// Every W003 diagnostic for `src` analysed under `dialect`, alongside
/// the exact source substring its span covers — the tight-highlight
/// assertions below check that text directly rather than trusting
/// hand-computed byte offsets.
fn w003_hits(src: &str, dialect: &str) -> Vec<(String, Diagnostic)> {
    let mut a = Analyser::new();
    let result = a.analyse(src, dialect);
    result
        .diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W003)
        .map(|d| {
            let text = src[d.span.start() as usize..d.span.end() as usize].to_string();
            (text, d)
        })
        .collect()
}

/// `source` as the body of a procedure, which the per-item walk analyses on
/// its own.
fn in_proc(source: &str) -> String {
    format!("proc p {{}} {{\n{source}\n}}\n")
}

// A `try` handler `-` fallthrough body.
//
// A `try` `on`/`trap` handler body may be a bare `-` to share the *next*
// handler's body (the same fallthrough mechanism `switch` uses for pattern
// bodies). Treating every handler body as a script compiles the solo `-` to a
// zero-argument call of the `-` command and trips a spurious E002 ("Too few
// arguments for '-'"). These tests assert both sides: the fallthrough `-`
// raises no E002, and a genuine zero-arg `-` command is still flagged.

fn count_code(src: &str, code: &str) -> usize {
    count_code_in(src, code, "tcl8.6")
}

/// [`count_code`] under an explicit dialect, for a check whose whole point is
/// which release the option belongs to.
fn count_code_in(src: &str, code: &str, dialect: &str) -> usize {
    let mut a = crate::analyser::Analyser::new();
    a.analyse(src, dialect)
        .diagnostics
        .iter()
        .filter(|d| d.code.as_str() == code)
        .count()
}

/// tclpkg manifest whole-file scoped environment (`tcl_registry::scoped::
/// file_scope_env` keyed on the `tclpkg.tcl` basename): directives resolve
/// against the environment, never against same-named Tcl/Tk commands.
mod tclpkg_manifest_env {
    use super::*;

    const MANIFEST: &str = "\
package     demo-app
version     0.1.0
description \"A demo\"
license     MIT
author      \"Dev <dev@example.org>\"

tcl >=8.6

require json    1.0.0
require http    2.9.0

dev-require tcltest 2.5.0

provides demo::serve
entry    main.tcl
";

    fn diags_for(path: Option<&str>) -> Vec<Diagnostic> {
        let mut analyser =
            crate::analyser::Analyser::new().with_file_path(path.map(str::to_string));
        analyser.analyse(MANIFEST, "tcl8.6").diagnostics
    }

    /// TN: every directive resolves in the manifest environment — no
    /// unknown-command / unknown-subcommand / missing-package noise.
    #[test]
    fn manifest_directives_resolve_cleanly() {
        let diags = diags_for(Some("/proj/tclpkg.tcl"));
        assert_eq!(
            diags.len(),
            0,
            "a valid manifest must produce no diagnostics: {diags:?}"
        );
    }

    /// TP control: the same text WITHOUT the manifest file name is plain
    /// Tcl — the directives draw their usual diagnostics (the environment
    /// must never leak into ordinary documents).
    #[test]
    fn plain_tcl_document_still_flags_directives() {
        let diags = diags_for(Some("/proj/other.tcl"));
        assert!(
            diags.iter().any(|d| d.code == DiagCode::W123),
            "outside a manifest the directives are unknown commands: {diags:?}"
        );
    }

    /// TP: a typo'd directive still draws W123 — the environment is closed.
    #[test]
    fn manifest_typo_directive_fires_w123() {
        let mut analyser =
            crate::analyser::Analyser::new().with_file_path(Some("/proj/tclpkg.tcl".to_string()));
        let result = analyser.analyse("package demo\nverison 0.1.0\n", "tcl8.6");
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == DiagCode::W123 && d.message.contains("verison")),
            "a typo'd directive must stay unknown: {:?}",
            result.diagnostics
        );
    }

    /// TP: a directive arity error is checked against the manifest
    /// grammar (`require <name> <minimum> ?-source <url>?`).
    #[test]
    fn manifest_directive_arity_checked() {
        let mut analyser =
            crate::analyser::Analyser::new().with_file_path(Some("/proj/tclpkg.tcl".to_string()));
        let result = analyser.analyse("package demo\nrequire json\n", "tcl8.6");
        assert!(
            result.diagnostics.iter().any(|d| d.code == DiagCode::E002),
            "`require` with one argument is below the directive's minimum: {:?}",
            result.diagnostics
        );
    }
}

mod did_you_mean_variables {
    //! W210 / W212 / W215 "did you mean…?" — the undefined-variable
    //! family suggests a close in-scope variable name (W215's own tests
    //! live with the emitter in `analyser::scope`).

    use super::*;

    fn find_code(src: &str, dialect: &str, code: DiagCode) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, dialect);
        r.diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap_or_else(|| panic!("expected {code:?} in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn w210_suggests_close_defined_variable() {
        // TP: `countr` is an edit-distance-1 typo of the defined `counter`.
        let d = find_code("set counter 1\nputs $countr\n", "tcl8.6", DiagCode::W210);
        assert!(
            d.message.ends_with("; did you mean 'counter'?"),
            "expected a 'counter' suggestion: {:?}",
            d.message
        );
        // The suggestion is informational only — no fix (the analyser
        // cannot know which spelling was intended).
        assert_eq!(d.fixes.len(), 0, "{:?}", d.fixes);
    }

    #[test]
    fn w210_suggests_inside_proc_bodies() {
        let d = find_code(
            "proc p {} {\n  set counter 1\n  puts $countr\n}\n",
            "tcl8.6",
            DiagCode::W210,
        );
        assert!(
            d.message.ends_with("; did you mean 'counter'?"),
            "expected a 'counter' suggestion: {:?}",
            d.message
        );
    }

    #[test]
    fn w210_case_mismatch_still_wins_over_edit_distance() {
        // `MYLIST` is 6 edits from `mylist` — far outside the scaled
        // budget — but the case-insensitive twin rule still names it.
        let d = find_code("set MYLIST 1\nputs $mylist\n", "tcl8.6", DiagCode::W210);
        assert!(
            d.message.ends_with("; did you mean 'MYLIST'?"),
            "expected the case twin: {:?}",
            d.message
        );
    }

    #[test]
    fn w210_no_suggestion_when_nothing_is_close() {
        // FP guard: `frobnicate` is nowhere near `totally`.
        let d = find_code(
            "set totally 1\nputs $frobnicate\n",
            "tcl8.6",
            DiagCode::W210,
        );
        assert!(
            !d.message.contains("did you mean"),
            "no suggestion expected: {:?}",
            d.message
        );
    }

    #[test]
    fn w210_short_name_never_fishes_a_full_replacement() {
        // FP guard: every other 1-char name is a full rewrite of `$u`,
        // not a typo correction — `m` must not be suggested.
        let d = find_code("set m 1\nputs $u\nputs $m\n", "tcl8.6", DiagCode::W210);
        assert!(
            !d.message.contains("did you mean"),
            "no suggestion expected for a 1-char read: {:?}",
            d.message
        );
    }

    #[test]
    fn w212_suggests_close_in_scope_variable_for_undefined_name() {
        // TP: `set $countr` where `countr` is undefined but `counter`
        // is in scope — the better correction is `counter`.
        let d = find_code("set counter 1\nset $countr 5\n", "tcl8.6", DiagCode::W212);
        assert!(
            d.message.contains("Did you mean 'counter'?"),
            "expected the close in-scope name: {:?}",
            d.message
        );
    }

    #[test]
    fn w212_keeps_de_sigil_suggestion_for_defined_name() {
        // `varname` is itself defined — dropping the `$` is the fix.
        let d = find_code("set varname x\nset $varname y\n", "tcl8.6", DiagCode::W212);
        assert!(
            d.message.contains("Did you mean 'varname'?"),
            "expected the de-sigiled name: {:?}",
            d.message
        );
    }

    #[test]
    fn w212_keeps_de_sigil_suggestion_when_nothing_is_close() {
        // FP guard: no in-scope name is close — the de-sigil suggestion
        // stands rather than fishing an unrelated variable.
        let d = find_code(
            "set totally 1\nincr $frobnicate\n",
            "tcl8.6",
            DiagCode::W212,
        );
        assert!(
            d.message.contains("Did you mean 'frobnicate'?"),
            "expected the de-sigiled name: {:?}",
            d.message
        );
    }
}

mod irule2002_drop_in_fix {
    //! IRULE2002 — a deprecated command whose registry spec marks the
    //! replacement as a drop-in rename carries a head-swap quick fix;
    //! non-mechanical replacements stay message-only.

    use super::*;

    fn irule2002_for(src: &str) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "f5-irules");
        r.diagnostics
            .iter()
            .find(|d| d.code == DiagCode::Irule2002)
            .unwrap_or_else(|| panic!("expected IRULE2002 in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn drop_in_replacement_carries_head_swap_fix() {
        // TP: `client_addr` → `IP::client_addr` is argument-compatible.
        let d = irule2002_for("when HTTP_REQUEST {\n  set a [client_addr]\n}\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        let fix = &d.fixes[0];
        assert_eq!(fix.new_text, "IP::client_addr");
        assert_eq!(fix.description, "Replace with 'IP::client_addr'");
        // The fix replaces exactly the command head token.
        assert_eq!(fix.span, d.span);
    }

    #[test]
    fn drop_in_fix_preserves_arguments() {
        // TP: an argument-taking drop-in (`decode_uri <s>` →
        // `URI::decode <s>`) swaps only the head; the argument words are
        // untouched (outside the fix span).
        let d = irule2002_for("when HTTP_REQUEST {\n  set u [decode_uri $raw]\n}\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "URI::decode");
        assert_eq!(d.fixes[0].span, d.span);
    }

    #[test]
    fn non_mechanical_replacements_get_no_fix() {
        // FP guards: `ip_addr` → `IP::addr` restructures its arguments
        // (`… mask …`); `matchclass` → `class` needs the `match`
        // subcommand (IRULE2001 carries the correct arity-aware fix);
        // `use` → `virtual` changes the statement shape entirely.
        for src in [
            "when HTTP_REQUEST {\n  set m [ip_addr 10.0.0.1 255.0.0.0]\n}\n",
            "when HTTP_REQUEST {\n  matchclass $u ::lib\n}\n",
            "when HTTP_REQUEST {\n  use pool aol_pool\n}\n",
        ] {
            let d = irule2002_for(src);
            assert_eq!(
                d.fixes.len(),
                0,
                "no fix expected for {src:?}: {:?}",
                d.fixes
            );
        }
    }
}

mod arity_usage_suffix {
    //! E002 / E003 / E005 — arity messages append the registry
    //! signature as a " — usage: …" suffix when the resolved spec
    //! declares a synopsis.

    use super::*;

    fn find_code(src: &str, code: DiagCode) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        r.diagnostics
            .iter()
            .find(|d| d.code == code)
            .unwrap_or_else(|| panic!("expected {code:?} in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn e002_appends_command_synopsis() {
        let d = find_code("lreplace\n", DiagCode::E002);
        assert_eq!(
            d.message,
            "Too few arguments for 'lreplace': expected at least 3, got 0 \
             — usage: lreplace list first last ?element element ...?"
        );
    }

    #[test]
    fn e003_appends_subcommand_synopsis() {
        // The subcommand path uses the resolved subcommand's synopsis,
        // not the parent ensemble's.
        let d = find_code("string length a b c\n", DiagCode::E003);
        assert!(
            d.message.ends_with(" — usage: string length string"),
            "expected the subcommand synopsis: {:?}",
            d.message
        );
    }

    #[test]
    fn e005_appends_command_synopsis() {
        let d = find_code("lmap a b c d\n", DiagCode::E005);
        assert!(
            d.message
                .ends_with(" — usage: lmap varlist1 list1 ?varlist2 list2 ...? body"),
            "expected the lmap synopsis: {:?}",
            d.message
        );
    }

    #[test]
    fn user_proc_arity_message_stays_count_only() {
        // A same-file proc has no registry synopsis — the message keeps
        // its count-only form.
        let d = find_code("proc pair {a b} {}\npair 1\n", DiagCode::E002);
        assert!(
            !d.message.contains("usage:"),
            "no usage suffix expected for a user proc: {:?}",
            d.message
        );
    }
}

mod w104_lappend_fix {
    //! W104 — the two-word leading-space `append` shape carries a
    //! whole-command `lappend` rewrite; every non-mechanical shape
    //! stays message-only.

    use super::*;

    fn w104_for(src: &str) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        r.diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W104)
            .unwrap_or_else(|| panic!("expected W104 in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn leading_space_var_piece_carries_whole_command_rewrite() {
        let src = "set item x\nset items {}\nappend items \" $item\"\n";
        let d = w104_for(src);
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        let fix = &d.fixes[0];
        assert_eq!(fix.new_text, "lappend items $item");
        assert_eq!(fix.description, "Rewrite with `lappend`");
        // The fix replaces exactly the whole `append` command.
        assert_eq!(
            &src[fix.span.start() as usize..fix.span.end() as usize],
            "append items \" $item\"",
        );
    }

    #[test]
    fn literal_piece_and_array_target_also_rewrite() {
        let d = w104_for("append out(k) \" item\"\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "lappend out(k) item");
    }

    #[test]
    fn trailing_space_shape_gets_no_fix() {
        // `append msg "item "` puts the separator *after* the piece;
        // `lappend` would move it before — not equivalent.
        let d = w104_for("append msg \"item \"\n");
        assert_eq!(d.fixes.len(), 0, "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn non_mechanical_shapes_get_no_fix() {
        for src in [
            // Several pieces in one value word.
            "set a 1\nset b 2\nappend out \" $a $b\"\n",
            // Extra pad spaces.
            "set a 1\nappend out \"  $a\"\n",
            // Several value words.
            "set a 1\nappend out \" $a\" tail\n",
            // Braced value — a deliberate literal separator.
            "append banner { }\n",
            // Piece with a word/list metacharacter.
            "set a 1\nappend out \" [list $a]\"\n",
            // Pad-only value.
            "append out \" \"\n",
        ] {
            let d = w104_for(src);
            assert_eq!(
                d.fixes.len(),
                0,
                "no fix expected for {src:?}: {:?}",
                d.fixes
            );
        }
    }
}

mod w114_unwrap_fix {
    //! W114 — a redundant nested `[expr {…}]` inside a braced outer
    //! expression carries an unwrap fix; unbraced shapes and
    //! string-comparison contexts stay message-only.

    use super::*;

    fn w114_for(src: &str) -> Diagnostic {
        let mut a = crate::analyser::Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        r.diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W114)
            .unwrap_or_else(|| panic!("expected W114 in {:?}", r.diagnostics))
            .clone()
    }

    #[test]
    fn compound_inner_body_is_parenthesised() {
        let src = "set a 1\nset b 3\nif {$a && [expr {$b + 1}]} { puts hi }\n";
        let d = w114_for(src);
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        let fix = &d.fixes[0];
        assert_eq!(fix.new_text, "($b + 1)");
        assert_eq!(fix.description, "Unwrap the nested `expr`");
        // The fix replaces exactly the nested `[expr {…}]` — the
        // diagnostic's own span.
        assert_eq!(fix.span, d.span);
        assert_eq!(
            &src[fix.span.start() as usize..fix.span.end() as usize],
            "[expr {$b + 1}]",
        );
    }

    #[test]
    fn atom_inner_body_drops_the_parentheses() {
        let src = "set x 2\nset y [expr {[expr {$x}] + 1}]\n";
        let d = w114_for(src);
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "$x");
        assert_eq!(
            &src[d.fixes[0].span.start() as usize..d.fixes[0].span.end() as usize],
            "[expr {$x}]",
        );
    }

    #[test]
    fn literal_atom_also_drops_the_parentheses() {
        let d = w114_for("set y [expr {[expr {42}] + 1}]\n");
        assert_eq!(d.fixes.len(), 1, "expected one fix, got {:?}", d.fixes);
        assert_eq!(d.fixes[0].new_text, "42");
    }

    #[test]
    fn unbraced_inner_body_gets_no_fix() {
        // Inlining an unbraced body re-exposes it to substitution.
        let d = w114_for("set x 1\nif {[expr $x + 1] > 0} {}\n");
        assert_eq!(d.fixes.len(), 0, "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn unbraced_outer_argument_gets_no_fix() {
        // `if [expr {$x}] …` — the whole argument is the nested call;
        // unwrapping to `($x)` would change the substitution pipeline.
        let d = w114_for("set x 1\nif [expr {$x}] {}\n");
        assert_eq!(d.fixes.len(), 0, "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn string_comparison_context_gets_no_fix() {
        // `[expr {$s}]` normalises "007" to 7; `($s) eq "007"` would
        // not — the unwrap could flip the verdict, so no fix.
        let d = w114_for("set s 007\nif {[expr {$s}] eq \"007\"} {}\n");
        assert_eq!(d.fixes.len(), 0, "no fix expected: {:?}", d.fixes);
    }

    /// TIP 461's `lt`/`le`/`gt`/`ge` share the exact same numeric-
    /// normalisation risk as `eq`/`ne`.  A hand-typed guard naming only
    /// `eq`/`ne`/`in`/`ni` misses these four entirely — a live safety gap in
    /// an *automatic* code fix, not just a cosmetic one.
    #[test]
    fn tip461_string_ordering_context_gets_no_fix() {
        let d = w114_for("set s 007\nif {[expr {$s}] lt \"010\"} {}\n");
        assert_eq!(d.fixes.len(), 0, "no fix expected: {:?}", d.fixes);
    }

    #[test]
    fn multi_group_expr_arguments_get_no_fix() {
        // `[expr {a} {b}]` concatenates its arguments — not one braced
        // group, so no textual inline.
        let d = w114_for("set a 1\nif {[expr {$a} {+ 1}] > 0} {}\n");
        assert_eq!(d.fixes.len(), 0, "no fix expected: {:?}", d.fixes);
    }
}

/// A recovered command's selector words are checked against the registry
/// like its count. tclsh 8.5.19 rejects `string lenght abc` (`bad option`),
/// `string l abc` (`ambiguous option "l"`), `lsort -stride 2 {a b}` (`bad
/// option "-stride"`) and `lsort -in {a b}` (`ambiguous option "-in"`);
/// tclsh 8.6.18 rejects the same except `lsort -stride 2 {a b}`, which
/// returns `a b`.
#[test]
fn a_recovered_tail_draws_registry_selector_errors() {
    for (tail, tcl85, tcl86) in [
        ("string lenght abc", vec!["W001"], vec!["W001"]),
        ("string l abc", vec!["W145"], vec!["W145"]),
        ("lsort -stride 2 {a b}", vec!["W004"], vec![]),
        ("lsort -in {a b}", vec!["W145"], vec!["W145"]),
        ("string length abc", vec![], vec![]),
    ] {
        let src = format!("set x [foo bar\n{tail}\n");
        let cut = src.find('\n').unwrap();
        for (dialect, expected) in [("tcl8.5", &tcl85), ("tcl8.6", &tcl86)] {
            let codes: Vec<String> = Analyser::new()
                .analyse(&src, dialect)
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.span.start() as usize > cut)
                .map(|diagnostic| diagnostic.code.to_string())
                .collect();
            assert_eq!(&codes, expected, "{dialect}: {tail}");
        }
    }
}

/// The commands recovered after a syntax cut are an editor tier: the registry
/// reports its count errors over the written words of a literal bareword
/// head the reached prefix does not rebind, and nothing else is reported
/// there. tclsh 8.4.20, 8.6.18 and 9.0.4 raise `wrong # args: should be "set
/// varName ?newValue?"` for a bare `set` and a `wrong # args` naming the
/// missing subcommand for a bare `string`.
#[test]
fn a_recovered_tail_draws_registry_count_errors_and_nothing_else() {
    let tail_codes = |src: &str, dialect: &str| -> Vec<String> {
        let cut = src.find('\n').unwrap();
        Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.span.start() as usize > cut)
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    };
    for (src, expected) in [
        ("set x [foo bar\nset\n", vec!["E002"]),
        ("set x [foo bar\nstring\n", vec!["E001"]),
        ("set x [foo bar\nputs a b c d\n", vec!["E003"]),
        ("set x [foo bar\nset y 1\n", vec![]),
        // No flow, existence or unknown-command diagnostic in the tail.
        ("set x [foo bar\nputs $nope\n", vec![]),
        ("set x [foo bar\nzz9unknowncmd a\n", vec![]),
        // A computed or qualified head is not checked.
        ("set x [foo bar\n$cmd\n", vec![]),
        ("set x [foo bar\n::set\n", vec![]),
    ] {
        for dialect in ["tcl8.6", "tcl"] {
            assert_eq!(tail_codes(src, dialect), expected, "{dialect}: {src:?}");
        }
    }
    // A procedure the reached prefix declares once and nothing rebinds is
    // checked against its own parameter list, under a bare or a qualified
    // head. tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0 raise `wrong # args:
    // should be "my_helper x"` for `my_helper` and for `my_helper 1 2`, and
    // `wrong # args: should be "::myns::helper x"` for `::myns::helper`.
    let declared = |src: &str, dialect: &str| -> Vec<String> {
        let cut = src.find("set q {").unwrap();
        Analyser::new()
            .analyse(src, dialect)
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.span.start() as usize > cut)
            .filter(|diagnostic| diagnostic.code.is_error())
            .filter(|diagnostic| !diagnostic.code.to_string().starts_with("E20"))
            .map(|diagnostic| diagnostic.code.to_string())
            .collect()
    };
    let helper = "proc my_helper {x} {puts $x}\n";
    let namespaced = "namespace eval myns {\n  proc helper {x} {return $x}\n}\n";
    for (prefix, tail, expected) in [
        (helper, "my_helper\n", vec!["E002"]),
        (helper, "my_helper 1 2\n", vec!["E003"]),
        (helper, "my_helper 1\n", vec![]),
        (namespaced, "::myns::helper\n", vec!["E002"]),
        (namespaced, "myns::helper\n", vec!["E002"]),
        (namespaced, "::myns::helper 1\n", vec![]),
        // Declared twice, renamed, or with a computed parameter list: no
        // single declaration decides the count.
        (
            "proc my_helper {x} {}\nproc my_helper {x y} {}\n",
            "my_helper\n",
            vec![],
        ),
        (
            "proc my_helper {x} {}\nrename my_helper other\n",
            "my_helper\n",
            vec![],
        ),
        ("proc my_helper $params {}\n", "my_helper\n", vec![]),
    ] {
        let src = format!("{prefix}\nset q {{\n  aaa\n{tail}");
        // The lenient `tcl` environment states no formal count for a
        // declaration (its arity is the abstaining `0..unlimited`), so only
        // pinned releases are asked.
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(declared(&src, dialect), expected, "{dialect}: {src:?}");
        }
    }
    // A head the reached prefix rebinds is not the registry's to check.
    let rebound = "proc set {args} {}\nset x [foo bar\nset\n";
    let cut = rebound.rfind("set\n").unwrap();
    for dialect in ["tcl8.6", "tcl"] {
        assert!(
            Analyser::new()
                .analyse(rebound, dialect)
                .diagnostics
                .iter()
                .all(|diagnostic| (diagnostic.span.start() as usize) < cut),
            "{dialect}"
        );
    }
}
