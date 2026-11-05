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

fn analyse(source: &str, dialect: &str) -> tcl_compiler::analyser::AnalysisResult {
    let environment = resolve_environment(dialect);
    let profile = environment.analyser_profile();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        environment.default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let result = tcl_compiler::analyser::Analyser::new()
        .with_resolved_input(input)
        .analyse(source, dialect);
    assert!(result.analysis_context_unavailable.is_none());
    assert!(result.resolved_input.is_some());
    result
}

fn diagnostic_summaries(
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> Vec<(tcl_core_types::DiagCode, tcl_lexer::Span, &str)> {
    analysis
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span,
                diagnostic.message.as_str(),
            )
        })
        .collect()
}

fn observed(tclsh: &std::path::Path, source: &str) -> tcl_test_support::ScriptOutcome {
    let word = tcl_codegen::codegen::helpers::tcl_list_element(source);
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
        r#"namespace eval ::n {}
proc ::n::expr {s} {return "SHADOW:$s"}
proc ::n::f {r} {return [expr {$r ** 2}]}
puts [::n::f 3]
"#,
        "8.4",
    );
}

#[test]
fn issue_2231() {
    preserves(
        r#"set x 1
set c [catch {incr x}]
if {$x == 2} {puts two} else {puts "not two: $x"}
"#,
        "8.4",
    );
}

#[test]
fn issue_2233() {
    preserves(
        r"set d {a 1}
tcl::dict::set d k v
puts $d
",
        "8.5",
    );
}

