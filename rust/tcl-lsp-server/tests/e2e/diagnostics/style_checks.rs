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

//! The style checks that must stay silent on idiomatic code: indirect array names, overridden library procs, single-variable bodies and regex end anchors.

use super::*;

// FP-STY-12: `${var}(idx)` in a varname position is the indirect-array-element
// idiom (`var` holds the array name), not a broken `$var(idx)` — so neither
// W216 nor W212 fire through the server pipeline. A value-position `${arr}(x)`
// still fires W216.

#[test]
fn set_indirect_array_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set token ::http::1\nset ${token}(status) eof\n");
    assert!(!has_code(&diags, "W216"), "{:?}", codes(&diags));
    assert!(!has_code(&diags, "W212"), "{:?}", codes(&diags));
}

#[test]
fn info_exists_indirect_array_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "info exists ${token}(-pipeline)\n");
    assert!(!has_code(&diags, "W216"));
    assert!(!has_code(&diags, "W212"));
}

#[test]
fn unset_and_vwait_indirect_array_silent() {
    for src in ["unset ${tok}(socketcoro)\n", "vwait ${token}(status)\n"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        assert!(!has_code(&lsp.open_ready(&uri, src), "W216"), "{src:?}");
    }
}

#[test]
fn value_position_still_fires_w216() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // `arr` is set first: an unset `${arr}` raises before the word is formed.
    let diags = lsp.open_ready(&uri, "set arr a\nputs ${arr}(x)\n");
    assert!(has_code(&diags, "W216"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W216").contains(&1));
}

#[test]
fn bare_dollar_name_still_fires_w212() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "set $x v\n"), "W212"));
}

// FP-STY-13: redefining an overridable Tcl *library* proc (`unknown`,
// `history`, `auto_*` …) is not shadowing a C built-in — no W113. Redefining a
// genuine built-in (`set`/`clock`) still fires.

#[test]
fn unknown_override_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "proc unknown args { return }\n"),
        "W113"
    ));
}

#[test]
fn library_procs_silent() {
    for name in ["history", "auto_execok", "tcl_findLibrary", "pkg_mkIndex"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("proc {name} {{args}} {{ return }}\n");
        assert!(!has_code(&lsp.open_ready(&uri, &src), "W113"), "{name}");
    }
}

#[test]
fn c_builtin_override_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(
        &lsp.open_ready(&uri, "proc set {a b} { return }\n"),
        "W113"
    ));
}

#[test]
fn non_bytecompiled_c_command_still_fires() {
    // clock/after/socket/glob are C commands (not byte-compiled) but must still
    // fire — the library-proc exemption must not over-reach.
    for name in ["clock", "after", "socket", "glob"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("proc {name} {{a}} {{ return }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "W113"), "{name}");
    }
}

// FP-STY-14: a body argument that is a single bare variable substitution
// (`eval $cmd`, `$state(-command)`, `after 0 $coroName`) is a script-valued
// reference, not an inline block — no W105 through the server pipeline. A
// quoted/composite interpolated body still fires.

#[test]
fn eval_single_var_body_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(&lsp.open_ready(&uri, "eval $cmd\n"), "W105"));
}

#[test]
fn callback_dispatch_body_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval :: $state(-command) $token\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W105"));
}

#[test]
fn execution_trace_callback_exact_arities_reach_lsp_diagnostics() {
    let mut lsp = Lsp::with_config(serde_json::json!({
        "features": { "linkedEditingRange": true, "crossFileResolution": true }
    }));
    let uri = unique_uri("tcl");
    let source = "proc target {} {}\n\
                  proc enterOk {command op} {}\n\
                  proc leaveOk {command code result op} {}\n\
                  proc enterWrong {command code result op} {}\n\
                  proc leaveWrong {command op} {}\n\
                  trace add execution target enter enterOk\n\
                  trace add execution target leave leaveOk\n\
                  trace add execution target enter enterWrong\n\
                  trace add execution target leave leaveWrong\n";
    lsp.open_ready(&uri, source);
    let diagnostics = lsp.pull_diagnostics(&uri);
    assert_eq!(on_line(&diagnostics, "E002"), BTreeSet::from([7]));
    assert_eq!(on_line(&diagnostics, "E003"), BTreeSet::from([8]));
}

