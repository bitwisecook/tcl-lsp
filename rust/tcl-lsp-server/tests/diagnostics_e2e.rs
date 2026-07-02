//! Native port of `tests/lsp_e2e/test_diagnostics_e2e.py`.
//!
//! Push diagnostics, end-to-end against the packaged server. The server
//! advertises no pull provider, so these assert on the `publishDiagnostics` the
//! server pushes after analysis, keyed by version.

mod common;

use common::{Lsp, unique_uri};

use serde_json::Value;
use std::collections::BTreeSet;
use std::time::Duration;

/// The set of `code` strings carried by `diags` (mirrors Python `_codes`).
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// Whether any diagnostic carries `code`.
fn has_code(diags: &[Value], code: &str) -> bool {
    codes(diags).contains(code)
}

/// The set of start lines for diagnostics carrying `code` (mirrors `_on_line`).
fn on_line(diags: &[Value], code: &str) -> BTreeSet<i64> {
    diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some(code) && d.get("range").is_some())
        .filter_map(|d| {
            d.get("range")
                .and_then(|r| r.get("start"))
                .and_then(|s| s.get("line"))
                .and_then(Value::as_i64)
        })
        .collect()
}

/// A diagnostic's `code`, normalised to a `String` (stringifying non-strings).
fn code_str(d: &Value) -> Option<String> {
    match d.get("code") {
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
        None => None,
    }
}

/// A diagnostic's `message` text (empty if absent).
fn message(d: &Value) -> &str {
    d.get("message").and_then(Value::as_str).unwrap_or("")
}

// -- TestPushDiagnostics -------------------------------------------------

#[test]
fn unbraced_expr_is_w100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "if $a {puts x}\n"), "W100"));
}

#[test]
fn catch_without_result_is_w302() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "catch {error e}\n"), "W302"));
}

#[test]
fn arity_error_is_e002_with_error_severity() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set\n");
    let e002: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E002"))
        .collect();
    assert!(!e002.is_empty());
    assert_eq!(e002[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn clean_file_has_no_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(
        lsp.open_ready(&uri, "set x [clock seconds]\nputs $x\n")
            .is_empty()
    );
}

#[test]
fn renamed_away_command_is_w128() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc a {} {return 1}\na\nrename a b\na\n");
    assert!(has_code(&diags, "W128"));
}

#[test]
fn unbraced_expr_inside_catch_body_is_w100() {
    // The analyser must recurse into `catch { ... }` bodies, so the unbraced
    // `expr` inside is a W100 just as at the top level (catch-body-walk fix).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "catch { set y [expr $a + $b] } msg\n");
    assert!(has_code(&diags, "W100"));
    assert_eq!(on_line(&diags, "W100"), BTreeSet::from([0]));
}

#[test]
fn unbraced_expr_inside_tcltest_body_is_w100() {
    // `tcltest::test` evaluates its body as Tcl, so diagnostics inside it are
    // real (tcltest body-role resolver fix).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "package require tcltest\ntcltest::test t {d} { set y [expr $a + $b] } {}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W100"));
}

// -- TestSubcommandOptionArity -------------------------------------------
// End-to-end arity checks for per-subcommand option flags (issue #581).
// `file link -symbolic linkName target` is valid Tcl — the optional
// `-linktype` flag precedes the two positionals — but the packaged server used
// to emit "Too many arguments for 'file link'" because the subcommand's declared
// options were never skipped before the positional count.