#[test]
fn issue_2234() {
    preserves(
        r"proc p {} {return [expr {10000000000000002.0}]}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2248() {
    preserves(
        r"set s [format %c 233]
puts $s
",
        "8.4",
    );
}

#[test]
fn issue_2258() {
    preserves(
        r#"proc p {} {set s "\{"; set y [lindex $s 0]; puts hi}
puts [catch p]
"#,
        "8.4",
    );
}

#[test]
fn issue_2260() {
    preserves(
        r"proc p {} {set {} 5; puts [set {}]}
p
",
        "8.4",
    );
}

#[test]
fn issue_2261() {
    preserves(
        r"proc p {} {set x 1; uplevel 0 {puts $x}; set x 2; return $x}
p
",
        "8.4",
    );
}

#[test]
fn issue_2262() {
    preserves(
        r"proc p {} {set n 1; foreach v [incr n] {}; puts $n}
p
",
        "8.4",
    );
}

#[test]
fn issue_2263() {
    preserves(
        r"proc p {} {set x 1; puts [unset x]; puts $x}
puts [catch p]
",
        "8.4",
    );
}

#[test]
fn issue_2264() {
    preserves(
        r"proc p {} {set y 5; set y}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2267() {
    preserves(
        r"proc p {} {set n abc; incr n}
puts [catch p]
",
        "8.4",
    );
}

#[test]
fn issue_2307() {
    preserves(
        r"proc p {} {set go 0; for {} {$go} {} {puts x}; incr go}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2308() {
    preserves(
        r"set x 5
after 0 {incr x}
vwait x
puts $x
",
        "8.4",
    );
}

#[test]
fn issue_2327() {
    preserves(
        r"set argv other
set a 1
switch -glob -- $argv x {} default {set a 2; if {$a == 1} {puts one} else {puts two}}
",
        "8.4",
    );
}

#[test]
fn issue_2334() {
    preserves(
        r"expr {1 + 2}
",
        "8.4",
    );
}

#[test]
fn issue_2339() {
    preserves(
        r"proc p {} {set i 0; set a([incr i]) 5; if {$i == 0} {return zero} else {return nonzero}}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2340() {
    preserves(
        r#"proc p {} {set r 1; return "$r x"}
puts [p]
"#,
        "8.4",
    );
}

#[test]
fn issue_2347() {
    preserves(
        r"proc p {} {try {error boom} on error {} {puts first} on 1 {m} {puts second}; puts after}
p
",
        "8.6",
    );
}

#[test]
fn issue_2348() {
    preserves(
        r"proc p {} {try {return r3; set v 8} on return {} {puts t8}; puts after}
p
",
        "8.6",
    );
}

#[test]
fn issue_2350() {
    preserves(
        r"set y 0; set y(c) 3; puts [array exists y]
",
        "8.4",
    );
}

#[test]
fn issue_2351() {
    preserves(
        r"proc p {} {catch {set y 1; set y(a) 2} m; return $m}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2352() {
    preserves(
        r#"proc p {} {set m boom; puts "e=[expr {$m eq {}}]"}
p
"#,
        "8.4",
    );
}

#[test]
fn issue_2382() {
    preserves(
        r"for {set i 0} {$i < 1} {incr i} {set r q}
puts $r
",
        "8.4",
    );
}

#[test]
fn issue_2383() {
    preserves(
        r"set c 1
if {$c} {for {set j 0} {$j < 3} {incr j} {}}
puts $j
",
        "8.4",
    );
}

#[test]
fn issue_2385() {
    preserves(
        r"set i 0
foreach ::x {1 2} {incr i $::x}
puts $i
",
        "8.4",
    );
}

#[test]
fn issue_2386() {
    preserves(
        r"proc p {l} {set t 0; foreach x $l {incr t $x}; return $t}
puts [p {1 2 3}]
",
        "8.4",
    );
}

#[test]
fn issue_2390() {
    preserves(
        r"proc p {} {expr {1/0}; return foo}
puts [catch p]
",
        "8.4",
    );
}

#[test]
fn issue_2391() {
    preserves(
        r"proc p {} {set x 5; return {$x}}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2394() {
    preserves(
        r"proc p {} {set x [expr {[return 7] + 1}]; return 2}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2396() {
    preserves(
        r#"proc p {} {set x "[string length ab][string length cde]"; return $x}
puts [p]
"#,
        "8.4",
    );
}

#[test]
fn issue_2397() {
    preserves(
        r#"array set a {k v}
array for {k v} a {puts "$k=$v"}
"#,
        "9.0",
    );
}

#[test]
fn issue_2398() {
    preserves(
        r"proc p {} {set x [string range foobar 0 2]; return $x}
puts [p]
proc string {args} {return SHADOW}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2399() {
    preserves(
        r#"proc b6 {} {global g; return $g}
set g 6
set r [b6]; puts "<$r>"
set g 123
set r [b6]; puts "<$r>"
unset g
"#,
        "8.4",
    );
}

#[test]
fn issue_2400() {
    preserves(
        r#"proc p {} {set x [expr {10 ** 2}]; return $x}
set r [p]
puts "<$r>"
"#,
        "8.4",
    );
}

#[test]
fn issue_2403() {
    preserves(
        r"proc p {} {for {return 1} {1} {} {}; return none}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_2404() {
    preserves(
        r"proc p {} {foreach ::x {1} {}; return 2}
puts [p]
puts $::x
",
        "8.4",
    );
}

#[test]
fn issue_2423() {
    preserves(
        r"proc ::tcl::dict::size {d} {return 7}
puts [dict size {a 1}]
puts [::tcl::dict::size {a 1}]
",
        "8.5",
    );
}

#[test]
fn issue_2424() {
    preserves(
        r"proc ip::version {a} {return shadowed}
puts [ip::version 1.2.3.4]
",
        "8.4",
    );
}

#[test]
fn issue_2430() {
    preserves(
        r"puts [string cat a b c]
",
        "8.4",
    );
}

#[test]
fn issue_2431() {
    preserves(
        r"puts [expr {0o17 + 1_000}]
",
        "8.4",
    );
}

#[test]
fn issue_2433() {
    preserves(
        r"puts {[string length abc]}
",
        "8.4",
    );
}

#[test]
fn issue_2434() {
    preserves(
        r#"puts "\x41[string length abc]"
"#,
        "8.4",
    );
}

#[test]
fn issue_2442() {
    preserves(
        r"proc p {} {set {a b} 3; return ${a b}}
puts [p]
",
        "8.4",
    );
}

#[test]
fn issue_1797() {
    preserves(
        r"namespace eval ::n [list proc writer {} {upvar 1 x y; ::set y 2}]
proc caller {} {set x 1; ::n::writer; return $x}
puts [caller]
proc writer {} {upvar 1 x y; ::set y 3}
rename writer old
proc writer {name} {upvar 1 $name y; ::set y 4}
proc generations {} {set x 0; old; puts $x; writer x; return $x}
puts [generations]
namespace eval ::n [list proc global_writer {} {global g; ::set g 8}]
set g 1
::n::global_writer
puts $g
",
        "8.4",
    );
}

#[test]
fn issue_2216() {
    let source = "proc p {} {\n  set a 1   \n  set a 2\n  return $a\n}\nputs [p]\n";
    let (rewritten, _) = optimise(source, "tcl8.6");
    assert_ne!(rewritten, source);
    assert!(!rewritten.contains("   \n"), "{rewritten}");
    preserves(source, "8.4");
}

#[test]
fn issue_2218() {
    let source = "proc double_expr {x} {return [expr {[expr {$x * 2}]}]}\nproc g {cmd} {$cmd}\n";
    let (_, applied) = optimise(source, "tcl8.6");
    assert!(
        applied
            .iter()
            .all(|row| row.code != tcl_core_types::DiagCode::O115),
        "a dynamic procedure head cannot justify nested expr identity"
    );
}

#[test]
fn issue_2373_and_2431_release_less_numerals() {
    for source in [
        "proc q {x} {if {$x == \"1_000\"} {set y $x; puts \"<$y>\"} else {puts no}}\nq 1000\n",
        "proc p {} {set s 1_000; if {$s == 1000} {puts yes} else {puts no}}\np\n",
        "set n 0d19\nif {$n == 19} {puts yes} else {puts no}\n",
        "set n 0b101\nif {$n == 5} {puts yes} else {puts no}\n",
    ] {
        let (rewritten, _) = optimise(source, "tcl");
        for version in tcl_dialect::TclVersion::ALL {
            let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
                continue;
            };
            assert_eq!(
                observed(&tclsh.path, source),
                observed(&tclsh.path, &rewritten),
                "{version:?}:\n{source}\n{rewritten}"
            );
        }
    }
}

#[test]
fn issue_2411() {
    preserves(
        "proc down {n} {if {$n > 0} {puts $n; down [expr {$n - 1}]}}\ndown 3\n",
        "8.4",
    );
    preserves(
        "proc up {name k} {upvar 1 $name v; incr v; if {$k > 1} {up v [expr {$k - 1}]}}\nset x 0\nup x 3\nputs $x\n",
        "8.4",
    );
}

#[test]
fn issue_2417() {
    for source in [
        "set g 1\nproc top {} {}\n",
        "set g 1\ntrace add variable g write {apply {{args} {}}}\nproc top {} {}\n",
    ] {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            assert!(result.analysis_context_unavailable.is_none());
        }
    }
}

#[test]
fn issue_2358_2406_and_2439_applied_reports() {
    for source in [
        "set x 5\n",
        "set x 1\nset r [expr {$x + [incr x] + $x}]\nputs \"$r $x\"\n",
        "set n 1\nincr n\nputs \"x $n\"\n",
        "set x 5\nexpr {$x + 1}\nset dir /tmp\nset f r.txt\nset p \"$dir/$f\"\nputs $p\n",
    ] {
        let (rewritten, applied) = optimise(source, "tcl8.6");
        assert!(applied.iter().all(|row| !row.hint_only));
        if rewritten == source {
            assert_eq!(applied.len(), 0, "unchanged source has no applied rewrites");
        }
        preserves(source, "8.4");
    }
    let (rewritten, applied) = optimise("expr {1 + 2}\n", "tcl8.6");
    assert_eq!(rewritten, "expr 3\n");
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].code, tcl_core_types::DiagCode::O101);
}

#[test]
fn issue_2341() {
    for (condition, excluded) in [(0, "consequent"), (1, "alternate")] {
        let source = format!("set n {condition}\nif {{$n}} {{puts then}} else {{puts else}}\n");
        let result = tcl_compiler::analyser::Analyser::new().analyse(&source, "tcl8.6");
        let messages: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::I230)
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains(excluded), "{messages:?}");
    }
}

#[test]
fn issue_2384_computed_false_loop_is_reported() {
    let control = analyse("for {set i 0} {$i < 0} {incr i} {puts never}\n", "tcl8.6");
    assert!(
        control
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == tcl_core_types::DiagCode::W240 }),
        "{:?}",
        diagnostic_summaries(&control)
    );
}

#[test]
fn issue_2384() {
    for body in [
        "set i 5",
        "set i 100",
        "incr i 50",
        "set j [incr i 5]",
        "foreach i {100} {}",
    ] {
        let source = format!("for {{set i 0}} {{$i < 3}} {{incr i}} {{{body}}}\nputs $i\n");
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = analyse(&source, dialect);
            assert!(
                result
                    .diagnostics
                    .iter()
                    .all(|diagnostic| { diagnostic.code != tcl_core_types::DiagCode::W240 }),
                "{source}: {:?}",
                diagnostic_summaries(&result)
            );
        }
    }
}

#[test]
fn issue_2384_literal_false_loop_is_still_reported() {
    for source in [
        "while 0 {puts never}\n",
        "for {} 0 {} {puts never}\n",
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub spin {cond:expr body:body} -loop\n# tcl-lsp: stubs-end\nspin 0 {puts never}\n",
    ] {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let result = analyse(source, dialect);
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == tcl_core_types::DiagCode::W240),
                "{dialect}: {source}: {:?}",
                diagnostic_summaries(&result)
            );
        }
    }
}

