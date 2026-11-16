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

//! Argument-count and argument-shape diagnostics for builtins and for same-file procs, aliases, `TclOO` calls and lambdas.

use super::*;

#[test]
fn arity_error_is_e002_with_error_severity() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set\n");
    let e002: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E002"))
        .collect();
    assert_ne!(e002, [] as [&serde_json::Value; 0]);
    assert_eq!(e002[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn invalid_literal_formal_parameter_list_is_e006() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc invalid {{name default extra}} {}\n");
    let e006 = with_code(&diags, "E006");
    assert_eq!(e006.len(), 1, "got {diags:?}");
    assert_eq!(e006[0].get("severity").and_then(Value::as_i64), Some(1));
    assert!(message(&e006[0]).contains("too many fields"));
}

// `file link -symbolic linkName target` is valid Tcl — the optional
// `-linktype` flag precedes the two positionals — so the arity check must
// skip declared option flags before counting positionals, or a valid call
// wrongly reports "Too many arguments for 'file link'".

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
    let diags = lsp.open_ready(&uri, "file link a b c\n");
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert_ne!(e003, [] as [&serde_json::Value; 0]);
    assert_eq!(e003[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn string_match_nocase_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string match -nocase $pat $str\n");
    assert!(!has_code(&diags, "E003"));
}

// The too-many-args squiggle covers only the surplus words, and several
// command arities match C Tcl 9.

#[test]
fn e003_too_many_args_highlights_only_the_surplus_words() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "lreverse {a b c} extra1 extra2\n");
    let e003 = with_code(&diags, "E003");
    assert_eq!(e003.len(), 1, "got {diags:?}");
    // Tight range: only `extra1 extra2` (cols 17..30), not the whole command
    // from column 0.
    assert_eq!(diag_range(&e003[0]), ((0, 17), (0, 30)), "got {diags:?}");
}

#[test]
fn lmap_even_arg_count_is_e005() {
    // `lmap` shares `foreach`'s odd/even grammar; an even count is wrong.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "lmap a b c d\n");
    assert!(has_code(&diags, "E005"), "got {diags:?}");
    // A valid odd count is silent.
    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "lmap x {1 2} {incr x}\n");
    assert!(!has_code(&diags2, "E005"), "got {diags2:?}");
}

#[test]
fn global_and_variable_zero_args_have_no_arity_error() {
    let mut lsp = Lsp::tcl();
    for src in ["global\n", "variable\n"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(!has_code(&diags, "E002"), "{src:?} → {diags:?}");
    }
}

// Arity checks apply beyond the builtin registry to same-file proc /
// `interp alias` / `rename` calls: the call site itself must be checked, not
// just out-of-range variable references inside the proc body (which fire
// W210 independently).

#[test]
fn same_file_proc_call_too_many_args_is_e003() {
    // A 7-parameter proc called with 8 arguments.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven eight
";
    let diags = lsp.open_ready(&uri, src);
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert!(
        !e003.is_empty(),
        "expected E003 for the 8-arg call to a 7-param proc; got {diags:?}"
    );
    assert!(message(e003[0]).contains("demonstrate"));
    assert_eq!(e003[0].get("severity").and_then(Value::as_i64), Some(1)); // Error
}

#[test]
fn same_file_proc_call_with_correct_arity_has_no_arity_error() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
proc demonstrate {arg1 arg2 arg3 arg4 arg5 arg6 arg7} {
    return \"$arg1 $arg2 $arg3 $arg4 $arg5 $arg6 $arg7\"
}
demonstrate one two three four five six seven
";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "E002"));
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn same_file_renamed_proc_call_inherits_original_arity() {
    // `rename` is a pure name move — the renamed name must still be
    // checked against the proc's own (unchanged) arity.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc target {a b c} {}\nrename target target_orig\ntarget_orig 1 2\n",
    );
    assert!(has_code(&diags, "E002"));
}

#[test]
fn same_file_interp_alias_call_arity_is_shifted_by_prepended_args() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2\n",
    );
    assert!(has_code(&diags, "E002"));
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "proc target {a b c} {}\ninterp alias {} shortcut {} target 100\nshortcut 2 3\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_forward_via_my_arity_is_shifted_by_prepended_args() {
    // `forward NAME my TARGET ?ARG…?` is the documented TclOO idiom for
    // forwarding to a sibling method — confirmed against tclsh 9.0.4 that
    // a bare method name (`forward NAME TARGET`) is never a valid forward
    // target, but routing through `my` resolves via the receiver's own
    // method-resolution order, arity-shifted by any arguments after it.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "\