#[test]
fn mixed_execution_trace_accepts_defaulted_and_variadic_callbacks() {
    let mut lsp = Lsp::with_config(serde_json::json!({
        "features": { "linkedEditingRange": true, "crossFileResolution": true }
    }));
    let uri = unique_uri("tcl");
    let source = "proc target {} {}\n\
                  proc fixedThree {a b c} {}\n\
                  proc defaulted {a b {c default} {d default}} {}\n\
                  proc variadic {a b args} {}\n\
                  trace add execution target {enter leave} fixedThree\n\
                  trace add execution target {enter leave} defaulted\n\
                  trace add execution target {enter leave} variadic\n";
    lsp.open_ready(&uri, source);
    let diagnostics = lsp.pull_diagnostics(&uri);
    assert_eq!(on_line(&diagnostics, "E002"), BTreeSet::from([4]));
    assert!(on_line(&diagnostics, "E003").is_empty(), "{diagnostics:?}");
    assert!(on_line(&diagnostics, "E005").is_empty(), "{diagnostics:?}");
}

#[test]
fn execution_trace_dynamic_and_malformed_operation_lists_abstain_end_to_end() {
    let mut lsp = Lsp::with_config(serde_json::json!({
        "features": { "linkedEditingRange": true, "crossFileResolution": true }
    }));
    let uri = unique_uri("tcl");
    let source = "proc target {} {}\n\
                  proc fixedThree {a b c} {}\n\
                  set operations {enter leave}\n\
                  trace add execution target $operations fixedThree\n\
                  trace add execution target \"{enter\" fixedThree\n";
    lsp.open_ready(&uri, source);
    let diagnostics = lsp.pull_diagnostics(&uri);
    for code in ["E002", "E003", "E005"] {
        assert!(
            on_line(&diagnostics, code).is_empty(),
            "{code} must abstain for unproved operation lists: {diagnostics:?}"
        );
    }
}

#[test]
fn after_and_dynamic_proc_silent() {
    for src in ["after 0 $coroName\n", "proc $fakeName $arglist $body\n"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        assert!(!has_code(&lsp.open_ready(&uri, src), "W105"), "{src:?}");
    }
}

#[test]
fn quoted_interpolated_body_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "eval \"do $script\"\n");
    assert!(has_code(&diags, "W105"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W105").contains(&0));
}

#[test]
fn composite_body_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "eval $cmd$args\n"), "W105"));
}

// FP-STY-15: a `$` immediately before a closing `"` (the regex end-anchor
// `"^foo$"` / `"\n$"`) is literal — the lexer must not merge the quoted word
// with the next, so no E002/E205 and no spurious W306. A live `$bar` in a quoted
// pattern still fires W306.

#[test]
fn regsub_end_anchor_no_errors() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "regsub \"\\n$\" $msg \"\" out\n");
    let cs = codes(&diags);
    assert!(!cs.contains("E002"), "{cs:?}");
    assert!(!cs.contains("E205"), "{cs:?}");
    assert!(!cs.contains("W306"), "{cs:?}");
}

#[test]
fn string_match_end_anchor_no_arity() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "string match \"abc$\" $x\n"),
        "E002"
    ));
}

#[test]
fn regex_end_anchor_no_w306() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "regexp -- \"^foo$\" $text\n"),
        "W306"
    ));
}

#[test]
fn live_var_in_quoted_pattern_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(
        &lsp.open_ready(&uri, "regexp -- \"^foo$bar\" $text\n"),
        "W306"
    ));
}

#[test]
fn quoted_pure_var_pattern_no_w306() {
    // `"$pattern"` is byte-for-byte identical at runtime to the bare
    // `$pattern` parameterised-pattern idiom (the quotes group nothing) — the
    // canonical form, not a foot-gun. No W306.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(!has_code(
        &lsp.open_ready(&uri, "regexp -- \"$pattern\" $text\n"),
        "W306"
    ));
}
