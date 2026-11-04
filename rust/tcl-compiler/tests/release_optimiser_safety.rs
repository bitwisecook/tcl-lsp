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

//! Deterministic release regressions for source optimiser safety.

use tcl_compiler::optimiser::manager::optimise_source_multipass_admitting_standalone_with_analysis_input;
use tcl_registry::model::ingress::resolve_environment;

fn optimise(source: &str, dialect: &str) -> (String, Vec<tcl_compiler::optimiser::Optimisation>) {
    let environment = resolve_environment(dialect);
    let context = environment.default_context_registry();
    let registry = std::sync::Arc::clone(context.commands());
    let profile = environment.analyser_profile();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let (text, applied, _) = optimise_source_multipass_admitting_standalone_with_analysis_input(
        source,
        &registry,
        Some(&input),
        8,
        |_, candidates| candidates,
    );
    (text, applied)
}

fn observed(tclsh: &std::path::Path, source: &str) -> tcl_test_support::ScriptOutcome {
    let word = tcl_compiler::codegen::helpers::tcl_list_element(source);
    let script =
        format!("set completion [catch {word} message]\nputs [list $completion $message]\n");
    tcl_test_support::run_script(tclsh, script.as_bytes()).expect("the Tcl oracle runs")
}

fn preserves(source: &str, first: &str) {
    for series in ["8.4", "8.5", "8.6", "9.0", "9.1"] {
        if series < first {
            continue;
        }
        let version =
            tcl_dialect::TclVersion::from_version_string(series).expect("modelled release");
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let (rewritten, rewrites) = optimise(source, &format!("tcl{series}"));
        let summaries: Vec<_> = rewrites
            .iter()
            .map(|rewrite| (rewrite.code, rewrite.span, &rewrite.replacement))
            .collect();
        assert_eq!(
            observed(&tclsh.path, source),
            observed(&tclsh.path, &rewritten),
            "Tcl {series}:\noriginal:\n{source}\nrewritten:\n{rewritten}\n{summaries:?}"
        );
    }
}

#[test]
fn issue_2217() {
    preserves(
        r###"namespace eval ::n {}
proc ::n::expr {s} {return "SHADOW:$s"}
proc ::n::f {r} {return [expr {$r ** 2}]}
puts [::n::f 3]
"###,
        "8.4",
    );
}

#[test]
fn issue_2231() {
    preserves(
        r###"set x 1
set c [catch {incr x}]
if {$x == 2} {puts two} else {puts "not two: $x"}
"###,
        "8.4",
    );
}

#[test]
fn issue_2233() {
    preserves(
        r###"set d {a 1}
tcl::dict::set d k v
puts $d
"###,
        "8.5",
    );
}