oo::class create Widget {
    method base {a b c} { return \"$a$b$c\" }
    forward fwd my base fixedarg
}
set w1 [Widget new]
$w1 fwd 1 2 3
";
    let diags = lsp.open_ready(&uri, src);
    let e003: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("E003"))
        .collect();
    assert!(
        !e003.is_empty(),
        "expected E003 for a 3-arg call to a forward shifted down to 2 args; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "oo::class create Widget {\n    method base {a b c} { return \"$a$b$c\" }\n    forward fwd my base fixedarg\n}\nset w1 [Widget new]\n$w1 fwd 1 2\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_constructor_call_arity_is_checked() {
    // `ClassName new ?args?` / `ClassName create name ?args?` is checked
    // against the class's own (or nearest inherited) `constructor`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Widget { constructor {a b} { } }\nWidget new 1\n",
    );
    assert!(
        has_code(&diags, "E002"),
        "expected E002 for a 1-arg call to a 2-arg constructor; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_create = lsp.open_ready(
        &uri2,
        "oo::class create Widget { constructor {a b} { } }\nWidget create fido 1 2 3\n",
    );
    assert!(
        has_code(&diags_create, "E003"),
        "expected E003 for create's mandatory name plus 3 extra args against a 2-arg constructor; got {diags_create:?}"
    );
    let uri3 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri3,
        "oo::class create Widget { constructor {a b} { } }\nWidget create fido 1 2\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_constructor_inherited_through_superclass_is_checked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Base { constructor {a b} { } }\noo::class create Sub { superclass Base }\nSub new 1\n",
    );
    assert!(
        has_code(&diags, "E002"),
        "a subclass with no constructor of its own must inherit the superclass's; got {diags:?}"
    );
}

#[test]
fn same_file_tcloo_no_explicit_constructor_is_never_checked() {
    // TclOO's default (inherited from `oo::object`) constructor accepts
    // and ignores any number of arguments.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Widget { method bar {} { } }\nWidget new 1 2 3 4 5\n",
    );
    assert!(!has_code(&diags, "E002"));
    assert!(!has_code(&diags, "E003"));
}

#[test]
fn same_file_tcloo_next_call_arity_is_checked() {
    // `next` re-invokes the current method's next-in-MRO implementation, so
    // its arity is checked against the resolved superclass method's own
    // signature.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Base { method speak {a b} { return \"$a$b\" } }\n\
         oo::class create Derived { superclass Base\n method speak {a b} { next 1 } }\n\
         [Derived new] speak x y\n",
    );
    assert!(
        has_code(&diags, "E002"),
        "expected E002 for a 1-arg `next` against a 2-arg superclass method; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "oo::class create Base { method speak {a b} { return \"$a$b\" } }\n\
         oo::class create Derived { superclass Base\n method speak {a b} { next 1 2 } }\n\
         [Derived new] speak x y\n",
    );
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn same_file_tcloo_next_in_a_constructor_has_its_arity_checked() {
    // A `next` inside a `constructor` chains to the superclass constructor.
    // Against tclsh 9.0.4, `oo::class create Base { constructor {n} {…} }` /
    // `oo::class create Derived { superclass Base; constructor {} { next 1 2 3 } }`
    // then `Derived new` crashes with `wrong # args: should be "next n"`, so
    // constructor `next` chains must be arity-checked the same as a `next`
    // inside an ordinary method.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Base { constructor {n} { set x $n } }\n\
         oo::class create Derived { superclass Base\n constructor {} { next too many args here } }\n\
         Derived new\n",
    );
    assert!(
        has_code(&diags, "E003"),
        "expected E003 for a 4-arg `next` against a 1-param superclass constructor; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(
        &uri2,
        "oo::class create Base { constructor {n} { set x $n } }\n\
         oo::class create Derived { superclass Base\n constructor {n} { next $n } }\n\
         Derived new 1\n",
    );
    assert!(!has_code(&diags_ok, "E002"), "{diags_ok:?}");
    assert!(!has_code(&diags_ok, "E003"), "{diags_ok:?}");
}

#[test]
fn same_file_tcloo_nextto_call_arity_checks_named_target() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Root { method speak {a} { return $a } }\n\
         oo::class create Mid { superclass Root\n method speak {a b} { return \"$a$b\" } }\n\
         oo::class create Derived { superclass Mid\n method speak {a b} { nextto Root 1 2 } }\n\
         [Derived new] speak x y\n",
    );
    assert!(
        has_code(&diags, "E003"),
        "expected E003 for a 2-arg `nextto Root` against Root's 1-arg speak; got {diags:?}"
    );
}

#[test]
fn same_file_apply_lambda_call_arity_is_checked() {
    // A direct `apply {{params} body} ?args?` call has its arity checked
    // against the lambda's own parameter list.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "apply {{a b} {return [expr {$a+$b}]}} 1\n");
    assert!(
        has_code(&diags, "E002"),
        "expected E002 for a 1-arg call to a 2-param lambda; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "apply {{a b} {return [expr {$a+$b}]}} 1 2\n");
    assert!(!has_code(&diags_ok, "E002"));
    assert!(!has_code(&diags_ok, "E003"));
}

#[test]
fn dict_create_odd_key_value_tail_is_checked() {
    // `dict create ?key value ...?` needs an even tail.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "dict create a\n");
    assert!(
        has_code(&diags, "E005"),
        "expected E005 for an odd dict create tail; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "dict create a b\n");
    assert!(!has_code(&diags_ok, "E005"));
}

#[test]
fn foreach_unpaired_varlist_is_checked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "foreach x $l y {puts $x}\n");
    assert!(
        has_code(&diags, "E005"),
        "expected E005 for an unpaired foreach var-list; got {diags:?}"
    );
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "foreach x $l {puts $x}\n");
    assert!(!has_code(&diags_ok, "E005"));
}