#[test]
fn file_link_symbolic_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "file link -symbolic $dst $src\n");
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn file_link_hard_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "file link -hard $dst $src\n");
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn file_link_too_many_positionals_is_e003() {
    // No option flag: three positionals genuinely exceed the max of 2, so the
    // arity error must still fire (the fix skips options, not real arguments).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "file link $a $b $c\n");
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert!(!e003.is_empty());
    assert_eq!(e003[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn string_match_nocase_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string match -nocase $pat $str\n");
    assert!(!has_code(&diags, "E003"));
}

// -- TestDiagnosticCanaries ----------------------------------------------
// One canary per analysis family, locked to the server's published output.

// -- W210: read of a possibly-unset variable -----------------------------

#[test]
fn w210_read_before_set_on_path_merge() {
    // `y` is defined only inside the `if` arm; the read at the merge point may
    // see it unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    if {$x} {\n        set y 1\n    }\n    return $y\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    // The diagnostic anchors on the read site (`return $y`), not the def.
    assert!(on_line(&diags, "W210").contains(&4));
}

#[test]
fn w210_use_after_unset() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set a 1\nunset a\nputs $a\n");
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W210").contains(&2));
}

#[test]
fn w210_silent_when_set_on_all_paths() {
    // Defined on both branches → no read-before-set (must-stay-silent).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    if {$x} {\n        set y 1\n    } else {\n        set y 2\n    }\n    return $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- W307: a variable used in command position ---------------------------

#[test]
fn w307_known_literal_non_command_fires() {
    // `cmd` resolves to the literal `foo` which is not a command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n    set cmd foo\n    $cmd bar\n}\n");
    assert!(has_code(&diags, "W307"), "{:?}", codes(&diags));
}

#[test]
fn w307_silent_for_opaque_dispatch_target() {
    // `$self` is an opaque parameter (a method-dispatch idiom); with no known
    // non-command literal value, W307 must not fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {self} {\n    $self configure -x 1\n}\n");
    assert!(!has_code(&diags, "W307"), "{:?}", codes(&diags));
}

// -- W220: dead store -----------------------------------------------------

#[test]
fn w220_dead_store_fires() {
    // `x` is set, then overwritten before any read → the first store is dead.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {} {\n    set x 1\n    set x 2\n    return $x\n}\n");
    assert!(has_code(&diags, "W220"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W220").contains(&1));
}

#[test]
fn w220_silent_when_value_is_read_between() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {} {\n    set x 1\n    puts $x\n    set x 2\n    return $x\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W220"));
}

// -- Clean code stays clean (cross-family negative control) --------------

#[test]
fn clean_dataflow_has_no_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {x} {\n    set d [dict create a 1]\n    if {[dict exists $d a]} {\n        return [dict get $d a]\n    }\n    return $x\n}\n";
    assert!(lsp.open_ready(&uri, src).is_empty());
}

// -- TestIndirectArrayIdiom ----------------------------------------------
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
    let diags = lsp.open_ready(&uri, "puts ${arr}(x)\n");
    assert!(has_code(&diags, "W216"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W216").contains(&0));
}

#[test]
fn bare_dollar_name_still_fires_w212() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "set $x v\n"), "W212"));
}

// -- TestOverridableLibraryProcs -----------------------------------------
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

// -- TestSingleVarBodyW105 -----------------------------------------------
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

// -- TestDollarBeforeCloseQuoteW306 --------------------------------------
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

// -- TestControlFlowRBSFamilyE2E -----------------------------------------
// W210 read-before-set, control-flow modelling family (PR #634).

// -- tailcall ends straight-line flow (FP-RBS-13) ------------------------

#[test]
fn tailcall_terminated_branch_silent() {
    // `tailcall g` returns from the proc, so `return $result` is reached only
    // via the else branch where `result` is set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {cond} {\n    if {$cond} {\n        tailcall g\n    } else {\n        set result 1\n    }\n    return $result\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn non_terminating_branch_still_fires() {
    // Replace the tailcall with a completing command → `result` is maybe-unset
    // on the then-path, so W210 must fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {cond} {\n    if {$cond} {\n        puts hi\n    } else {\n        set result 1\n    }\n    return $result\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
}

// -- non-empty-literal foreach runs its body (FP-RBS-17) -----------------

#[test]
fn foreach_non_empty_literal_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {1 2 3} { set y $x }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn foreach_empty_literal_still_fires() {
    // An empty literal never runs the body → `y` is unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {} { set y $x }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- for whose condition is true on entry runs its body (FP-RBS-18) ------

#[test]
fn for_true_on_entry_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    for {set i 0} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn for_false_on_entry_still_fires() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    for {set i 5} {$i < 3} {incr i} { set y $i }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- while 1 only exits via break, where the var is set (FP-RBS-16) ------

#[test]
fn while1_break_set_silent() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    while 1 { set y 1; break }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn normal_while_still_fires() {
    // A non-constant condition may run zero times → maybe-unset read.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {n} {\n    while {$n > 0} { set y 1; incr n -1 }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- opaque switch whose every arm exits is a terminator (FP-RBS-15) -----

#[test]
fn all_arms_return_makes_trailing_read_unreachable() {
    // Every arm returns, so `puts $y` is unreachable dead code → no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 1 } default { return 2 } }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn no_default_switch_falls_through_fires() {
    // Without a `default` an unmatched subject falls through to the read, so the
    // switch is not a terminator and `y` is maybe-unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 1 } b* { return 2 } }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- opaque switch must-define excludes non-completing arms (FP-RBS-14) --

#[test]
fn returning_arm_excluded_from_must_define() {
    // The `a*` arm returns before reaching `puts $y`; the only path that does
    // (default) sets `y` → definitely defined, no W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {x} {\n    switch -glob -- $x { a* { return 0 } default { set y 2 } }\n    puts $y\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn break_arm_escaping_loop_still_fires() {
    // Codex regression on #634: `break`/`continue` are loop-jumps, not
    // proc-exits, so a break arm does NOT define the other arm's var on the path
    // that escapes the loop — `y` is maybe-unset, W210 must fire.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    foreach x {a} { switch -glob -- $x { a* { break } default { set y 1 } } }\n    puts $y\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- TestWhenBodyDialectGatingE2E ----------------------------------------
// `when` is an iRules-only builtin (PR #640). Under plain Tcl it is an unknown
// would-be user command whose braced argument is opaque *data*, not a handler
// script — so its body must not be analysed.

#[test]
fn when_body_not_analysed_under_plain_tcl() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n");
    // `when` itself is unknown under Tcl, but the opaque body must not be
    // recursed into: no W123 naming the body command, no W210 on its var.
    let body_w123: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W123") && message(d).contains("boguscmd"))
        .collect();
    assert!(body_w123.is_empty(), "{diags:?}");
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
}

// -- TestConstantStringConditionFoldE2E ----------------------------------
// I230 (always-true/false condition; alternate branch unreachable) now folds
// `==`/`!=` on string operands, matching Tcl's polymorphic compare
// (`expr {"foo" == "foo"}` -> 1) — previously only the `eq`/`ne` spelling folded
// (PR #640).

#[test]
fn double_equals_string_condition_folds() {
    for op in ["==", "eq"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "I230"), "{op}");
    }
}

#[test]
fn bang_equals_string_condition_folds() {
    for op in ["!=", "ne"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "I230"), "{op}");
    }
}