#[test]
fn issue_2384_shadowed_loop_heads_are_not_reported() {
    for (name, parameters, arguments) in [
        ("while", "condition body", "0 {puts never}"),
        ("while", "condition body", "1 {puts never}"),
        ("for", "start test next body", "{} 0 {} {puts never}"),
        ("for", "start test next body", "{} 1 {} {puts never}"),
    ] {
        let source = format!("proc {name} {{{parameters}}} {{puts always}}\n{name} {arguments}\n");
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let result = analyse(&source, dialect);
            assert!(
                result.diagnostics.iter().all(|diagnostic| !matches!(
                    diagnostic.code,
                    tcl_core_types::DiagCode::W240
                        | tcl_core_types::DiagCode::W241
                        | tcl_core_types::DiagCode::W242
                )),
                "{dialect}: {source}: {:?}",
                diagnostic_summaries(&result)
            );
        }
    }
}

#[test]
fn issue_2384_qualified_and_alias_loop_heads() {
    for (source, warns) in [
        ("::while 0 {puts never}\n", true),
        ("::for {} 0 {} {puts never}\n", true),
        ("interp alias {} spin {} while\nspin 0 {puts never}\n", true),
        ("interp alias {} spin {} while 0\nspin {puts never}\n", true),
        (
            "proc ::while {condition body} {puts always}\n::while 0 {puts never}\n",
            false,
        ),
        (
            "proc ::for {start test next body} {puts always}\n::for {} 0 {} {puts never}\n",
            false,
        ),
        (
            "proc route {condition body} {puts always}\nrename while core_while\ninterp alias {} while {} route\nwhile 0 {puts never}\n",
            false,
        ),
    ] {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let result = analyse(source, dialect);
            let warnings: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|diagnostic| {
                    matches!(
                        diagnostic.code,
                        tcl_core_types::DiagCode::W240
                            | tcl_core_types::DiagCode::W241
                            | tcl_core_types::DiagCode::W242
                    )
                })
                .map(|diagnostic| diagnostic.code)
                .collect();
            let expected = if warns {
                vec![tcl_core_types::DiagCode::W240]
            } else {
                Vec::new()
            };
            assert_eq!(
                warnings,
                expected,
                "{dialect}: {source}: {:?}",
                diagnostic_summaries(&result)
            );
        }
    }
}