#[test]
fn switch_flat_unpaired_pattern_is_checked() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "switch $s a b c\n");
    assert!(
        has_code(&diags, "E005"),
        "expected E005 for an unpaired switch pattern; got {diags:?}"
    );
    // Both valid shapes stay silent: the flat paired form and the
    // single-braced-body shorthand.
    let uri2 = unique_uri("tcl");
    let diags_ok = lsp.open_ready(&uri2, "switch $s a b c d\n");
    assert!(!has_code(&diags_ok, "E005"));
    let uri3 = unique_uri("tcl");
    let diags_braced = lsp.open_ready(&uri3, "switch $s {a b c d e f}\n");
    assert!(!has_code(&diags_braced, "E005"));
}

/// `tcl::OptProc` (the `opt` package's automatic-option-parsing proc
/// definer) needs its own `AnalyserHookId` — without one, `all_procs`
/// keeps the stub's `{}`-arity `ProcDef` and every real call falsely draws
/// "wrong number of arguments" (E003). tclsh 9.0/8.6 confirm the runtime
/// always installs `::proc $name args {...}`, so any call arity is
/// legitimate.
#[test]
fn opt_proc_real_call_draws_no_false_arity_diagnostic_end_to_end() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require opt\n::tcl::OptProc greet {child -use -display} { return $child }\ngreet a b c\n",
    );
    assert!(
        !has_code(&diags, "E003") && !has_code(&diags, "W123"),
        "a real tcl::OptProc call must never draw a false arity or unknown-command diagnostic: {:?}",
        codes(&diags)
    );
}

/// The `tcl::OptProc` arity trade-off is a deliberate choice, not an
/// oversight.
///
/// The definer installs `::proc $name args {…}` at run time, so the
/// declared optlist says nothing about how many *positional* arguments a
/// call may pass; C Tcl enforces it from the parsed optlist at call time,
/// which is a runtime fact this analysis does not model. tclsh 9.0.4 /
/// 8.6.16 do raise `too many arguments` for the overflow below — and this
/// analysis deliberately stays silent about it, because the only
/// alternative was a false E003 on legitimate calls. Both halves are
/// asserted so the asymmetry cannot drift unnoticed.
#[test]
fn opt_proc_arity_is_deliberately_unchecked_in_both_directions_end_to_end() {
    let mut lsp = Lsp::tcl();
    for call in ["greet interp1", "greet interp1 extraPositionalArg"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(
            &uri,
            &format!(
                "package require opt\n::tcl::OptProc greet {{child -use -display}} {{ return $child }}\n{call}\n"
            ),
        );
        assert!(
            !has_code(&diags, "E002") && !has_code(&diags, "E003"),
            "OptProc calls draw no arity diagnostic either way — `{call}`: {:?}",
            codes(&diags)
        );
    }
    // TN control — an ordinary proc still gets real arity checking, so the
    // silence above is scoped to the definer and not a global retreat.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc plain {a} { return $a }\nplain 1 2\n");
    assert!(
        has_code(&diags, "E002") || has_code(&diags, "E003"),
        "an ordinary proc must still be arity-checked: {:?}",
        codes(&diags)
    );
}
