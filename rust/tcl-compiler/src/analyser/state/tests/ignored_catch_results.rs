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

//! W302: `catch` without a result variable, and where its fix inserts one.

use super::*;

// -- W302 catch-without-result-var emitter

#[test]
fn analyse_emits_w302_for_catch_without_result_var() {
    let mut a = Analyser::new();
    let r = a.analyse("catch { puts hi }\n", "tcl");
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1, "got {:?}", r.diagnostics);
    assert!(
        w302[0].message.contains("silently swallows errors"),
        "got {:?}",
        w302[0].message
    );
    assert!(matches!(w302[0].severity, crate::analyser::Severity::Hint));
}

#[test]
fn analyse_no_w302_when_catch_has_result_var() {
    // ``catch BODY result`` — result variable is present, so
    // errors aren't silently swallowed.
    let mut a = Analyser::new();
    let r = a.analyse("catch { puts hi } result\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w302_when_catch_has_options_var() {
    // ``catch BODY result options`` — both optional vars
    // present.  Still no W302.
    let mut a = Analyser::new();
    let r = a.analyse("catch { puts hi } result options\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w302_for_multi_token_catch_body() {
    // ``catch pre$x`` — a multi-token-word body is a dynamic
    // body (not a single-token word), so it is not treated as a
    // braced catch body and W302 never fires.  The emitter gates
    // on the body being a single-token word.
    let mut a = Analyser::new();
    let r = a.analyse("catch pre$x\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W302),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w302_anchors_at_catch_keyword() {
    // W302 highlights just the ``catch`` command token — the
    // narrowest span that identifies the issue — rather than the
    // whole ``catch {…}`` statement
    // (which also dropped the closing brace under the lexer's
    // inner-end convention).
    let mut a = Analyser::new();
    let src = "catch { puts hi }\n";
    let r = a.analyse(src, "tcl");
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1);
    let span = w302[0].span;
    let text = &src[span.start() as usize..span.end() as usize];
    assert_eq!(text, "catch", "W302 should span only the catch keyword");
}

// -- W302 quick-fix insertion anchor
//
// The diagnostic anchors at the `catch` keyword, so the fix must carry
// its **own** span: the point past the *body's* closing delimiter.  Each
// test applies the fix to the source and asserts the resulting text, so a
// wrong anchor cannot pass by matching only the inserted string.

/// Apply W302's `nth` fix to `src` and return the rewritten source.
fn apply_w302_fix(src: &str, nth: usize, dialect: &str) -> String {
    let mut a = Analyser::new();
    let r = a.analyse(src, dialect);
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1, "expected one W302, got {:?}", r.diagnostics);
    let fix = w302[0]
        .fixes
        .get(nth)
        .unwrap_or_else(|| panic!("no fix {nth}: {:?}", w302[0].fixes));
    let mut out = src.to_string();
    out.replace_range(
        fix.span.start() as usize..fix.span.end() as usize,
        &fix.new_text,
    );
    out
}

#[test]
fn w302_result_fix_inserts_after_the_body() {
    // The issue's reproducer.  Anchoring at the diagnostic's end would
    // produce `catch result {error oops}`, which C Tcl reads as a catch
    // of the script `result` — completion code 1, result
    // `invalid command name "result"`.
    assert_eq!(
        apply_w302_fix("catch {error oops}\n", 0, "tcl9.0"),
        "catch {error oops} result\n"
    );
}

#[test]
fn w302_options_fix_inserts_after_the_body() {
    assert_eq!(
        apply_w302_fix("catch {error oops}\n", 1, "tcl9.0"),
        "catch {error oops} result options\n"
    );
}

#[test]
fn w302_fix_handles_a_multiline_body() {
    let src = "catch {\n    error oops\n}\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {\n    error oops\n} result\n"
    );
}

#[test]
fn w302_fix_handles_a_trailing_comment() {
    let src = "catch {error oops} ;# ignore\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {error oops} result ;# ignore\n"
    );
}

#[test]
fn w302_fix_handles_a_semicolon_separated_command() {
    let src = "catch {error oops}; puts done\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {error oops} result; puts done\n"
    );
}

#[test]
fn w302_fix_handles_nested_substitutions_in_the_body() {
    let src = "catch {puts [expr {1 + [foo $x]}]}\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "catch {puts [expr {1 + [foo $x]}]} result\n"
    );
}

#[test]
fn w302_fix_does_not_overshoot_an_empty_body() {
    // `catch {}`'s body span already covers its own closer, so a naive
    // `span.end() + 1` anchor would insert one byte past the brace.
    let src = "catch {}\n";
    assert_eq!(apply_w302_fix(src, 0, "tcl9.0"), "catch {} result\n");
}

#[test]
fn w302_fix_handles_a_leading_qualified_head() {
    // `::catch` resolves to the same registry spec (and the same
    // `AnalyserHookId::Catch`), so it is diagnosed and fixed identically.
    let src = "::catch {error oops}\n";
    assert_eq!(
        apply_w302_fix(src, 0, "tcl9.0"),
        "::catch {error oops} result\n"
    );
}

#[test]
fn w302_offers_only_the_result_variable_under_tcl84() {
    // Tcl 8.4's `catch script ?varName?` has no options-dictionary
    // argument, so the second fix must not be offered there — the count
    // comes from the dialect-applicable synopsis, not from a constant.
    let mut a = Analyser::new();
    let r = a.analyse("catch {error oops}\n", "tcl8.4");
    let w302: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W302)
        .collect();
    assert_eq!(w302.len(), 1, "got {:?}", r.diagnostics);
    assert_eq!(
        w302[0].fixes.len(),
        1,
        "8.4 documents one optional word: {:?}",
        w302[0].fixes
    );
    assert_eq!(w302[0].fixes[0].new_text, " var");
}

#[test]
fn w302_fixes_are_never_bulk_applicable() {
    // Capturing the result writes a new variable in the caller's frame,
    // which a program already using that name observes — so the fix is
    // hardening, not a semantics-preserving rewrite.
    let mut a = Analyser::new();
    let r = a.analyse("catch {error oops}\n", "tcl9.0");
    let w302 = r
        .diagnostics
        .iter()
        .find(|d| d.code == DiagCode::W302)
        .expect("W302");
    assert_ne!(w302.fixes, [] as [crate::irules_checks::CodeFix; 0]);
    assert!(
        w302.fixes.iter().all(|f| !f.safety.is_bulk_applicable()),
        "got {:?}",
        w302.fixes
    );
}

#[test]
fn w302_carries_no_fix_for_a_malformed_catch() {
    // An unterminated body never reaches a well-formed insertion point;
    // whatever the recovery path emits, no W302 fix may point outside
    // the buffer.
    let mut a = Analyser::new();
    let src = "catch {error oops\n";
    let r = a.analyse(src, "tcl9.0");
    for diag in r.diagnostics.iter().filter(|d| d.code == DiagCode::W302) {
        for fix in &diag.fixes {
            assert!(
                fix.span.end() as usize <= src.len(),
                "fix span {:?} outside the buffer",
                fix.span
            );
        }
    }
}
