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

//! W304 missing option terminators and W101 `eval` of a concatenated string.

use super::*;

// -- W304 missing-option-terminator emitter
//
// Resolution profile lives in ``tcl-registry``; tristate severity
// / two-diagnostic origin / code-fix logic lives in
// ``analyser/diagnostics.rs``.

fn w304_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W304)
        .collect()
}

#[test]
fn analyse_emits_w304_for_regexp_pattern_variable() {
    let diags = w304_diags("regexp $pattern $text\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
    assert!(
        diags[0].message.to_lowercase().contains("option-injection"),
        "got {:?}",
        diags[0].message
    );
    assert!(matches!(
        diags[0].severity,
        crate::analyser::Severity::Suggestion
    ));
}

#[test]
fn analyse_no_w304_for_regexp_safe_literal_pattern() {
    // Pattern starts with `(` — non-dynamic, doesn't start with
    // `-`, so the OFF gate suppresses regardless of command.
    let diags = w304_diags("regexp {(a+)+$} $text\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_regexp_literal_dash_pattern() {
    // ``regexp {-[0-9]+} $text`` — the literal pattern starts
    // with `-` so the positional scanner treats it as an
    // unknown option and lands on the next positional
    // (``$text``).  The diagnostic still fires; severity comes
    // from the dynamic-var INFO path because the diag anchors on
    // ``$text``, not the pattern literal.
    let diags = w304_diags("regexp {-[0-9]+} $text\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_exec_literal_dash_after_first_positional() {
    // ``exec foo -bad`` — ``first_positional_without_terminator``
    // treats ``foo`` (index 0) as the first positional, so the
    // OFF gate suppresses W304 there (non-dynamic, doesn't start
    // with ``-``).  The later literal ``-bad`` is not
    // re-considered as a candidate "first positional" argument,
    // so no diagnostic fires.
    //
    // This pins the scanner / first-positional behaviour for
    // ``exec`` rather than exercising the literal-dash WARN
    // branch.  The WARN branch is covered by
    // `analyse_emits_w304_for_regexp_literal_dash_pattern`
    // (literal pattern starting with `-`) and
    // `analyse_w304_constant_propagation_dash_value_warns`
    // (variable resolved via constant-prop to a `-`-prefixed
    // value).
    let diags = w304_diags("exec foo -bad\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_regexp_with_terminator() {
    let diags = w304_diags("regexp -- $pattern $text\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_regexp_with_option_value_then_variable() {
    // ``-start`` consumes the next arg as its value; the first
    // positional after it is the pattern variable.
    let diags = w304_diags("regexp -start 0 $pattern $text\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_regsub_variable() {
    let diags = w304_diags("regsub $pattern $text X out\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_subst() {
    // ``subst`` does not declare a ``--`` option — registry-
    // level filter suppresses W304 entirely.
    let diags = w304_diags("subst $template\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_exec_variable() {
    let diags = w304_diags("exec $cmd\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_exec_with_terminator() {
    let diags = w304_diags("exec -- $cmd\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_glob_safe_literal() {
    // ``*.tcl`` does not start with `-`; OFF gate suppresses.
    let diags = w304_diags("glob *.tcl\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_string_match() {
    // ``string match`` does not support ``--`` — registry filter.
    let diags = w304_diags("string match $pattern $value\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_lsearch() {
    // ``lsearch`` does not declare ``--`` either.
    let diags = w304_diags("lsearch -exact $domain c\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_file_delete_variable() {
    // ``file delete`` is subcommand-scoped — profile.scan_start
    // == 1 to skip the ``delete`` keyword.
    let diags = w304_diags("file delete $path\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w304_for_file_delete_with_terminator() {
    let diags = w304_diags("file delete -- $path\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w304_for_load_variable() {
    let diags = w304_diags("load $fileName\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_w304_constant_propagation_emits_info_with_origin() {
    // ``set X "datagroup"; exec $X`` — the variable resolves to a
    // literal value that doesn't start with `-`, so severity drops
    // to INFO and a second "origin" diagnostic anchors at the
    // literal's range.  (The two-arg braced ``switch $X { ... }``
    // form is exempt from W304 entirely — FP-NAB-05 — so an
    // option-bearing command that still flags is used here.)
    let src = "set totp_key_storage \"datagroup\"\n\
                   exec $totp_key_storage\n";
    let diags = w304_diags(src);
    assert_eq!(diags.len(), 2, "got {diags:?}");

    let main = diags
        .iter()
        .find(|d| !d.fixes.is_empty())
        .expect("main diag");
    let origin = diags
        .iter()
        .find(|d| d.fixes.is_empty())
        .expect("origin diag");

    assert!(
        matches!(main.severity, crate::analyser::Severity::Suggestion),
        "main severity {:?}",
        main.severity
    );
    assert!(
        main.message.contains("totp_key_storage") && main.message.contains("datagroup"),
        "main message {:?}",
        main.message
    );

    let highlighted = &src[main.span.start() as usize..main.span.end() as usize];
    assert_eq!(highlighted, "$totp_key_storage", "got {highlighted:?}");

    // The origin diag points at the ``"datagroup"`` literal in
    // the preceding ``set``.
    let origin_text = &src[origin.span.start() as usize..origin.span.end() as usize];
    assert!(
        origin_text.contains("datagroup"),
        "origin span text {origin_text:?}"
    );
}

#[test]
fn analyse_w304_constant_propagation_dash_value_warns() {
    // The variable resolves to ``-something`` — escalates to
    // WARNING.
    let src = "set evil \"-rf\"\n\
                   exec $evil /\n";
    let diags = w304_diags(src);
    assert!(!diags.is_empty(), "got {diags:?}");
    let main = diags
        .iter()
        .find(|d| !d.fixes.is_empty())
        .expect("main diag");
    assert!(matches!(main.severity, crate::analyser::Severity::Warning));
}

// -- W101 eval-string-concat emitter
//
// Canonical-list-idiom suppression lives in the registry
// (``is_canonical_list_command``); substitution-detection
// approximation lives in the analyser.

fn w101_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::W101)
        .collect()
}

#[test]
fn analyse_emits_w101_for_eval_with_variable() {
    let diags = w101_diags("eval \"puts $x\"\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
    assert!(
        diags[0].message.to_lowercase().contains("injection"),
        "got {:?}",
        diags[0].message
    );
    assert!(matches!(
        diags[0].severity,
        crate::analyser::Severity::Warning
    ));
}

#[test]
fn analyse_no_w101_for_eval_braced_script() {
    let diags = w101_diags("eval {puts hello}\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_multiple_braced() {
    let diags = w101_diags("eval {set x 1} {puts $x}\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w101_for_eval_with_command_subst() {
    // ``eval [build_cmd]`` — single CMD token, but `build_cmd`
    // isn't a canonical-list-producing command.
    let diags = w101_diags("eval [build_cmd]\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_literal_no_substitution() {
    // ``eval puts hello`` — both args are bare literals; no
    // substitution at any level.
    let diags = w101_diags("eval puts hello\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_list_idiom() {
    // ``eval [list ...]`` — ``list`` produces a canonical list,
    // safe re-parse.
    let diags = w101_diags("eval [list set $varname $value]\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_linsert_idiom() {
    // ``linsert`` returns TclType::List → canonical.
    let diags = w101_diags("eval [linsert $cmdlist 0 extraarg]\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_split_idiom() {
    let diags = w101_diags("eval [split $line :]\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_emits_w101_for_eval_concat_idiom() {
    // ``concat`` is the explicit non-canonical exclusion —
    // strips one level of grouping, not safe for re-parse.
    let diags = w101_diags("eval [concat $script $args]\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_non_eval_commands() {
    // The emitter is gated on ``cmd_name == "eval"`` — other
    // substitution-bearing commands are handled elsewhere (W301
    // covers uplevel; W312 covers interp eval).
    let diags = w101_diags("uplevel 1 \"puts $x\"\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_w101_anchors_at_first_arg_token() {
    let src = "eval \"puts $x\"\n";
    let diags = w101_diags(src);
    assert_eq!(diags.len(), 1);
    let span = diags[0].span;
    let text = &src[span.start() as usize..span.end() as usize];
    // First arg is the quoted string ``"puts $x"`` — the
    // representative token's span anchors the diagnostic.
    assert!(text.contains("puts") || text.contains("$x"), "got {text:?}");
}

#[test]
fn analyse_w101_rejects_multi_command_subscript() {
    // ``[list a; set x $user]`` — multi-command script can't be
    // proven safe (last command's result wins, and that's
    // ``set``, not ``list``).
    let diags = w101_diags("eval [list a\\; set x $user]\n");
    assert_eq!(diags.len(), 1, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_literal_multi_token_word() {
    // ``eval foo{bar}`` is a multi-token word (Esc + Str joined,
    // ``single_token_word == false``) that contains no Var/Cmd
    // substitution — W101 only fires on actual VAR/CMD tokens, so
    // this must not trigger it.  The check is a brace/backslash-aware
    // source-byte scan that looks for unescaped ``$`` / ``[``
    // outside ``{...}``.
    let diags = w101_diags("eval foo{bar}\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_no_w101_for_eval_backslash_escaped_dollar() {
    // ``eval "no\$x"`` — the ``\$`` is a backslash-escape, so
    // the lexer produces a single ESC token with no Var.  The
    // word-span scan must skip the next byte after ``\`` to
    // avoid mis-detecting the literal ``$``.
    let diags = w101_diags("eval no\\$x\n");
    assert_eq!(diags.len(), 0, "got {diags:?}");
}

#[test]
fn analyse_w304_code_fix_inserts_terminator() {
    let src = "exec $cmd\n";
    let diags = w304_diags(src);
    assert_eq!(diags.len(), 1, "got {diags:?}");
    let fix = diags[0]
        .fixes
        .first()
        .expect("expected an insert-terminator fix");
    let mut applied = src.to_string();
    let start = fix.span.start() as usize;
    let end = fix.span.end() as usize;
    applied.replace_range(start..end, &fix.new_text);
    assert_eq!(applied, "exec -- $cmd\n", "got {applied:?}");
}
