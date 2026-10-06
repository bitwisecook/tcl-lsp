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

//! Selected Tcl frames retain their physical owners across coroutine suspension.
//! Expected bytes were measured independently on C Tcl 8.6, 9.0 and 9.1.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::DialectProfile;
use tcl_vm::Vm;

fn diagnose_entry_stage(source: &str, profile: &'static DialectProfile) {
    let mut vm = Vm::default();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let segments = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
    );
    for segment in segments {
        let span = segment.execution_span(source);
        let command = &source[span.start() as usize..span.end() as usize];
        match vm.try_eval_source(command) {
            Ok(completion) => eprintln!(
                "natural entry {command:?}: {:?} {:?}",
                completion.code,
                completion.result.to_str()
            ),
            Err(error) => {
                eprintln!("natural entry {command:?}: {error:?}");
                break;
            }
        }
    }
}

fn check(source: &str, expected: &str) {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = Vm::default();
        let profile = DialectProfile::find(dialect).unwrap();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let completion = vm.try_eval_source(source).unwrap_or_else(|error| {
            diagnose_entry_stage(source, profile);
            panic!("{dialect}: {error:?}")
        });
        assert!(
            completion.code.is_ok(),
            "{dialect}: {}",
            completion.result.to_str()
        );
        assert_eq!(completion.result.to_str().as_ref(), expected, "{dialect}");
        // A completed/deleted flow leaves the resumer at its original root.
        let level = tcl_cmd_core::info::level(&mut vm, None).unwrap();
        assert_eq!(level.to_str().as_ref(), "0", "{dialect}");
    }
}

#[test]
fn caller_write() {
    check(
        r#"proc inner {} {uplevel 1 {yield HELLO;set x AFTER}};proc outer {} {set x BEFORE;inner;set x};list [coroutine C outer] [C]"#,
        "HELLO AFTER",
    );
}

#[test]
fn selected_proc_upvar() {
    check(
        r#"proc step {} {upvar 1 x link;yield HELLO;set link AFTER};proc inner {} {uplevel 1 {step}};proc outer {} {set x BEFORE;inner;set x};list [coroutine C outer] [C]"#,
        "HELLO AFTER",
    );
}

#[test]
fn hidden_link() {
    check(
        r#"proc inner {} {upvar 1 x link;uplevel 1 {yield HELLO;set x AFTER};list $link [info level]};proc outer {} {set x BEFORE;list [inner] $x};list [coroutine C outer] [C]"#,
        "HELLO {{AFTER 2} AFTER}",
    );
}

#[test]
fn selected_namespace() {
    check(
        r#"namespace eval N {proc inner {} {uplevel 1 {yield HELLO;set x AFTER;helper}};proc helper {} {set ::called N};proc outer {} {set x BEFORE;inner;list $x $::called [namespace current]}};list [coroutine C N::outer] [C]"#,
        "HELLO {AFTER N ::N}",
    );
}

#[test]
fn global_target() {
    check(
        r#"proc inner {} {set x LOCAL;uplevel #0 {yield HELLO;set x GLOBAL};list $x $::x [info level]};list [coroutine C inner] [C]"#,
        "HELLO {LOCAL GLOBAL 1}",
    );
}

#[test]
fn error_restores_caller() {
    check(
        r#"proc inner {} {set x INNER;set code [catch {uplevel 1 {yield HELLO;set x AFTER;error BOOM}} value];list $code $value $x [info level]};proc outer {} {set x BEFORE;list [inner] $x};list [coroutine C outer] [C]"#,
        "HELLO {{1 BOOM INNER 2} AFTER}",
    );
}

#[test]
fn return_restores_caller() {
    check(
        r#"proc inner {} {uplevel 1 {yield HELLO;return DONE};error UNREACHED};proc outer {} {set x OUTER;list [inner] $x};list [coroutine C outer] [C]"#,
        "HELLO {DONE OUTER}",
    );
}