// -- TestDiagnosticsTrackEdits -------------------------------------------

#[test]
fn fixing_the_source_clears_the_diagnostic() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if $a {puts x}\n");
    assert!(has_code(&diags, "W100"));
    // Wrap the expression in braces via an incremental edit: `$a` -> `{$a}`.
    lsp.change_document(
        &uri,
        2,
        serde_json::json!([
            {
                "range": {
                    "start": { "line": 0, "character": 3 },
                    "end": { "line": 0, "character": 3 },
                },
                "text": "{",
            },
            {
                "range": {
                    "start": { "line": 0, "character": 6 },
                    "end": { "line": 0, "character": 6 },
                },
                "text": "}",
            },
        ]),
    );
    let cleared = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(!has_code(&cleared, "W100"));
}

#[test]
fn introducing_an_error_publishes_it() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(lsp.open_ready(&uri, "puts hello\n").is_empty());
    // Replace the whole line with an arity error.
    lsp.replace_document(&uri, 2, "set\n");
    let diags = lsp.await_diagnostics_version(&uri, Some(2), Duration::from_secs(30));
    assert!(has_code(&diags, "E002"));
}

// -- TestSoundnessRegressionsE2E -----------------------------------------
// End-to-end coverage for three latent W210 soundness bugs surfaced while
// reviewing the Rust port and fixed in the analyser. Ground truth is real
// tclsh 9.0.3.

// -- Omitted-arg call-site constants are poisoned (interproc) ------------

#[test]
fn omitted_default_arg_does_not_hide_read_before_set() {
    // `p` (slot 0 omitted → default `x == 0`) leaves `y` unset, so `puts $y` is
    // a real read-before-set; the literal `1` passed by `p 1` must not be bound
    // as a constant for `x`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc p {{x 0}} {\n    if {$x} {\n        set y 5\n    }\n    puts $y\n}\np\np 1\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "W210"), "{:?}", codes(&diags));
    assert!(on_line(&diags, "W210").contains(&4));
}

#[test]
fn uniform_literal_arg_still_binds_silent() {
    // Every caller passes `1` at slot 0 → the constant binding holds, the
    // `if {$x}` body is provably taken, `y` is always set: stay silent.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc q {{x 0}} {\n    if {$x} {\n        set y 5\n    }\n    puts $y\n}\nq 1\nq 1\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- regexp `-expanded` is not unconditionally literal-safe --------------

#[test]
fn regexp_expanded_whitespace_pattern_silent() {
    // `-expanded` ignores unescaped whitespace, so `{a b}` matches the substring
    // `ab` and writes `v`; reading `v` must not fire W210.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc g {input} {\n    regexp -expanded {a b} $input v\n    puts $v\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn regexp_expanded_clean_literal_still_fires() {
    // A whitespace/`#`-free literal stays provably literal: `{x}` never matches
    // `X`, never writes `w` → reading `w` is read-before-set.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc g {} {\n    regexp -expanded {x} X w\n    puts $w\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}

// -- try body throw keeps its handler exception edge ---------------------

#[test]
fn try_body_throw_keeps_handler_defs_silent() {
    // `x` is set before the only throw, so the handler always sees it (tclsh
    // prints `1`): stay silent.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {} {\n    try {\n        set x 1\n        error boom\n    } on error {} {\n        puts $x\n    }\n}\n";
    assert!(!has_code(&lsp.open_ready(&uri, src), "W210"));
}

#[test]
fn try_body_earlier_conditional_throw_fires() {
    // The handler is reachable from every throw point; `x` is unset on the
    // earlier `if {$c} {error a}` path, so the read is maybe-unset.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc f {c} {\n    try {\n        if {$c} { error a }\n        set x 1\n        error b\n    } on error {} {\n        puts $x\n    }\n}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W210"));
}