#[test]
fn issue_2384_loop_execution() {
    for (body, condition, expected) in [
        ("set i 5", "$i < 3", "1 6"),
        ("set i 100", "$i < 3", "1 101"),
        ("incr i 50", "$i < 3", "1 51"),
        ("set j [incr i 5]", "$i < 3", "1 6"),
        ("foreach i {100} {}", "$i < 3", "1 101"),
        ("", "$i < 0", "0 0"),
    ] {
        let source = format!(
            "set count 0\nfor {{set i 0}} {{{condition}}} {{incr i}} {{incr count; {body}}}\nputs [list $count $i]\n"
        );
        let mut engines: Vec<_> = tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter_map(tcl_test_support::witness_tclsh)
            .map(|oracle| {
                (
                    format!("tcl{}", oracle.version.version_string()),
                    oracle.path,
                )
            })
            .collect();
        let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
            Some(tcl_test_support::require_jimsh().expect("required current Jim oracle"))
        } else {
            tcl_test_support::locate_jimsh().expect("validated Jim oracle")
        };
        if let Some(jim) = jim {
            engines.push(("jim".to_owned(), jim.path));
        }
        for (dialect, binary) in engines {
            let original = observed(&binary, &source);
            assert_eq!(
                String::from_utf8_lossy(&original.stdout),
                format!("{expected}\n0 {{}}\n"),
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimise(&source, &dialect);
            assert_eq!(
                original,
                observed(&binary, &rewritten),
                "{dialect}: {rewritten}"
            );
        }
    }
}