#[test]
fn nested_selection() {
    check(
        r#"proc deep {} {uplevel #0 {yield HELLO;set ::g GLOBAL}};proc inner {} {uplevel 1 {deep;set x AFTER}};proc outer {} {set x BEFORE;inner;list $x $::g};list [coroutine C outer] [C]"#,
        "HELLO {AFTER GLOBAL}",
    );
}

#[test]
fn twice_yield() {
    check(
        r#"proc inner {} {uplevel 1 {set x ONE;yield $x;set x TWO;yield $x;set x THREE}};proc outer {} {set x BEFORE;inner;set x};list [coroutine C outer] [C] [C]"#,
        "ONE TWO THREE",
    );
}

#[test]
fn delete_hidden_owners() {
    check(
        r#"set events {};proc gone {tag args} {lappend ::events $tag};proc inner {} {set b INNER;trace add variable b unset {gone inner};uplevel 1 {yield HELLO;set a AFTER}};proc outer {} {set a OUTER;trace add variable a unset {gone outer};inner};set first [coroutine C outer];rename C {};list $first $events [info commands C]"#,
        "HELLO {inner outer} {}",
    );
}

#[test]
fn delete_nested_owners() {
    check(
        r#"set events {};proc gone {tag args} {lappend ::events $tag};proc deep {} {set c DEEP;trace add variable c unset {gone deep};uplevel #0 {yield HELLO}};proc inner {} {set b INNER;trace add variable b unset {gone inner};uplevel 1 {deep}};proc outer {} {set a OUTER;trace add variable a unset {gone outer};inner};set first [coroutine C outer];rename C {};list $first $events [info commands C]"#,
        "HELLO {deep inner outer} {}",
    );
}

#[test]
fn two_coroutines() {
    check(
        r#"proc inner {tag} {uplevel 1 {yield READY;incr x};set tag};proc outer {n} {set x $n;inner $n;set x};set a [coroutine A outer 10];set b [coroutine B outer 20];list $a $b [A] [B]"#,
        "READY READY 11 21",
    );
}

#[test]
fn shifted_error_stack_survives_suspension() {
    check(
        r#"proc inner {} {catch {uplevel 1 {yield HELLO;error BOOM}} result options;set stack [dict get $options -errorstack];list $result [lrange $stack 0 3]};proc outer {} {inner};list [coroutine C outer] [C]"#,
        "HELLO {BOOM {INNER {returnImm BOOM {}} UP 1}}",
    );
}

#[test]
fn selected_namespace_token_survives_namespace_deletion() {
    check(
        r#"namespace eval N {proc inner {} {uplevel 1 {yield HELLO;set x AFTER}};proc outer {} {set x BEFORE;inner;set x}};set first [coroutine C N::outer];namespace delete N;list $first [C] [namespace exists N]"#,
        "HELLO AFTER 0",
    );
}

#[test]
fn malformed_body_preserves_prefix_writes_and_restores_caller() {
    check(
        r#"proc inner {} {set x INNER;set body {set x TARGET; "};set code [catch {uplevel 1 $body} result];list $code $x [info level]};proc outer {} {set x BEFORE;list [inner] $x};outer"#,
        "{1 INNER 2} TARGET",
    );
}

#[test]
fn malformed_tail_after_yield_preserves_selected_prefix_and_restores_caller() {
    check(
        r#"proc inner {} {set x INNER;set body {yield HELLO;set x TARGET; "};set code [catch {uplevel 1 $body} result];list $code $x [info level]};proc outer {} {set x BEFORE;list [inner] $x};list [coroutine C outer] [C]"#,
        "HELLO {{1 INNER 2} TARGET}",
    );
}

#[test]
fn caught_quoted_error_after_yield_reads_the_selected_live_cell() {
    check(
        r#"proc inner {} {uplevel 1 {set code [catch {set x [yield a]; error "boom-$x"} result]; list $code $result $x [info level]}};proc outer {} {set x BEFORE;inner};list [coroutine C outer] [C b]"#,
        "a {1 boom-b b 1}",
    );
}