#[test]
fn issue_2234() {
    preserves(
        r###"proc p {} {return [expr {10000000000000002.0}]}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2248() {
    preserves(
        r###"set s [format %c 233]
puts $s
"###,
        "8.4",
    );
}

#[test]
fn issue_2258() {
    preserves(
        r###"proc p {} {set s "\{"; set y [lindex $s 0]; puts hi}
puts [catch p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2260() {
    preserves(
        r###"proc p {} {set {} 5; puts [set {}]}
p
"###,
        "8.4",
    );
}

#[test]
fn issue_2261() {
    preserves(
        r###"proc p {} {set x 1; uplevel 0 {puts $x}; set x 2; return $x}
p
"###,
        "8.4",
    );
}

#[test]
fn issue_2262() {
    preserves(
        r###"proc p {} {set n 1; foreach v [incr n] {}; puts $n}
p
"###,
        "8.4",
    );
}

#[test]
fn issue_2263() {
    preserves(
        r###"proc p {} {set x 1; puts [unset x]; puts $x}
puts [catch p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2264() {
    preserves(
        r###"proc p {} {set y 5; set y}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2267() {
    preserves(
        r###"proc p {} {set n abc; incr n}
puts [catch p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2307() {
    preserves(
        r###"proc p {} {set go 0; for {} {$go} {} {puts x}; incr go}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2308() {
    preserves(
        r###"set x 5
after 0 {incr x}
vwait x
puts $x
"###,
        "8.4",
    );
}

#[test]
fn issue_2327() {
    preserves(
        r###"set argv other
set a 1
switch -glob -- $argv x {} default {set a 2; if {$a == 1} {puts one} else {puts two}}
"###,
        "8.4",
    );
}

#[test]
fn issue_2334() {
    preserves(
        r###"expr {1 + 2}
"###,
        "8.4",
    );
}

#[test]
fn issue_2339() {
    preserves(
        r###"proc p {} {set i 0; set a([incr i]) 5; if {$i == 0} {return zero} else {return nonzero}}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2340() {
    preserves(
        r###"proc p {} {set r 1; return "$r x"}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2347() {
    preserves(
        r###"proc p {} {try {error boom} on error {} {puts first} on 1 {m} {puts second}; puts after}
p
"###,
        "8.6",
    );
}

#[test]
fn issue_2348() {
    preserves(
        r###"proc p {} {try {return r3; set v 8} on return {} {puts t8}; puts after}
p
"###,
        "8.6",
    );
}

#[test]
fn issue_2350() {
    preserves(
        r###"set y 0; set y(c) 3; puts [array exists y]
"###,
        "8.4",
    );
}

#[test]
fn issue_2351() {
    preserves(
        r###"proc p {} {catch {set y 1; set y(a) 2} m; return $m}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2352() {
    preserves(
        r###"proc p {} {set m boom; puts "e=[expr {$m eq {}}]"}
p
"###,
        "8.4",
    );
}

#[test]
fn issue_2382() {
    preserves(
        r###"for {set i 0} {$i < 1} {incr i} {set r q}
puts $r
"###,
        "8.4",
    );
}

#[test]
fn issue_2383() {
    preserves(
        r###"set c 1
if {$c} {for {set j 0} {$j < 3} {incr j} {}}
puts $j
"###,
        "8.4",
    );
}

#[test]
fn issue_2385() {
    preserves(
        r###"set i 0
foreach ::x {1 2} {incr i $::x}
puts $i
"###,
        "8.4",
    );
}

#[test]
fn issue_2386() {
    preserves(
        r###"proc p {l} {set t 0; foreach x $l {incr t $x}; return $t}
puts [p {1 2 3}]
"###,
        "8.4",
    );
}

#[test]
fn issue_2390() {
    preserves(
        r###"proc p {} {expr {1/0}; return foo}
puts [catch p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2391() {
    preserves(
        r###"proc p {} {set x 5; return {$x}}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2394() {
    preserves(
        r###"proc p {} {set x [expr {[return 7] + 1}]; return 2}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2396() {
    preserves(
        r###"proc p {} {set x "[string length ab][string length cde]"; return $x}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2397() {
    preserves(
        r###"array set a {k v}
array for {k v} a {puts "$k=$v"}
"###,
        "9.0",
    );
}

#[test]
fn issue_2398() {
    preserves(
        r###"proc p {} {set x [string range foobar 0 2]; return $x}
puts [p]
proc string {args} {return SHADOW}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2399() {
    preserves(
        r###"proc b6 {} {global g; return $g}
set g 6
set r [b6]; puts "<$r>"
set g 123
set r [b6]; puts "<$r>"
unset g
"###,
        "8.4",
    );
}

#[test]
fn issue_2400() {
    preserves(
        r###"proc p {} {set x [expr {10 ** 2}]; return $x}
set r [p]
puts "<$r>"
"###,
        "8.4",
    );
}

#[test]
fn issue_2403() {
    preserves(
        r###"proc p {} {for {return 1} {1} {} {}; return none}
puts [p]
"###,
        "8.4",
    );
}

#[test]
fn issue_2404() {
    preserves(
        r###"proc p {} {foreach ::x {1} {}; return 2}
puts [p]
puts $::x
"###,
        "8.4",
    );
}

#[test]
fn issue_2423() {
    preserves(
        r###"proc ::tcl::dict::size {d} {return 7}
puts [dict size {a 1}]
puts [::tcl::dict::size {a 1}]
"###,
        "8.5",
    );
}

#[test]
fn issue_2424() {
    preserves(
        r###"proc ip::version {a} {return shadowed}
puts [ip::version 1.2.3.4]
"###,
        "8.4",
    );
}

#[test]
fn issue_2430() {
    preserves(
        r###"puts [string cat a b c]
"###,
        "8.4",
    );
}

#[test]
fn issue_2431() {
    preserves(
        r###"puts [expr {0o17 + 1_000}]
"###,
        "8.4",
    );
}

#[test]
fn issue_2433() {
    preserves(
        r###"puts {[string length abc]}
"###,
        "8.4",
    );
}

#[test]
fn issue_2434() {
    preserves(
        r###"puts "\x41[string length abc]"
"###,
        "8.4",
    );
}

#[test]
fn issue_2442() {
    preserves(
        r###"proc p {} {set {a b} 3; return ${a b}}
puts [p]
"###,
        "8.4",
    );
}