#[test]
fn issue_2402() {
    let control =
        tcl_compiler::analyser::Analyser::new().analyse("while 1 {puts running}\n", "tcl8.6");
    assert!(
        control
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == tcl_core_types::DiagCode::W241 })
    );
    for source in [
        "proc p {} {set body {return 1}; while 1 $body; return none}\nputs [p]\n",
        "set body {break}\nwhile 1 $body\nputs after\n",
        "proc p {} {for {return 1} {1} {} {}; return none}\nputs [p]\n",
    ] {
        let result = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| { diagnostic.code != tcl_core_types::DiagCode::W241 }),
            "{source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn issue_2434_escape_boundaries() {
    for source in [
        "puts \"\\u41[string length abc]\"\n",
        "puts \"\\1[string length abc]\"\n",
        "puts \"\\x41[string repeat {} 3]3\"\n",
        "puts \"\\u0041[string length abc]\"\n",
    ] {
        preserves(source, "8.4");
        let (rewritten, _) = optimise(source, "jim");
        if let Some(jimsh) = std::env::var_os("TCL_LSP_JIMSH") {
            let jimsh = std::path::Path::new(&jimsh);
            assert_eq!(observed(jimsh, source), observed(jimsh, &rewritten));
        }
    }
}

#[test]
fn issue_2093_relative_self_stays_conservative() {
    let source =
        "oo::class create ::C {method m {} {puts [self class]}}\nset object [::C new]\n$object m\n";
    let (rewritten, _) = optimise(source, "tcl8.6");
    assert!(rewritten.contains("[self class]"), "{rewritten}");
    preserves(source, "8.6");
}

#[test]
fn issue_2419() {
    let Some(tcllib) = std::env::var_os("TCLLIB_2_0_DIR") else {
        return;
    };
    let modules = std::path::Path::new(&tcllib).join("modules");
    let source = format!(
        "lappend auto_path {}\n{}",
        tcl_codegen::codegen::helpers::tcl_list_element(&modules.to_string_lossy()),
        r#"package require struct::set
package require struct::list
proc inc {} {set s {}; struct::set include s a; return $s}
proc exc {} {set s {a b}; struct::set exclude s a; return $s}
proc add {} {set s {a}; struct::set add s {b}; return $s}
proc sub {} {set s {a b}; struct::set subtract s {a}; return $s}
proc shf {} {set l {a b c}; struct::list shift l; return $l}
proc asg {} {set x 0; struct::list assign {1 2} x y; return $x}
foreach p {inc exc add sub shf asg} {puts "$p: [$p]"}
"#,
    );
    for version in tcl_dialect::TclVersion::ALL {
        if version == tcl_dialect::TclVersion::V8_4 {
            continue;
        }
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let original = observed(&tclsh.path, &source);
        assert!(
            String::from_utf8_lossy(&original.stdout).contains("inc: a"),
            "{original:?}"
        );
        let (rewritten, _) = optimise(&source, &format!("tcl{}", version.version_string()));
        assert_eq!(original, observed(&tclsh.path, &rewritten), "{rewritten}");
    }
}

#[test]
fn additional_reported_witnesses() {
    for source in [
        "set x 5\ncatch {incr x} m\nputs \"$x $m\"\n",
        "set x 1\nset c [catch {append x y}]\nputs $x\n",
        "proc q {} {set x 1; uplevel 0 {info exists x}}\nputs [q]\n",
        "proc q {} {set x 1; foreach v [unset x] {}; puts [info exists x]}\nq\n",
        "set x 5\nafter 0 {puts $x}\nupdate\nset x 6\nputs $x\n",
        "proc p {} {set r 1; return \"$r x\"}\nputs [p]\n",
        "proc add {a b} {return [expr {$a + $b}]}\nproc p {} {set y [add abc 1]; puts hi}\np\n",
        "proc p {} {set y [string repeat x -1]; set z [lindex {a b} 0 0 0]; set w [llength \"a \\{\"]; puts hi}\np\n",
        "proc p {} {set y \"v[expr {1/0}]\"; puts hi}\np\n",
        "proc p {} {set {} 5; puts \"[set {}]\"}\np\n",
        "proc p {} {set {} 5; puts \"${} [set {}]\"}\np\n",
        "proc p {} {expr {1/0}; return foo}\nputs [p]\n",
        "proc p {a} {expr {1/0}; return [expr {$a + 1}]}\nputs [p 5]\n",
        "puts [expr {\"a\" in {a b}}]\n",
        "puts [expr {2.0 ** 0.5}]\n",
        "puts [lindex {a b c} 1+1]\n",
        "puts [string reverse abc]\n",
        "proc down {n} {if {$n > 0} {down [expr {$n - 1}]} else {expr {7}}}\nputs [down 3]\n",
    ] {
        preserves(source, "8.4");
    }
}

