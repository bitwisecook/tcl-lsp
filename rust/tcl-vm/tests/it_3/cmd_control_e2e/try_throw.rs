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

//! `try` and `throw` through the registered builtins (`cmd_try.rs`): handlers, `trap`, `finally`, grammar errors and the `-during` chain.

use super::*;

// try / throw  (cmd_try.rs)
//
// Driven through a computed command name `set t try; $t ...` so the construct
// routes through the registered `cmd_try`/`cmd_throw` builtins. (The literal
// `try { ... } on ... { ... }` form is mis-compiled — see `try_literal_form_*`.)

/// `try body` with no handlers returns the body result; an unmatched error
/// propagates.
#[test]
fn try_body_only_passes_through() {
    // tclsh: `2`
    assert_eq!(run("set t try; $t { expr {1+1} }").1, "2");
    // No matching handler: the error propagates.
    // tclsh: error "boom"
    let (ok, msg, _) = run("set t try; $t { error boom }");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

/// `on code varList script`: an `on ok` handler binds the body result and runs;
/// an `on error` handler binds `[resultVar optionsVar]` and the options dict
/// carries `-code`/`-errorcode`.
#[test]
fn try_on_ok_and_on_error() {
    // tclsh: `5`
    assert_eq!(run("set t try; $t { expr {2+3} } on ok r { set r }").1, "5");
    // on error binds result + options; -code is 1 (error).
    // tclsh: `oops 1`
    assert_eq!(
        run("set t try; $t { error oops } on error {m o} { list $m [dict get $o -code] }").1,
        "oops 1",
    );
}

/// `on` matches the numeric completion code: `on 7` matches a `return -code 7`
/// body; the symbolic word `break` (code 3) matches a `break` body.
#[test]
fn try_on_numeric_and_symbolic_codes() {
    // A custom completion code 7 raised by the body matches `on 7`.
    // tclsh: `hi`
    assert_eq!(
        run("set t try; $t { return -code 7 hi } on 7 r { set r }").1,
        "hi",
    );
    // Symbolic `break` (= code 3): a `break` body is caught by `on break`.
    // tclsh: `caught`
    assert_eq!(
        run("set t try; $t { break } on break {} { set r caught }").1,
        "caught",
    );
}

/// `trap pattern varList script`: matches an error whose `-errorcode` has the
/// given leading sublist. An empty pattern matches any error.
#[test]
fn try_trap_errorcode_prefix() {
    // A `throw {MY ERR} boom` is trapped by `trap MY` (leading-sublist match).
    // tclsh: `caught boom {MY ERR}`
    assert_eq!(
        run("set t try; $t { throw {MY ERR} boom } trap MY {m o} \
             { list caught $m [dict get $o -errorcode] }")
        .1,
        "caught boom {MY ERR}",
    );
    // A `return -code error -errorcode {A B}` body is trapped by `trap {A B}`.
    // tclsh: `fail`
    assert_eq!(
        run("set t try; $t { return -code error -errorcode {A B} fail } trap {A B} m { set m }").1,
        "fail",
    );
    // Empty trap pattern matches any error.
    // tclsh: `any`
    assert_eq!(
        run("set t try; $t { error whatever } trap {} {} { set r any }").1,
        "any",
    );
}

/// A `trap` whose pattern is not a prefix of the error's `-errorcode` does NOT
/// match, so the error propagates.
#[test]
fn try_trap_no_match_propagates() {
    // errorcode is {MY ERR}; `trap OTHER` does not match → propagate.
    // tclsh: error "boom" (rc 1)
    let (ok, msg, _) = run("set t try; $t { throw {MY ERR} boom } trap OTHER {} { set r x }");
    assert!(!ok);
    assert_eq!(msg, "boom");
}

/// `finally` always runs; an ok finally leaves the prior outcome, a throwing
/// finally overrides it.
#[test]
fn try_finally_runs_and_can_override() {
    // finally runs for its side effect; the on-error result is preserved.
    // tclsh stdout: "FIN\n", and the `try` result is "got".
    assert_eq!(
        run("set t try; puts [$t { error e } on error {} { set r got } finally { puts FIN }]").2,
        "FIN\ngot\n",
    );
    // A throwing finally overrides the handler's result/outcome.
    // tclsh: `finerr`
    let (ok, msg, _) =
        run("set t try; $t { error e } on error {} { set x ok } finally { error finerr }");
    assert!(!ok);
    assert_eq!(msg, "finerr");
}

/// `finally` runs even when the body succeeds and no handler matches.
#[test]
fn try_finally_runs_on_success() {
    // tclsh stdout: "FIN\n", result "5".
    assert_eq!(
        run("set t try; puts [$t { expr {2+3} } finally { puts FIN }]").2,
        "FIN\n5\n",
    );
}

/// A handler that itself throws supersedes the body's exception and chains it
/// via `-during` (TIP 329 exception chaining): the body's options ride along.
#[test]
fn try_handler_throws_chains_during() {
    let (ok, _msg, _) = run("set t try; $t { error body } on error {} { error handler }");
    assert!(!ok);
    // The propagated error's options carry a `-during` link to the body error,
    // whose own `-code` is 1.
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set t try\n",
            "catch {$t { error body } on error {} { error handler }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

/// An `on error -` (dash body) falls through to the next clause's body.
#[test]
fn try_dash_fallthrough() {
    // The first clause's `-` body falls through to the next `on` clause's body.
    // tclsh: `fell`
    assert_eq!(
        run("set t try; $t { error e } on error {} - on error {} { set r fell }").1,
        "fell",
    );
    // A multi-step `-` chain across `trap` clauses resolves to the next real body.
    // tclsh: `td`
    assert_eq!(
        run("set t try; $t { throw {A} x } trap A {} - trap {} {} { set r td }").1,
        "td",
    );
}

/// `try`'s handler-type word is a `Tcl_GetIndexFromObj(…,
/// "handler type", 0)` table, so the three types abbreviate and the empty word
/// — a prefix of all three — is `ambiguous handler type ""`, not `bad`.
///
/// tclsh 8.6.16 / 9.0.4:
///   try {} o error {} {}  -> {}       ;  try {} f {}      -> {}
///   try {} t {} {} {}     -> {}
///   try {} {} error {} {} -> ambiguous handler type "": must be finally, on, or trap
///   try {} x error {} {}  -> bad handler type "x": must be finally, on, or trap
///   try {} x              -> bad handler type "x": … (the type precedes the arity)
#[test]
fn try_handler_type_resolves_like_tcl_get_index_from_obj() {
    const MUST: &str = "must be finally, on, or trap";
    // Unique prefixes resolve to their clause grammar.
    assert_eq!(run("set t try; $t {set x ok} o error {} {set x h}").1, "ok");
    assert_eq!(run("set t try; $t {set x ok} f {set y 1}").1, "ok");
    assert_eq!(run("set t try; $t {set x ok} t {} {} {set x h}").1, "ok");
    // The empty word prefixes all three ⇒ ambiguous.
    assert_eq!(
        run("set t try; catch {$t good {} error {} {x}} e; set e").1,
        format!("ambiguous handler type \"\": {MUST}"),
    );
    // The type is resolved before the clause's arity.
    assert_eq!(
        run("set t try; catch {$t good x} e; set e").1,
        format!("bad handler type \"x\": {MUST}"),
    );
}

/// `try` grammar errors are raised before the body runs (validated by
/// `parse_clauses`).
#[test]
fn try_grammar_errors() {
    // tclsh: `bad handler type "badkw": must be finally, on, or trap`
    assert_eq!(
        run("set t try; catch {$t good badkw foo} e; set e").1,
        "bad handler type \"badkw\": must be finally, on, or trap",
    );
    // tclsh: `wrong # args to on clause: must be "... on code variableList script"`
    assert_eq!(
        run("set t try; catch {$t good on} e; set e").1,
        "wrong # args to on clause: must be \"... on code variableList script\"",
    );
    // tclsh: `wrong # args to trap clause: must be "... trap pattern variableList script"`
    assert_eq!(
        run("set t try; catch {$t good trap} e; set e").1,
        "wrong # args to trap clause: must be \"... trap pattern variableList script\"",
    );
    // tclsh: `wrong # args to finally clause: must be "... finally script"`
    assert_eq!(
        run("set t try; catch {$t good finally} e; set e").1,
        "wrong # args to finally clause: must be \"... finally script\"",
    );
    // tclsh: `finally clause must be last`
    assert_eq!(
        run("set t try; catch {$t good finally a b} e; set e").1,
        "finally clause must be last",
    );
    // tclsh: `bad completion code "notacode": ...`
    assert_eq!(
        run("set t try; catch {$t good on notacode {} {x}} e; set e").1,
        "bad completion code \"notacode\": must be ok, error, return, break, \
         continue, or an integer",
    );
    // tclsh: `last non-finally clause must not have a body of "-"`
    assert_eq!(
        run("set t try; catch {$t good on error {} -} e; set e").1,
        "last non-finally clause must not have a body of \"-\"",
    );
    // tclsh: `wrong # args: should be "try body ?handler ...? ?finally script?"`
    assert_eq!(
        run("set t try; catch {$t} e; set e").1,
        "wrong # args: should be \"try body ?handler ...? ?finally script?\"",
    );
}

/// A `trap` pattern that is not a well-formed list errors (the `as_list` Err arm
/// in `parse_clauses`). The VM reports a `must be a list` prefix error.
#[test]
fn try_trap_bad_pattern_list() {
    // An unbalanced brace makes the pattern an invalid list; the parse error is
    // caught. The VM's `parse_clauses` rejects it with a "must be a list" wording.
    let (ok, m, _) = run("set t try; catch {$t good trap \"a b \\{\" {} {x}} e; set e");
    assert!(ok, "the catch should complete: {m}");
    assert!(
        m.contains("must be a list"),
        "expected a 'must be a list' prefix error, got: {m}",
    );
}

/// A completion code outside the signed 32-bit range is not a valid `on` code
/// word (error-20.2): `code_word_to_int` rejects it.
#[test]
fn try_on_code_out_of_int_range() {
    // tclsh: `bad completion code "9999999999": ...`
    assert_eq!(
        run("set t try; catch {$t good on 9999999999 {} {x}} e; set e").1,
        "bad completion code \"9999999999\": must be ok, error, return, break, \
         continue, or an integer",
    );
}

/// `throw type message` raises an error with `-errorcode type`. The type must be
/// a non-empty list; `throw` arity is exactly 2.
#[test]
fn throw_command() {
    // tclsh: `boom {MY ERR}`
    assert_eq!(
        run("set th throw; catch {$th {MY ERR} boom} m o; list $m [dict get $o -errorcode]").1,
        "boom {MY ERR}",
    );
    // tclsh: `type must be non-empty list`
    let (ok, msg, _) = run("set th throw; $th {} msg");
    assert!(!ok);
    assert_eq!(msg, "type must be non-empty list");
    // tclsh: `wrong # args: should be "throw type message"`
    let (ok, msg, _) = run("set th throw; $th");
    assert!(!ok);
    assert_eq!(msg, "wrong # args: should be \"throw type message\"");
}

/// The LITERAL `try { body } on error {vars} { handler }` form must route
/// through `cmd_try`. The analysis lowering emits the bytecode exception-range
/// opcode (`beginCatch`) for it, which the VM does not implement; the *bytecode*
/// lowering the real VM runtime uses (`lower_to_ir_for_bytecode`, mirrored by
/// this file's `run`) instead drops it to a runtime-command barrier, so the
/// handler runs. Both literal and dynamic forms must preserve the handler
/// and `finally` completion semantics.
///
/// tclsh 8.6/9.0:  `try { error oops } on error {m o} { set m }`  ->  result "oops" (rc 0)
///
#[test]
fn try_literal_form_on_error_handler() {
    // tclsh: `oops` (the handler runs and returns the body's message)
    let (ok, msg, _) = run("try { error oops } on error {m o} { set m }");
    assert!(
        ok,
        "literal try/on-error should run the handler, got error: {msg}",
    );
    assert_eq!(msg, "oops");
}

// try / throw — additional error-path coverage  (cmd_try.rs)

/// A hard *parse* error in a `try` body (not just a runtime error) is mapped to
/// an error completion that a handler can catch — the `eval_body` `Err(e)` arm.
#[test]
fn try_body_parse_error_is_catchable() {
    // `set "x"yyy` is a hard parse error (text after a close-quote); the
    // `on error` handler runs.
    // tclsh: result "h" (the handler ran)
    assert_eq!(
        run("set t try; $t { set \"x\"yyy 1 } on error {} { set r h }").1,
        "h",
    );
}

/// A handler's variable bind that *fails* (here: binding into a constant, a Tcl
/// 9 feature the VM targets) becomes the handler outcome (skipping the body) and
/// chains the body's exception via `-during` (C's `handlerFailed`).
#[test]
fn try_handler_var_bind_failure_chains_during() {
    // Binding the result var into a constant `c` fails.
    // tclsh 9.0: `can't set "c": variable is a constant`
    let (ok, msg, _) = run("const c 1; set t try; $t { error body } on error {c} { set r ran }");
    assert!(!ok);
    assert_eq!(msg, "can't set \"c\": variable is a constant");
    // The bind failure chains the body's exception as `-during` (-code 1).
    // tclsh 9.0: `1`
    assert_eq!(
        run(concat!(
            "const c 1; set t try\n",
            "catch {$t { error body } on error {c} { set r ran }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

/// `add_during` REPLACES an existing `-during` rather than duplicating it: a
/// handler throws (chaining the body as `-during`), then a `finally` throws,
/// whose `add_during` over the same options must overwrite the link. The
/// surviving `-during` still resolves to the body's `-code` (1).
#[test]
fn try_add_during_replaces_existing() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set t try\n",
            "catch {$t { error body } on error {} { error handler } finally { error fin }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

/// `exit` inside a `try` body is not catchable (C's `Tcl_Exit`): the unwind
/// propagates *without* running any handler or the `finally` clause.
#[test]
fn try_exit_skips_handlers_and_finally() {
    // The finally must NOT run (no "FIN" on stdout) and an on-error handler must
    // not catch the exit. (tclsh actually terminates the process here with the
    // exit code; the VM models the unwind, but the observable — finally skipped —
    // matches.)
    let (ok, _msg, out) =
        run("set t try; $t { exit 3 } on error {} { set r h } finally { puts FIN }");
    assert!(!ok, "exit should propagate as a non-ok completion");
    assert_eq!(
        out, "",
        "neither the handler nor finally should run on exit"
    );
}

/// `throw` whose `type` is not a well-formed list errors (the `ty.as_list()` Err
/// arm in `cmd_throw`).
#[test]
fn throw_bad_type_list() {
    // An unbalanced brace makes the type an invalid list.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set th throw; $th \"a \\{ b\" msg");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// `add_during` REPLACES (does not duplicate) a `-during` key that is already
/// present in the options dict it is given. Three nested `try`s: the outer body
/// errors; the outer handler runs a nested `try` whose own handler throws
/// (yielding an error that already carries `-during`); chaining that as the
/// outer outcome's `-during` must overwrite the inner link in place. The
/// surviving `-during` still resolves to the body's `-code` (1).
#[test]
fn try_add_during_replaces_in_nested_chain() {
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set t try\n",
            "catch {$t { error obody } on error {} \
                 { $t { error ibody } on error {} { error ihandler } }} m o\n",
            "dict get [dict get $o -during] -code"
        ))
        .1,
        "1",
    );
}

// The VM must not diverge from tclsh on the following, each asserting the
// correct tclsh behaviour.

/// `Vm::set_var`, via `cmd_try`'s `bind_handler_vars`: binding a
/// `try` handler's result/options variable into an *array element whose base is
/// a scalar* (`x(y)` while `x` is a scalar) should fail with
/// `can't set "x(y)": variable isn't array` (C's `handlerFailed` → the bind
/// error becomes the handler outcome, skipping the body). The `set` *command*
/// already enforces this (a bare `set x(y) 5` errors correctly — verified), but
/// the internal `Vm::set_var` that `bind_handler_vars` calls does not, so the
/// bind silently succeeds and the handler body runs.
///
/// tclsh 8.6/9.0:  result is the error `can't set "x(y)": variable isn't array`.
///
/// Fixed: `bind_handler_vars` now writes through `Vm::var_set`, which resolves
/// `x(y)` to the array element and fails on a scalar base (guards regression).
#[test]
fn try_handler_var_bind_array_on_scalar_should_fail() {
    let (ok, msg, _) = run("set t try; set x 1; $t { error e } on error {x(y)} { set r ran }");
    assert!(
        !ok,
        "binding handler result into x(y) while x is scalar should fail, got ok with: {msg}",
    );
    assert_eq!(msg, "can't set \"x(y)\": variable isn't array");
}
