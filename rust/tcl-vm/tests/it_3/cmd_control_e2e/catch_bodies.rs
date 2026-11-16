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

//! `catch` and nested `try` bodies: command order, stale-command replay, result and options variables, and dynamic words the compiler declines.

use super::*;

/// A literal `catch` in a procedure may inline its exception range, but its
/// body is still a complete Tcl script. Commands after a separator must not be
/// folded into the first command's argument list; the first command's local
/// write is visible to the later error-producing command.
#[test]
fn literal_catch_multicommand_proc_body_executes_in_order() {
    // tclsh 9.0.4: `1 boom-ok`
    assert_eq!(
        run(
            "proc p {} {set code [catch {set x ok; error boom-$x} message]; list $code $message}; p"
        )
        .1,
        "1 boom-ok",
    );
}

/// The catch argument parser already removes its own outer word delimiters.
/// Braces at the beginning and end of the resulting script value belong to
/// command words and must survive inline lowering.
#[test]
fn literal_catch_preserves_braced_first_and_last_words() {
    // tclsh 9.0.4: `0 1 1`
    assert_eq!(
        run("proc p {} {set code [catch {{set} x {1}} r]; list $code $r $x}; p").1,
        "0 1 1",
    );
}

/// Each command in a segmented inline body owns its own replay text. If the
/// first command mutates the command table, deoptimising the second must not
/// replay the enclosing catch expression or run the mutation twice.
#[test]
fn literal_catch_replays_only_the_stale_nested_command_after_mutation() {
    // tclsh 9.0.4: `0 REPLACED 1`
    assert_eq!(
        run(
            "set ::runs 0; \
             proc mutate {} {incr ::runs; rename error saved_error; proc error args {return REPLACED}; return first}; \
             proc p {} {set code [catch {mutate; error \"boom\"} result]; list $code $result $::runs}; p"
        )
        .1,
        "0 REPLACED 1",
    );
}

/// Diagnostic line metadata changes as a multiline inline body advances, but
/// returning from its final nested command to the synthetic catch epilogue is
/// not a new execution of the enclosing source command. Only explicit command
/// boundaries may trigger stale-bytecode replay.
#[test]
fn multiline_catch_epilogue_does_not_replay_the_enclosing_command() {
    // tclsh 9.0.4: `0 REPLACED 1`
    assert_eq!(
        run(
            "set ::runs 0\n\
             proc mutate {} {incr ::runs; rename error saved_error; proc error args {return REPLACED}; return first}\n\
             proc p {} {\n\
                 set code [catch {\n\
                     mutate\n\
                     error boom\n\
                 } result]\n\
                 list $code $result $::runs\n\
             }\n\
             p"
        )
        .1,
        "0 REPLACED 1",
    );
}

/// The narrow nested `catch {try ... on error ...}` compiler specialisation
/// keeps distinct live ranges for the try body and handler. Both are complete
/// scripts: body errors reach the handler, successful handler results reach the
/// outer catch, and body success bypasses the handler.
#[test]
fn nested_catch_try_multicommand_phases_preserve_results() {
    // tclsh 9.0.4: `0 boom-ok/handled`
    assert_eq!(
        run(
            "proc p {} {set code [catch {try {set x ok; error boom-$x} on error message {set suffix handled; set message $message/$suffix}} result]; list $code $result}; p"
        )
        .1,
        "0 boom-ok/handled",
    );
    // tclsh 9.0.4: `0 ok`
    assert_eq!(
        run(
            "proc p {} {set code [catch {try {set x ok; set x} on error message {error unexpected}} result]; list $code $result}; p"
        )
        .1,
        "0 ok",
    );
}

/// The nested inline shape uses Tcl's list grammar for `variableList` and
/// binds the body's result and options separately, like Tcl 9.0.4.
#[test]
fn nested_catch_try_binds_result_and_options_variables() {
    // tclsh 9.0.4: `0 boom`
    assert_eq!(
        run(
            "proc p {} {set code [catch {try {error boom} on error {m o} {set m}} result]; list $code $result}; p"
        )
        .1,
        "0 boom",
    );
    // tclsh 9.0.4: `0 {boom 1}`
    assert_eq!(
        run(
            "proc p {} {set code [catch {try {error boom} on error {m o} {list $m [dict get $o -code]}} result]; list $code $result}; p"
        )
        .1,
        "0 {boom 1}",
    );
}

/// Substituted catch/try phase words must retain Tcl's pre-invocation word
/// substitution. The literal specialisers decline these shapes and the shared
/// explicit-stack runtime commands evaluate their resulting script values.
#[test]
fn catch_and_nested_try_decline_dynamic_phase_words() {
    // tclsh 9.0.4: `1 boom`
    assert_eq!(
        run("proc p {} {set body {error boom}; set code [catch $body result]; list $code $result}; p").1,
        "1 boom",
    );
    // tclsh 9.0.4: `0 {boom 1}` for a substituted try body.
    assert_eq!(
        run(
            "proc p {} {set body {error boom}; set code [catch {try $body on error {m o} {list $m [dict get $o -code]}} result]; list $code $result}; p"
        )
        .1,
        "0 {boom 1}",
    );
    // tclsh 9.0.4: `0 {boom 1}` for a substituted handler body.
    assert_eq!(
        run(
            "proc p {} {set handler {list $m [dict get $o -code]}; set code [catch {try {error boom} on error {m o} $handler} result]; list $code $result}; p"
        )
        .1,
        "0 {boom 1}",
    );
    // tclsh 9.0.4: `0 {boom 1}` for a substituted handler variable list.
    assert_eq!(
        run(
            "proc p {} {set vars {m o}; set code [catch {try {error boom} on error $vars {list $m [dict get $o -code]}} result]; list $code $result}; p"
        )
        .1,
        "0 {boom 1}",
    );
}

/// A braced handler body may still contain a dynamic variable-name word. The
/// handler's typed `set` lowering hook is not enough to use `STORE_SCALAR1`:
/// Tcl substitutes `$target` before selecting the variable to write.
#[test]
fn nested_try_handler_declines_dynamic_set_target() {
    // tclsh 9.0.4: `0 changed changed`
    assert_eq!(
        run(
            "proc p {} {set target m; set m outer; set code [catch {try {error boom} on error {m o} {set $target changed}} result]; list $code $result $m}; p"
        )
        .1,
        "0 changed changed",
    );
}

/// A quoted catch body has already undergone Tcl's word substitution. It must
/// stay on the generic command path even when its current text happens to be
/// static.
#[test]
fn catch_declines_unbraced_script_body() {
    // tclsh 9.0.4: "1 boom"
    assert_eq!(
        run("proc p {} {set code [catch \"error boom\" result]; list $code $result}; p").1,
        "1 boom",
    );
}