#[test]
fn issue_2431_release_less_semantic_axes() {
    for source in [
        "puts [expr {0o17 + 1_000}]\n",
        "puts [expr {2 ** 3}]\n",
        "puts [expr {a in {a b}}]\n",
        "puts [expr {a ni {a b}}]\n",
        "puts [expr {a lt b}]\n",
        "puts [expr {a le b}]\n",
        "puts [expr {a gt b}]\n",
        "puts [expr {a ge b}]\n",
        "puts [string cat a b]\n",
        "puts [string reverse abc]\n",
        "puts [dict create a b]\n",
        "puts [lrepeat 3 a]\n",
        "puts [lreverse {a b c}]\n",
        "puts [string length \"\\x41424344\"]\n",
        "puts [expr {10000000000000002.0 + 0.0}]\n",
    ] {
        let (rewritten, _) = optimise(source, "tcl");
        for version in tcl_dialect::TclVersion::ALL {
            let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
                continue;
            };
            assert_eq!(
                observed(&tclsh.path, source),
                observed(&tclsh.path, &rewritten),
                "{version:?}:\n{source}\n{rewritten}"
            );
        }
    }
}

#[test]
fn issue_2218_preserves_known_command_heads() {
    for body in [
        "return [expr {[expr {$x * 2}]}]",
        "puts [expr {[expr {$x * 2}]}]",
    ] {
        for suffix in [
            "",
            "proc g {} {puts safe}",
            "proc g {} {set cmd puts; $cmd safe}",
            "proc g {value} {puts $value}",
        ] {
            let source = format!("proc double_expr {{x}} {{{body}}}\n{suffix}\n");
            let (_, applied) = optimise(&source, "tcl8.6");
            assert!(
                applied
                    .iter()
                    .any(|row| row.code == tcl_core_types::DiagCode::O115),
                "{body}: {suffix}"
            );
        }
        for suffix in [
            "proc g {cmd} {$cmd}",
            "proc g {cmd} {return [$cmd]}",
            "proc g {cmd} {if {1} {$cmd}}",
            "proc g {} {unknown_extension_command}",
        ] {
            let source = format!("proc double_expr {{x}} {{{body}}}\n{suffix}\n");
            let (_, applied) = optimise(&source, "tcl8.6");
            assert!(
                applied
                    .iter()
                    .all(|row| row.code != tcl_core_types::DiagCode::O115),
                "{body}: {suffix}"
            );
        }
    }
}

#[test]
fn issue_2218_unknown_future_target_preserves_execution() {
    let source = "proc double_expr {x} {return [expr {[expr {$x * 2}]}]}\nproc g {cmd} {$cmd}\n";
    // The public dynamic procedure can select a target installed after load.
    let driver = "proc mutation {} {rename ::expr ::core_expr; proc ::expr {text} {return \"SHADOW:$text\"}}\ng mutation\nputs [double_expr 3]\n";
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
            continue;
        };
        let (rewritten, applied) = optimise(source, &format!("tcl{}", version.version_string()));
        assert!(
            applied
                .iter()
                .all(|row| row.code != tcl_core_types::DiagCode::O115)
        );
        let original = observed(&tclsh.path, &format!("{source}{driver}"));
        assert!(String::from_utf8_lossy(&original.stdout).contains("SHADOW:[expr"));
        assert_eq!(
            original,
            observed(&tclsh.path, &format!("{rewritten}{driver}")),
            "{version:?}\n{rewritten}"
        );
    }
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(tcl_test_support::require_jimsh().expect("required current Jim oracle"))
    } else {
        tcl_test_support::locate_jimsh().expect("validated Jim oracle")
    };
    if let Some(jim) = jim {
        let (rewritten, applied) = optimise(source, "jim");
        assert!(
            applied
                .iter()
                .all(|row| row.code != tcl_core_types::DiagCode::O115)
        );
        let original = observed(&jim.path, &format!("{source}{driver}"));
        assert!(String::from_utf8_lossy(&original.stdout).contains("SHADOW:[expr"));
        assert_eq!(
            original,
            observed(&jim.path, &format!("{rewritten}{driver}"))
        );
    }
}
