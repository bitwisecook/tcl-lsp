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

//! The value-transfer witnesses, program by program
//! (`docs/design/lanes/value-transfers.md` § *Plan for slices 2–13*).
//!
//! Each test drives the compiler the way a user reaches it — the shared
//! lattice through `CompilationUnit::build`, the rewrites through the
//! multipass optimiser — and, where the witness is a program's output,
//! runs the original and the optimised program under every `tclsh` release
//! found on `PATH`. A release that is not installed is skipped and the
//! test says which ran; none at all is reported, never skipped silently.

use std::io::Write as _;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::intervals::{Interval, compute_intervals_with, numbers_for_dialect};
use tcl_compiler::ir::Statement;
use tcl_compiler::lowering::lower_to_ir_with_dialect;
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::optimiser::manager::{optimise_source_multipass, optimise_with_dialect};
use tcl_compiler::static_loops::{
    DEFAULT_MAX_STATIC_LOOP_ITERS, LoopSemantics, StaticEnv, StaticValue, parse_literal_value,
    summarise_for_statement,
};
use tcl_compiler::tcl_expr_eval::FoldPolicy;
use tcl_core_types::DiagCode;
use tcl_registry::model::ingress::{resolve_environment, static_context_for};

/// The releases the oracle runs, oldest first.
const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// The multipass optimiser's iteration ceiling for a witness program.
const PASSES: usize = 8;

/// The dialects the exit witnesses analyse under: a release per numeral
/// grammar and integer tower; `f5-irules`, a vendor dialect that evaluates
/// under the 8.4 base it declares (ruling 8); and the lenient `tcl` profile,
/// which declares no release, so every axis answers by unanimity (ruling 7).
const DIALECTS: [&str; 5] = ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules", "tcl"];

/// Run `script` from a file under `tclsh`: whether it exited cleanly and
/// what it printed. A script run from a file exits non-zero on an error,
/// where one read from stdin does not.
fn run_script(tclsh: &str, script: &str) -> Option<(bool, String)> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "value-transfer-witness-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::File::create(&path)
        .and_then(|mut file| file.write_all(script.as_bytes()))
        .ok()?;
    let output = Command::new(tclsh).arg(&path).output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// A `tclsh<series>` on `PATH` whose patchlevel starts with `series`.
fn find_tclsh(series: &str) -> Option<String> {
    let command = format!("tclsh{series}");
    match run_script(&command, "puts -nonewline [info patchlevel]") {
        Some((true, level)) if level.starts_with(series) => Some(command),
        _ => None,
    }
}

/// Every release on `PATH` with its interpreter, oldest first; reports on
/// stderr when none is installed.
fn releases_on_path() -> Vec<(&'static str, String)> {
    let found: Vec<(&'static str, String)> = RELEASES
        .iter()
        .filter_map(|&series| find_tclsh(series).map(|tclsh| (series, tclsh)))
        .collect();
    if found.is_empty() {
        eprintln!("no tclsh on PATH: the output witnesses were not run");
    }
    found
}

/// The dialect profile name a release is analysed under.
fn dialect_of(series: &str) -> String {
    format!("tcl{series}")
}

/// `source` rewritten by the multipass optimiser under `dialect`, with
/// every rewrite it applied.
fn optimised(source: &str, dialect: &str) -> (String, Vec<Optimisation>) {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    optimise_source_multipass(source, registry, Some(profile), PASSES)
}

/// The single-pass rewrites of `source` under `dialect`, each span an
/// offset into `source` itself.
fn rewrites_of(source: &str, dialect: &str) -> Vec<Optimisation> {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    optimise_with_dialect(source, registry, Some(profile))
}

/// The compilation unit `CompilationUnit::build` makes of `source` under
/// `dialect` — the direct path every consumer reads the shared lattice
/// from.
fn unit_of(source: &str, dialect: &str) -> CompilationUnit {
    let registry = static_context_for(dialect).commands();
    CompilationUnit::build_for_dialect(source, registry, false, dialect)
}

/// The shared lattice's value for `var`'s version `version` in `proc`.
fn value_at(unit: &CompilationUnit, proc: &str, var: &str, version: u32) -> Option<LatticeValue> {
    let function = unit.procedures.get(proc).expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.values.get(&(symbol, version)).cloned()
}

/// The shared lattice's value for the last version of `var` in `proc`
/// under `dialect`.
fn last_value(source: &str, dialect: &str, proc: &str, var: &str) -> LatticeValue {
    let unit = unit_of(source, dialect);
    let function = unit.procedures.get(proc).expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function
        .sccp
        .values
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, value)| value.clone())
        .expect("a definition")
}

/// The answers the run recorded for `command`'s statements in `proc`.
fn answers_for(unit: &CompilationUnit, proc: &str, command: &str) -> Vec<String> {
    unit.procedures
        .get(proc)
        .expect("the procedure")
        .sccp
        .explanations
        .iter()
        .filter(|explanation| explanation.command == command)
        .map(|explanation| explanation.answer.clone())
        .collect()
}

fn text(value: &str) -> LatticeValue {
    LatticeValue::Const(ConstValue::String(value.to_owned()))
}

/// Every release on `PATH` prints `expected` for `source` and for its
/// optimised form under that release's dialect.
fn prints_under_every_release(source: &str, expected: &str) {
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, expected.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A value-position cell update whose read the host statement does not
/// hold declines rather than answering a pending bottom a join launders:
/// `tcl opt` had rewritten `puts $x` to `puts 5`, where `f 1` prints 2.
#[test]
fn a_value_position_increment_never_launders_a_join() {
    let source =
        "proc f {cond} {set n 1; if {$cond} {set x [incr n]} else {set x 5}; puts $x}\nf 1\nf 0\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts $x"),
            "{dialect}: `puts $x` must keep its variable\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "2\n5\n");
}

/// A cell update reads its words as Tcl substitutes them: a braced `$x` is
/// the text `$x`, not a read of `x`, and a quoted `\t` is a tab. Read by
/// their spelling, `puts $t` was rewritten to `puts a1` and `lappend l
/// "a\tb"` folded to `set l {{a\tb}}`.
#[test]
fn a_cell_update_reads_its_words_as_tcl_substitutes_them() {
    let cases: [(&str, &str, LatticeValue, &str); 4] = [
        (
            "proc p {} {set x 1; set s a; append s {$x}; set t $s; puts $t}\np\n",
            "s",
            text("a$x"),
            "a$x\n",
        ),
        (
            "proc p {} {set x 1; set l {}; lappend l {$x}; puts $l}\np\n",
            "l",
            text("{$x}"),
            "{$x}\n",
        ),
        (
            "proc p {} {set s x; append s \"a\\tb\"; set t $s; puts $t}\np\n",
            "s",
            text("xa\tb"),
            "xa\tb\n",
        ),
        (
            "proc p {} {set l {}; lappend l \"a\\tb\"; puts $l}\np\n",
            "l",
            text("{a\tb}"),
            "{a\tb}\n",
        ),
    ];
    for (source, var, value, output) in cases {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                last_value(source, dialect, "::p", var),
                value,
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// `[string range "a\tb" 0 1]` in value position reads the quoted word as
/// Tcl substitutes it (`a` and a tab); a braced word keeps its backslash.
#[test]
fn value_position_words_are_read_as_tcl_substitutes_them() {
    let quoted = "proc p {} {set r [string range \"a\\tb\" 0 1]; puts $r}\np\n";
    let braced = "proc p {} {set r [string range {a\\tb} 0 1]; puts $r}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert_eq!(last_value(quoted, dialect, "::p", "r"), text("a\t"));
        assert_eq!(last_value(braced, dialect, "::p", "r"), text("a\\"));
    }
    prints_under_every_release(quoted, "a\t\n");
    prints_under_every_release(braced, "a\\\n");
}

/// The shared lattice folds under the module's observed bindings, the
/// stance a rewrite's re-run proves under too: with `proc incr` defined at
/// the top level, `incr n` calls that procedure, so `n#2` is not the
/// builtin's 2, and the statement says why (#2164).
#[test]
fn the_shared_lattice_declines_a_renamed_head() {
    let body = "proc p {} {set n 1; incr n; return $n}\n";
    let shadowed = format!("proc incr {{v args}} {{return 99}}\n{body}");
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(&shadowed, dialect);
        assert_eq!(
            value_at(&unit, "::p", "n", 2),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["declined: rebinding-suspected"],
            "{dialect}"
        );
        let unit = unit_of(body, dialect);
        assert_eq!(
            value_at(&unit, "::p", "n", 2),
            Some(LatticeValue::Const(ConstValue::Int(2))),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["evaluated"],
            "{dialect}"
        );
    }
}

/// A `::`-qualified global a nested cell update writes is in its
/// procedure's global-write summary with no `global` declaration, so a
/// caller no longer forwards the value the global held before the call
/// or deletes the store it reads (#2214). An unqualified name with no
/// declaration stays local.
#[test]
fn a_qualified_nested_cell_update_is_a_global_write() {
    use tcl_compiler::cfg_builder::global_write_info::detect_global_write_procs;
    use tcl_compiler::lowering::lower_to_ir;
    let registry = static_context_for("tcl8.6").commands();
    for body in ["set y [incr ::hits]", "set y [expr {[incr ::hits] + 1}]"] {
        let module = lower_to_ir(&format!("proc bump {{}} {{{body}; return $y}}\n"), registry);
        let writes = detect_global_write_procs(&module);
        assert!(
            writes["bump"].names.contains("::hits"),
            "{body}: {writes:?}"
        );
        assert!(writes["bump"].names.contains("hits"), "{body}: {writes:?}");
    }
    let module = lower_to_ir("proc bump {} {set y [incr hits]; return $y}\n", registry);
    let writes = detect_global_write_procs(&module);
    assert!(writes["bump"].is_empty(), "{writes:?}");

    let source = "set hits 0\nproc bump {} {set y [incr ::hits]; return $y}\nbump\nputs $hits\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 0") && rewritten.contains("puts $hits"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "1\n");
}

/// The same holds for a qualified global a procedure reaches through
/// `upvar #0`: the caller at the global frame reads `::hits` as `hits`.
#[test]
fn a_qualified_alias_target_is_a_global_write_at_the_caller() {
    let source = "set hits 0\nproc bump {} {upvar #0 ::hits h; incr h}\nbump\nputs $hits\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 0") && rewritten.contains("puts $hits"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "1\n");
}

/// A callee's global-write summary records writes, not reads, and a
/// callee that writes a global may read it first: `set hits 10` ahead of
/// a `bump` running `global hits; incr hits 5` is observed, not dead
/// (tclsh 8.4 to 9.1 print 15, where deleting it printed 5).
#[test]
fn a_store_a_global_writing_callee_reads_is_kept() {
    let source = "set hits 10\nproc bump {} {global hits; incr hits 5}\nbump\nputs $hits\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 10"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "15\n");
}

/// A callee that only reads a global keeps the store it reads: the
/// slice-two record said O109 deleted `set hits 0` ahead of `show`, and it
/// does not — the rewrite keeps both stores, and tclsh 8.4 to 9.1 print 0
/// then 1 for both programs.
#[test]
fn a_store_a_global_reading_callee_observes_is_kept() {
    let source = "set hits 0\nproc show {} {global hits; puts $hits}\nshow\nset hits 1\nshow\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("set hits 0") && rewritten.contains("set hits 1"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "0\n1\n");
}

/// `[set x]` in value position reads the variable through the cell-write
/// route (the Tier-1 list); `[set x 10]` writes storage a value position
/// cannot land, so its host keeps no value.
#[test]
fn a_value_position_set_reads_the_variable() {
    let source = "proc p {} {set x hello; set r [set x]; set w [set x 10]; return $r$w}\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules"] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(text("hello")),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit, "::p", "w", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
}

/// Program (3) of the interface contract: `incr n` and `incr n 2` over
/// `set n 1` give `n#3` the 4 every release prints, in the shared lattice
/// under every dialect — each statement's route answering — and the
/// optimiser forwards it into `puts $n` (O100).
#[test]
fn program_three_folds_in_every_consumer() {
    let source = "proc p {} {set n 1; incr n; incr n 2; puts $n}\np\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "n", 3),
            Some(LatticeValue::Const(ConstValue::Int(4))),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["evaluated", "evaluated"],
            "{dialect}"
        );
        let rewrites = rewrites_of(source, dialect);
        assert!(
            rewrites.iter().any(|r| r.code == DiagCode::O100),
            "{dialect}: {rewrites:#?}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(rewritten.contains("puts 4"), "{dialect}:\n{rewritten}");
    }
    prints_under_every_release(source, "4\n");
}

/// A model's answer for `x` after the statement: the text of its value, or
/// `None` where it declines.
fn lattice_text(value: LatticeValue) -> Option<String> {
    match value {
        LatticeValue::Const(ConstValue::Int(i)) => Some(i.to_string()),
        LatticeValue::Const(ConstValue::String(s)) => Some(s),
        _ => None,
    }
}

/// The loop simulator's answer for `x` after `statement` runs once as the
/// body of `for {set i 0} {$i < 1} {incr i} {…}`, seeded as the solver
/// seeds it — `x` from `init` and `n` from 3 through the literal ingress.
fn simulated_text(dialect: &str, init: &str, statement: &str) -> Option<String> {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    let module = lower_to_ir_with_dialect(
        &format!("for {{set i 0}} {{$i < 1}} {{incr i}} {{{statement}}}\n"),
        registry,
        tcl_lexer::LexerConfig::default(),
        Some(profile),
    );
    let for_stmt = module
        .top_level
        .statements
        .iter()
        .find(|stmt| matches!(stmt, Statement::For { .. }))
        .expect("the loop");
    let mut seed = StaticEnv::new();
    seed.insert("x".to_owned(), parse_literal_value(init));
    seed.insert("n".to_owned(), parse_literal_value("3"));
    let env = summarise_for_statement(
        for_stmt,
        &seed,
        DEFAULT_MAX_STATIC_LOOP_ITERS,
        LoopSemantics {
            policy: FoldPolicy::from_registry(registry),
            registry,
        },
    )?;
    match env.get("x").expect("x after the loop") {
        StaticValue::Int(i) => Some(i.to_string()),
        StaticValue::Str(s) => Some(s.clone()),
        other => panic!("{dialect}: an increment gave {other:?}"),
    }
}

/// The interval domain's answer at `x`'s last definition in `::p`.
fn interval_at(unit: &CompilationUnit, dialect: &str) -> Interval {
    let function = unit.procedures.get("::p").expect("the procedure");
    let profile = resolve_environment(dialect).analyser_profile();
    let intervals = compute_intervals_with(
        &function.cfg,
        &function.ssa,
        &function.sccp.values,
        numbers_for_dialect(Some(profile)),
    );
    let x = function.ssa.var_symbol("x").expect("x");
    intervals
        .iter()
        .filter(|((symbol, _), _)| *symbol == x)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, interval)| *interval)
        .expect("an interval for x")
}

/// Whether `interval` holds the integer `text` spells (a bignum included).
fn interval_holds(interval: Interval, text: &str) -> bool {
    let value: i128 = text.parse().expect("an integer");
    interval.lo.is_none_or(|lo| i128::from(lo) <= value)
        && interval.hi.is_none_or(|hi| value <= i128::from(hi))
}

/// The lattice, the loop simulator and the interval domain answer one
/// increment alike: the lattice and the simulator give the same value or
/// both decline, and the interval never excludes the lattice's value.
/// Each value is the release's own: `incr x $n` is 8 everywhere; `incr x`
/// over `010` is 9 up to 8.6 and 11 from 9.0; over `9223372036854775807`
/// it is `9223372036854775808` from 8.5 and `-8` under 8.4, which no model
/// computes, so 8.4 declines. `f5-irules` answers as its 8.4 base does
/// (ruling 8), and the lenient profile, naming no single release, declines
/// both release-dependent cases.
#[test]
fn the_three_incr_models_agree() {
    let cases: [(&str, &str, [Option<&str>; 5]); 3] = [
        ("5", "incr x $n", [Some("8"); 5]),
        (
            "010",
            "incr x",
            [Some("9"), Some("9"), Some("11"), Some("9"), None],
        ),
        (
            "9223372036854775807",
            "incr x",
            [
                None,
                Some("9223372036854775808"),
                Some("9223372036854775808"),
                None,
                None,
            ],
        ),
    ];
    for (init, statement, expected) in cases {
        let source = format!("proc p {{}} {{set x {init}; set n 3; {statement}; return $x}}\n");
        for (dialect, want) in DIALECTS.into_iter().zip(expected) {
            let unit = unit_of(&source, dialect);
            let lattice =
                lattice_text(value_at(&unit, "::p", "x", 2).expect("the increment's definition"));
            let simulated = simulated_text(dialect, init, statement);
            assert_eq!(
                lattice.as_deref(),
                want,
                "{dialect}: lattice, {statement} over {init}"
            );
            assert_eq!(
                simulated, lattice,
                "{dialect}: simulator, {statement} over {init}"
            );
            if let Some(value) = &lattice {
                let interval = interval_at(&unit, dialect);
                assert!(
                    interval_holds(interval, value),
                    "{dialect}: {interval:?} excludes {value}"
                );
            }
        }
        for (series, tclsh) in releases_on_path() {
            let unit = unit_of(&source, &dialect_of(series));
            let Some(value) = lattice_text(value_at(&unit, "::p", "x", 2).expect("the definition"))
            else {
                continue;
            };
            let program = format!("set x {init}; set n 3; {statement}; puts $x");
            assert_eq!(
                run_script(&tclsh, &program),
                Some((true, format!("{value}\n"))),
                "tclsh{series}: {program}"
            );
        }
    }
}

/// `set result [incr n]` reads the store it increments, so neither dead
/// store rewrite (O109, O126) takes `set n 1` and the increment stays;
/// the optimised program prints 2 twice, as every release does (#2050).
#[test]
fn set_result_incr_keeps_its_increment() {
    let source = "proc p {} {set n 1; set result [incr n]; puts $result; puts $n}\np\n";
    let store = source.find("set n 1").expect("the store");
    for dialect in DIALECTS {
        let rewrites = rewrites_of(source, dialect);
        assert!(
            !rewrites.iter().any(|r| {
                matches!(r.code, DiagCode::O109 | DiagCode::O126)
                    && (r.span.start() as usize..r.span.end() as usize).contains(&store)
            }),
            "{dialect}: {rewrites:#?}"
        );
        let (rewritten, all) = optimised(source, dialect);
        assert!(
            rewritten.contains("set n 1") && rewritten.contains("[incr n]"),
            "{dialect}:\n{rewritten}\n{all:#?}"
        );
    }
    prints_under_every_release(source, "2\n2\n");
}

/// The string and list build chains fold through a `$var` piece the
/// lattice proves: `append s $p` (O104) — a leading space kept (#2052) —
/// and `lappend l a $x` (O130).
#[test]
fn o104_and_o130_fold_a_chain_through_a_lattice_operand() {
    let cases = [
        (
            "set s hello\nset p again\nappend s $p\nputs $s\n",
            DiagCode::O104,
            "set s helloagain",
            "helloagain\n",
        ),
        (
            "set s hello\nset p { again}\nappend s $p\nputs $s\n",
            DiagCode::O104,
            "set s {hello again}",
            "hello again\n",
        ),
        (
            "set l {}\nset x b\nlappend l a $x\nputs $l\n",
            DiagCode::O130,
            "set l {a b}",
            "a b\n",
        ),
    ];
    for (source, code, folded, output) in cases {
        for dialect in DIALECTS {
            let rewrites = rewrites_of(source, dialect);
            assert!(
                rewrites
                    .iter()
                    .any(|r| r.code == code && r.replacement == folded),
                "{dialect}: {source}\n{rewrites:#?}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// Deleting a store whose value was a quoted word takes the whole word:
/// no line of the optimised program holds only its closing quote (#2053).
#[test]
fn deleting_a_quoted_store_leaves_no_quote() {
    let source = "set s hello\nset p \"again\"\nappend s $p\nputs $s\n";
    for dialect in DIALECTS {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.lines().any(|line| line.trim() == "\""),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "helloagain\n");
}

/// `lassign` is no write under a profile whose release lacks it: under
/// 8.4 the call is an unknown command `catch` absorbs, so `a` is still
/// `old` at the `puts` — the optimiser forwards it, and the store that fed
/// it is then dead, where deleting the store while keeping the read made
/// 8.4 fail with `can't read "a"` — and 8.4 prints `old`. From 8.5
/// `lassign` assigns `new`, which no rewrite forwards past (#2144).
#[test]
fn lassign_is_not_a_write_under_a_profile_without_it() {
    let source = "set a old\ncatch {lassign {new second} a b}\nputs $a\n";
    let (rewritten, rewrites) = optimised(source, "tcl8.4");
    assert!(rewritten.contains("puts old"), "{rewritten}\n{rewrites:#?}");
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts $a"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(source, &dialect_of(series));
        let want = if series == "8.4" { "old\n" } else { "new\n" };
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, want.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A typed assignment reads its value word as Tcl substitutes it: a quoted
/// `\t` is a tab and a braced backslash-newline one space. Read by its
/// spelling, `set s "a\tb"` held four characters, so `string length $s`
/// folded to 4 and `[list $c]` quoted a backslash (tclsh 8.4 to 9.1 print
/// 3, 3, 2, `{x<TAB>y}` and 3).
#[test]
fn a_typed_assignment_reads_its_word_as_tcl_substitutes_it() {
    let cases: [(&str, &str, LatticeValue, &str); 5] = [
        (
            "proc p {} {set s \"a\\tb\"; puts [string length $s]}\np\n",
            "s",
            text("a\tb"),
            "3\n",
        ),
        (
            "proc p {} {set e \"p\\tq\"; set f [set e]; puts [string length $f]}\np\n",
            "f",
            text("p\tq"),
            "3\n",
        ),
        (
            "proc p {} {set b \"\\t\"; append b q; puts [string length $b]}\np\n",
            "b",
            text("\tq"),
            "2\n",
        ),
        (
            "proc p {} {set c \"x\\ty\"; set d [list $c]; puts $d}\np\n",
            "d",
            text("{x\ty}"),
            "{x\ty}\n",
        ),
        (
            "proc p {} {set s {a\\\nb}; puts [string length $s]}\np\n",
            "s",
            text("a b"),
            "3\n",
        ),
    ];
    for (source, var, value, output) in cases {
        for dialect in DIALECTS {
            assert_eq!(
                last_value(source, dialect, "::p", var),
                value,
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// A braced `incr` amount is its own text: `incr x {$n}` raises `expected
/// integer but got "$n"` in every release, so `x#2` has no value and the
/// statement reads no `n`. Read as a substitution it folded to 4.
#[test]
fn a_braced_increment_amount_is_its_text() {
    let body = "proc p {} {set n 3; set x 1; incr x {$n}; return $x}\n";
    for dialect in DIALECTS {
        let unit = unit_of(body, dialect);
        assert_eq!(
            value_at(&unit, "::p", "x", 2),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["declined: wrong-representation"],
            "{dialect}"
        );
        let function = &unit.procedures["::p"];
        let n = function.ssa.var_symbol("n").expect("n");
        let increment = function
            .ssa
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find(|stmt| matches!(stmt.statement, Statement::Incr { .. }))
            .expect("the increment");
        assert!(
            !increment.uses.contains_key(&n),
            "{dialect}: a braced amount reads nothing"
        );
    }
    let source = format!("{body}puts [catch p msg]; puts $msg\n");
    prints_under_every_release(&source, "1\nexpected integer but got \"$n\"\n");
}

/// A list's first element that starts with `#` is brace-quoted from 8.5 and
/// bare in 8.4 (`puts [list # a]` prints `# a` under tclsh 8.4 and `{#} a`
/// from 8.5), so each release folds its own rendering and a profile naming
/// no release declines.
#[test]
fn a_leading_hash_is_quoted_per_release() {
    let source = "proc p {} {set r [list # a]; set l {}; lappend l # b; puts $r; puts $l}\np\n";
    for (dialect, list, appended) in [
        ("tcl8.4", Some("# a"), Some("# b")),
        ("tcl8.6", Some("{#} a"), Some("{#} b")),
        ("tcl9.0", Some("{#} a"), Some("{#} b")),
        ("f5-irules", Some("# a"), Some("# b")),
        ("tcl", None, None),
    ] {
        let expect = |value: Option<&str>| value.map_or(LatticeValue::Overdefined, text);
        assert_eq!(
            last_value(source, dialect, "::p", "r"),
            expect(list),
            "{dialect}"
        );
        assert_eq!(
            last_value(source, dialect, "::p", "l"),
            expect(appended),
            "{dialect}"
        );
    }
    // The same rule reaches every consumer that renders a list for the
    // target: the `lappend` chain fold (O130) and the `args` list an
    // argument-sensitive procedure fold (O103) seeds.
    let variadic = "proc f {args} {return $args}\nputs [f # a]\nputs [f #x b]\n";
    for (series, tclsh) in releases_on_path() {
        let (expected, variadic_expected) = if series == "8.4" {
            ("# a\n# b\n", "# a\n#x b\n")
        } else {
            ("{#} a\n{#} b\n", "{#} a\n{#x} b\n")
        };
        for (program, expected) in [(source, expected), (variadic, variadic_expected)] {
            let (rewritten, _) = optimised(program, &dialect_of(series));
            for program in [program, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, expected.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A typed assignment is `set`'s lowering, so once the module shadows
/// `set` it proves nothing: with `proc set {name value} {return ZZZ}`,
/// `set s hello` never writes `s` and `append s world` leaves `world`
/// (tclsh 8.4 to 9.1), where folding the assignment rewrote the program to
/// print `helloworld`.
#[test]
fn a_typed_assignment_declines_once_set_is_rebound() {
    let source =
        "proc set {name value} {return ZZZ}\nproc p {} {set s hello; append s world; puts $s}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "s", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("helloworld"),
            "{dialect}:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "world\n");
}

/// O100 forwards a value holding a tab as a word that re-reads to the same
/// value: the tab stays a tab inside the braces (tclsh 8.4 to 9.1 print
/// `a<TAB>b` and `x<TAB>y`, before and after the rewrite).
#[test]
fn o100_forwards_a_tab_verbatim() {
    for (source, word, output) in [
        ("set r {a\tb}\nputs $r\n", "{a\tb}", "a\tb\n"),
        (
            "set b [string range \"x\\ty\" 0 2]\nputs $b\n",
            "{x\ty}",
            "x\ty\n",
        ),
    ] {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let rewrites = rewrites_of(source, dialect);
            assert!(
                rewrites
                    .iter()
                    .any(|r| r.code == DiagCode::O100 && r.replacement == word),
                "{dialect}: {rewrites:#?}"
            );
        }
        prints_under_every_release(source, output);
    }
}

/// A keyed update reads the dictionary it rewrites under every spelling
/// that reaches it: the qualified `::tcl::dict::` commands, a nested
/// `[dict set …]`, and an alias of `dict set`. The read had been recorded
/// only for the `dict` ensemble at statement level, so O109 deleted the
/// store feeding the others: `::tcl::dict::set d k v` printed `k v` where
/// tclsh 8.5 to 9.1 print `a 1 k v`. Tcl 8.4 has no `dict`, and every
/// program fails there the same way before and after.
#[test]
fn a_keyed_update_reads_its_dictionary_under_every_spelling() {
    let programs = [
        (
            "proc p {} {set d {a 1}; ::tcl::dict::set d k v; return $d}\nputs [p]\n",
            "a 1 k v\n",
        ),
        (
            "proc p {} {set d {a 1}; puts [dict set d k v]}\np\n",
            "a 1 k v\n",
        ),
        (
            "proc p {} {set d {a 1}; puts [::tcl::dict::set d k v]}\np\n",
            "a 1 k v\n",
        ),
        (
            "proc p {} {set d {a 1 b 2}; ::tcl::dict::unset d a; return $d}\nputs [p]\n",
            "b 2\n",
        ),
        (
            "proc p {} {set d {a 1}; ::tcl::dict::incr d a; return $d}\nputs [p]\n",
            "a 2\n",
        ),
        (
            "proc p {} {set d {a 1}; ::tcl::dict::append d a x; return $d}\nputs [p]\n",
            "a 1x\n",
        ),
        (
            "proc p {} {set d {a 1}; ::tcl::dict::lappend d a x; return $d}\nputs [p]\n",
            "a {1 x}\n",
        ),
        (
            "interp alias {} ds {} dict set\nproc p {} {set d {a 1}; ds d k v; return $d}\nputs [p]\n",
            "a 1 k v\n",
        ),
    ];
    for (source, expected) in programs {
        for (series, tclsh) in releases_on_path() {
            let (rewritten, _) = optimised(source, &dialect_of(series));
            if series == "8.4" {
                assert_eq!(
                    run_script(&tclsh, &rewritten),
                    run_script(&tclsh, source),
                    "tclsh8.4:\n{rewritten}"
                );
                continue;
            }
            for program in [source, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, expected.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A BPF-Tcl expression never takes the Tcl engine's answer: BPF-Tcl's
/// signed division truncates towards zero (`-7 / 2` is `-3`) where Tcl
/// floors (`-4` under tclsh 8.4 to 9.1), so the route under the BPF
/// language evaluates nothing and the BPF frontend's own arithmetic stays
/// the only answer. No shipped command declares the BPF language yet; the
/// synthetic `bpfexpr` stands for the frontend's expression arguments.
#[test]
fn a_bpf_expression_never_takes_the_tcl_answer() {
    use tcl_registry::spec::CommandSpec;
    use tcl_registry::value_transfer::SemanticsDeclaration;
    use tcl_registry::value_transfer::builtins::BPF_EXPR;
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "bpfexpr",
        semantics: SemanticsDeclaration::Declared(&BPF_EXPR),
        ..CommandSpec::DEFAULT
    });
    let source = "proc p {} {set r [bpfexpr {-7 / 2}]; set t [expr {-7 / 2}]; return $r$t}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "r", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "bpfexpr"),
        ["declined: unsupported"]
    );
    assert_eq!(
        value_at(&unit, "::p", "t", 1),
        Some(LatticeValue::Const(ConstValue::Int(-4)))
    );
    prints_under_every_release("puts [expr {-7 / 2}]\n", "-4\n");
}

/// A math function the module rebinds declines. From 8.5 `abs(…)`
/// dispatches to the command `::tcl::mathfunc::abs`, so the module's `proc`
/// of that name is what runs: tclsh 8.5 to 9.1 print 99. Under 8.4 the
/// grammar dispatches internally and has no such command, so the `proc`
/// cannot even be created and the program prints 2. A namespace-local
/// override is the same rebinding: from 8.5 `expr` resolves
/// `tcl::mathfunc::abs` relative to the namespace it runs in first, so
/// `abs(…)` inside `::ns` calls `::ns::tcl::mathfunc::abs` (tclsh 8.5 to
/// 9.1: 99; 8.4: 2).
#[test]
fn abs_rebinding_declines() {
    let rebound = "catch {rename ::tcl::mathfunc::abs ::tcl::mathfunc::saved_abs}\n\
                   catch {proc ::tcl::mathfunc::abs {x} {return 99}}\n\
                   proc p {} {set r [expr {abs(-2)}]; return $r}\nputs [p]\n";
    let local = "namespace eval ns {\n\
                 namespace eval tcl::mathfunc { proc abs {x} {return 99} }\n\
                 proc p {} {set r [expr {abs(-2)}]; return $r}\n\
                 }\nputs [ns::p]\n";
    let plain = "proc p {} {set r [expr {abs(-2)}]; return $r}\nputs [p]\n";
    let two = Some(LatticeValue::Const(ConstValue::Int(2)));
    for (dialect, want) in [
        ("tcl8.4", two.clone()),
        ("tcl8.6", Some(LatticeValue::Overdefined)),
        ("tcl9.0", Some(LatticeValue::Overdefined)),
    ] {
        let unit = unit_of(rebound, dialect);
        assert_eq!(value_at(&unit, "::p", "r", 1), want, "{dialect}");
        assert_eq!(
            value_at(&unit_of(local, dialect), "::ns::p", "r", 1),
            want,
            "{dialect}: the namespace-local override"
        );
        assert_eq!(
            value_at(&unit_of(plain, dialect), "::p", "r", 1),
            two,
            "{dialect}"
        );
    }
    assert_eq!(
        answers_for(&unit_of(rebound, "tcl9.0"), "::p", "expr"),
        ["declined: rebinding-suspected"]
    );
    assert_eq!(
        answers_for(&unit_of(local, "tcl9.0"), "::ns::p", "expr"),
        ["declined: rebinding-suspected"]
    );
    for (series, tclsh) in releases_on_path() {
        let expected = if series == "8.4" { "2\n" } else { "99\n" };
        for source in [rebound, local] {
            let (rewritten, _) = optimised(source, &dialect_of(series));
            for program in [source, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, expected.to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A nested command the module rebinds declines the whole expression: with
/// the module's own `proc llength`, `[llength {a b}]` is 99, and tclsh 8.4
/// to 9.1 print 198 for `expr {[llength {a b}] * 2}`; the builtin fold would
/// have answered 4.
#[test]
fn a_rebound_nested_head_declines_the_expression() {
    let source = "proc llength {l} {return 99}\n\
                  proc p {} {set r [expr {[llength {a b}] * 2}]; return $r}\nputs [p]\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "expr"),
            ["declined: rebinding-suspected"],
            "{dialect}"
        );
    }
    let pure = "proc p {} {set r [expr {[llength {a b}] * 2}]; return $r}\nputs [p]\n";
    for dialect in DIALECTS {
        assert_eq!(
            value_at(&unit_of(pure, dialect), "::p", "r", 1),
            Some(LatticeValue::Const(ConstValue::Int(4))),
            "{dialect}"
        );
    }
    prints_under_every_release(source, "198\n");
    prints_under_every_release(pure, "4\n");
}

/// A condition over one finite input decides when every member agrees:
/// `foreach a {1 2}` makes `a` the set `{1, 2}`, so `$a > 0` is true for
/// each and the branch folds, while `$a > 1` differs between the members
/// and stays open (tclsh 8.4 to 9.1 print `pos pos` and `small big`).
#[test]
fn a_finite_condition_decides_when_every_member_agrees() {
    let decided = |source: &str, dialect: &str| -> Vec<(String, bool)> {
        unit_of(source, dialect)
            .procedures
            .get("::p")
            .expect("the procedure")
            .sccp
            .constant_branches
            .iter()
            .map(|branch| (branch.condition.clone(), branch.value))
            .collect()
    };
    let agree = "proc p {} {foreach a {1 2} {if {$a > 0} {puts pos} else {puts neg}}}\np\n";
    let split = "proc p {} {foreach a {1 2} {if {$a > 1} {puts big} else {puts small}}}\np\n";
    for dialect in DIALECTS {
        assert!(
            decided(agree, dialect).contains(&("$a > 0".to_owned(), true)),
            "{dialect}: {:?}",
            decided(agree, dialect)
        );
        assert!(
            !decided(split, dialect)
                .iter()
                .any(|(condition, _)| condition == "$a > 1"),
            "{dialect}: {:?}",
            decided(split, dialect)
        );
    }
    prints_under_every_release(agree, "pos\npos\n");
    prints_under_every_release(split, "small\nbig\n");
}

/// A folded call keeps a double's Tcl spelling: `proc p {} {return [expr
/// {1.0 * 3}]}; puts [p]` prints `3.0` under tclsh 8.4 to 9.1, and O103 had
/// rewritten the call to `puts 3` with Rust's float rendering.
#[test]
fn a_folded_call_keeps_a_doubles_spelling() {
    for source in [
        "proc p {} {return [expr {1.0 * 3}]}\nputs [p]\n",
        "proc p {} {set r [expr {1.0 * 3}]; return $r}\nputs [p]\n",
    ] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(!rewritten.contains("puts 3\n"), "{dialect}:\n{rewritten}");
        }
        prints_under_every_release(source, "3.0\n");
    }
}

/// `format` runs the shared core on its registry-owned route, reading its
/// operands from the lattice and its literal words under the document's
/// escape grammar: tclsh 8.4 and 8.5 read `"\x4142"` as the one byte `B`
/// (`\x` takes every hex digit before 8.6) and 8.6 to 9.1 as `A42`; `%03d`
/// of the constant 5 is `005` and `%5.2f` of 3.14159 is ` 3.14` everywhere.
#[test]
fn format_folds_through_the_shared_core() {
    let source = "proc p {} {set n 5; set r [format %03d $n]; set s [format %s \"\\x4142\"]; \
                  set t [format %5.2f 3.14159]; return \"$r|$s|$t\"}\nputs [p]\n";
    for (dialect, bytes) in [("tcl8.4", "B"), ("tcl8.6", "A42"), ("tcl9.0", "A42")] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(text("005")),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit, "::p", "s", 1),
            Some(text(bytes)),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit, "::p", "t", 1),
            Some(text(" 3.14")),
            "{dialect}"
        );
    }
    for (series, tclsh) in releases_on_path() {
        let expected = if matches!(series, "8.4" | "8.5") {
            "005|B| 3.14\n"
        } else {
            "005|A42| 3.14\n"
        };
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, expected.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// The route tally counts every family's entries once — the fixed point
/// re-evaluates each statement several times before it settles, so only
/// the settled sweep counts — with a nested route entry counted beside its
/// host's: `set r [expr {"x"}]; set n [expr {[string length abcdef] *
/// 2}]` is two fused expressions, one nesting a direct-routed `string
/// length`, so `direct 1 · expression 2 · implementation 0`; `incr` alone
/// is a direct entry with no expression; a decided literal condition is an
/// expression entry, recorded once for reachability and once for the
/// collected branch, so `if {1} {…}` is two expression entries and no
/// direct one. No command declares an `EvaluatorCapability` yet, so
/// `implementation` stays 0 throughout (slice 4). An entry is counted only
/// once the route is entered: with the module's own `proc expr`, `set r
/// [expr {1 + 1}]` declines at the trust check and never reaches the
/// engine, so it counts nothing.
#[test]
fn route_entries_are_counted_per_family() {
    let tally_of = |source: &str, dialect: &str| {
        unit_of(source, dialect)
            .procedures
            .get("::p")
            .expect("the procedure")
            .sccp
            .route_tally
    };
    let nested_expr = "proc p {} {set r [expr {\"x\"}]; \
                        set n [expr {[string length abcdef] * 2}]}";
    let direct_only = "proc p {} {set x 1; incr x; return $x}";
    let expression_only = "proc p {} {if {1} {puts a} else {puts b}}";
    let rebound_expr = "proc expr {args} {return 99}\nproc p {} {set r [expr {1 + 1}]}";
    for dialect in DIALECTS {
        let tally = tally_of(nested_expr, dialect);
        assert_eq!(tally.direct, 1, "{dialect}: the nested `string length`");
        assert_eq!(tally.expression, 2, "{dialect}: both fused `expr`s");
        assert_eq!(tally.implementation, 0, "{dialect}");

        let tally = tally_of(direct_only, dialect);
        assert_eq!(tally.direct, 1, "{dialect}: the typed `incr`");
        assert_eq!(tally.expression, 0, "{dialect}");
        assert_eq!(tally.implementation, 0, "{dialect}");

        let tally = tally_of(expression_only, dialect);
        assert_eq!(tally.direct, 0, "{dialect}");
        assert_eq!(
            tally.expression, 2,
            "{dialect}: the condition, reachability and the collected branch"
        );
        assert_eq!(tally.implementation, 0, "{dialect}");

        let tally = tally_of(rebound_expr, dialect);
        assert_eq!(
            (tally.direct, tally.expression, tally.implementation),
            (0, 0, 0),
            "{dialect}: a rebound `expr` enters no route"
        );
    }
}

/// The interface contract's `expr` acceptance list: multi-argument forms,
/// braced versus quoted arguments, short-circuit operators and ternaries,
/// strings that look like code, nested pure substitutions, errors,
/// bignums, and target release ambiguity
/// (`docs/design/compiler/value-transfers-migration.md`, slice 3's exit).
/// Each case is `proc p {} {<prelude>; set r [<expr call>]}`, oracle
/// values checked against `tclsh8.4` to `tclsh9.1` directly.
/// `expression_witnesses_match_every_release_on_path` re-runs the same
/// programs against the real interpreters. `2**64` and `1 << 70` are
/// beyond-wide from 8.5: `tclsh8.4` raises for the first (`**` is not an
/// 8.4 operator) and wraps to 0 for the second, so both decline under
/// `tcl8.4` (`WrongRepresentation`) and under `f5-irules`, whose runtime
/// base is 8.4's (D48); `"010" + 0` reads the leading zero as octal up to
/// 8.6, `f5-irules` included, and as decimal from 9.0. The lenient profile
/// declares no release, so all three release-dependent cases decline there.
#[test]
fn expr_acceptance_list() {
    let cases: [(&str, &str, &str, [Option<&str>; 5]); 10] = [
        ("multi-argument form", "", "expr 1 + 2", [Some("3"); 5]),
        (
            "a quoted argument substituted as text",
            "set a {1 + 1}",
            "expr \"$a * 2\"",
            [Some("3"); 5],
        ),
        (
            "short-circuit &&",
            "",
            "expr {0 && [error never]}",
            [Some("0"); 5],
        ),
        (
            "a ternary",
            "",
            "expr {1 ? \"yes\" : \"no\"}",
            [Some("yes"); 5],
        ),
        (
            "a value that looks like code is never re-substituted",
            "set a {[exit]}",
            "expr {$a eq {[exit]}}",
            [Some("1"); 5],
        ),
        (
            "a nested pure substitution",
            "",
            "expr {[string length abcdef] * 2}",
            [Some("12"); 5],
        ),
        ("an error", "", "expr {1/0}", [None; 5]),
        (
            "a bignum",
            "",
            "expr {2**64}",
            [
                None,
                Some("18446744073709551616"),
                Some("18446744073709551616"),
                None,
                None,
            ],
        ),
        (
            "a shift beyond wide",
            "",
            "expr {1 << 70}",
            [
                None,
                Some("1180591620717411303424"),
                Some("1180591620717411303424"),
                None,
                None,
            ],
        ),
        (
            "a leading-zero numeral, release-dependent",
            "",
            "expr {\"010\" + 0}",
            [Some("8"), Some("8"), Some("10"), Some("8"), None],
        ),
    ];
    for (description, prelude, expr_call, expected) in cases {
        let source = format!("proc p {{}} {{{prelude}\nset r [{expr_call}]\n}}");
        for (dialect, want) in DIALECTS.into_iter().zip(expected) {
            let unit = unit_of(&source, dialect);
            let folded = value_at(&unit, "::p", "r", 1).and_then(lattice_text);
            assert_eq!(
                folded.as_deref(),
                want,
                "{dialect}: {description} ({expr_call})"
            );
        }
    }
}

/// One distinct finite SSA value stays correlated with itself: `foreach a
/// {1 2} {set r [expr {$a * $a}]}` gives the in-loop `r` the set `{1, 4}`,
/// never `{1, 2, 4}` — the cartesian product a wrongly-independent pairing
/// would produce (§ *The correlated finite-set limit*).
#[test]
fn the_square_of_one_finite_input_stays_correlated() {
    let source = "proc p {} {foreach a {1 2} {set r [expr {$a * $a}]}}";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let value = value_at(&unit, "::p", "r", 1).expect("r's in-loop definition");
        let LatticeValue::ConstSet(mut members) = value else {
            panic!("{dialect}: {value:?} is not a finite set");
        };
        members.sort_by_key(|c| match c {
            ConstValue::Int(i) => *i,
            other => panic!("{dialect}: {other:?}"),
        });
        assert_eq!(
            members,
            vec![ConstValue::Int(1), ConstValue::Int(4)],
            "{dialect}"
        );
    }
}

/// The mirror pairs of the interface page
/// (`docs/design/compiler/value-transfers.md` § *The correlated
/// finite-set limit*): `a` and `b` are the loop's two binders, so pairing
/// them by position or taking their cartesian product would both be
/// unsound, and neither post-loop branch decides — `x` is 20 and `y` is
/// 25 in every release, but only ordered enumeration (slice 12) answers
/// that, never the finite-set lift.
///
/// On this pre-slice-5 tree a two-binder `foreach`'s source layout is
/// still declined (the migration plan's ledger: `foreach` / `lmap`
/// "declining the source layout until slice 5"), so `a` and `b` are
/// `Overdefined` from the header rather than the page's two distinct
/// `Finite` identities, and `expr` declines `not-exact` rather than the
/// page's `CorrelatedSets` — the reason slice 5 gives its named shape.
/// The outcome this test pins, that `x` / `y` never fold and neither
/// branch decides, holds either way.
#[test]
fn the_mirror_pairs_decline_as_correlated() {
    let source = "proc p {} {\n\
                   set x 0\n\
                   foreach {a b} {1 10 2 20} { set x [expr {$b / $a}] }\n\
                   if {$x == 20} { puts twenty } else { puts other }\n\
                   set y 0\n\
                   foreach {a b} {1 20 2 10} { set y [expr {$b / $a}] }\n\
                   if {$y == 25} { puts twentyfive } else { puts other }\n\
                  }\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        for var in ["x", "y"] {
            let symbol = function.ssa.var_symbol(var).expect(var);
            let last = function
                .sccp
                .values
                .iter()
                .filter(|((sym, _), _)| *sym == symbol)
                .max_by_key(|((_, version), _)| *version)
                .map(|(_, value)| value.clone())
                .expect("a definition");
            assert_eq!(last, LatticeValue::Overdefined, "{dialect}: {var}");
        }
        for condition in ["$x == 20", "$y == 25"] {
            assert!(
                function
                    .sccp
                    .constant_branches
                    .iter()
                    .all(|b| b.condition != condition),
                "{dialect}: {condition} decided: {:?}",
                function.sccp.constant_branches
            );
        }
    }
}

/// A condition over a leading-zero digit string decides nothing under 8.x:
/// the grammar reads `08` as an invalid octal, so it reaches the condition
/// as text and `if` raises `expected boolean value but got "08"` (tclsh 8.5
/// and 8.6; 8.4, 9.0 and 9.1 print `yes`). It was decided true (I230) and
/// rewritten to `if {1}`. Under 9.0 `08` is the number 8 and the branch is
/// decided.
#[test]
fn an_invalid_octal_condition_decides_nothing() {
    for (literal, quoted) in [("08", false), ("-08", false), ("08", true)] {
        let condition = if quoted {
            format!("\"{literal}\"")
        } else {
            "$x".to_owned()
        };
        let source = format!(
            "proc p {{}} {{set x {literal}; if {{{condition}}} {{puts yes}} else {{puts no}}}}\np\n"
        );
        for (dialect, decided) in [("tcl8.6", false), ("tcl9.0", true)] {
            let unit = unit_of(&source, dialect);
            let function = unit.procedures.get("::p").expect("the procedure");
            assert_eq!(
                !function.sccp.constant_branches.is_empty(),
                decided,
                "{dialect}: {source}: {:?}",
                function.sccp.constant_branches
            );
        }
        for (series, tclsh) in releases_on_path() {
            let (rewritten, rewrites) = optimised(&source, &dialect_of(series));
            assert_eq!(
                run_script(&tclsh, &rewritten),
                run_script(&tclsh, &source),
                "tclsh{series}:\n{rewritten}\n{rewrites:#?}"
            );
        }
    }
}

/// `expr_acceptance_list`'s programs, run against the real `tclsh` per
/// release found on `PATH`: wherever the compiler's fold answers a value
/// it is the same value `tclsh` prints, and wherever the underlying
/// program raises the fold has declined too. At least half the programs
/// must fold on any release, so the witness is not vacuous. The optimised
/// program must print what the original prints — or raise where it raises —
/// under the same release, so a rewrite never folds what the lattice
/// declines: the last four programs are O101's (`1 << 70` wraps to 0 under
/// tclsh 8.4; `1e308 * 10` raises `floating-point value too large to
/// represent` there; `min` is unknown before 8.5; `ABS` is no math function
/// in any release).
#[test]
fn expression_witnesses_match_every_release_on_path() {
    let cases: [(&str, &str); 13] = [
        ("", "expr 1 + 2"),
        ("set a {1 + 1}", "expr \"$a * 2\""),
        ("", "expr {0 && [error never]}"),
        ("", "expr {1 ? \"yes\" : \"no\"}"),
        ("set a {[exit]}", "expr {$a eq {[exit]}}"),
        ("", "expr {[string length abcdef] * 2}"),
        ("", "expr {1/0}"),
        ("", "expr {2**64}"),
        ("", "expr {1 << 70}"),
        ("", "expr {\"010\" + 0}"),
        ("", "expr {1e308 * 10}"),
        ("", "expr {min(1,2)}"),
        ("", "expr {ABS(-2)}"),
    ];
    let mut releases = 0usize;
    for (series, tclsh) in releases_on_path() {
        releases += 1;
        let dialect = dialect_of(series);
        let mut answered = 0usize;
        for (prelude, expr_call) in cases {
            let source = format!("proc p {{}} {{{prelude}\nset r [{expr_call}]\n}}");
            let unit = unit_of(&source, &dialect);
            let folded = value_at(&unit, "::p", "r", 1).and_then(lattice_text);
            let oracle = run_script(&tclsh, &format!("{prelude}\nset r [{expr_call}]\nputs $r"));
            match (&folded, oracle) {
                (Some(value), Some((true, printed))) => {
                    assert_eq!(
                        value.as_str(),
                        printed.trim_end_matches('\n'),
                        "tclsh{series}: {expr_call}"
                    );
                    answered += 1;
                }
                (Some(value), other) => panic!(
                    "tclsh{series}: {expr_call} folded to {value} but tclsh answered {other:?}"
                ),
                // A decline under a release whose runtime tclsh happens
                // not to raise for (`1 << 70` wraps silently under 8.4)
                // is still a decline: nothing to check either way.
                (None, _) => {}
            }
            let program =
                format!("proc p {{}} {{{prelude}\nset r [{expr_call}]\nreturn $r\n}}\nputs [p]\n");
            let (rewritten, rewrites) = optimised(&program, &dialect);
            assert_eq!(
                run_script(&tclsh, &rewritten),
                run_script(&tclsh, &program),
                "tclsh{series}: {expr_call}\n{rewritten}\n{rewrites:#?}"
            );
        }
        assert!(
            answered * 2 > cases.len(),
            "tclsh{series}: the fold answered only {answered} of {} cases",
            cases.len()
        );
    }
    if releases == 0 {
        eprintln!("no tclsh on PATH: expression_witnesses_match_every_release_on_path ran nothing");
    }
}

/// O101 rewrites only what the shared expression route proves, so the
/// programs the old constant folder rewrote against the target stay put
/// (`tcl opt --profile full` applies every pass, as `optimised` does).
/// Oracle, tclsh 8.4: `1 << 70` is `0` (the wide shift wraps), `1e308 * 10`
/// raises `floating-point value too large to represent`, and `min(1,2)`
/// raises `unknown math function "min"`; 8.5 to 9.1 answer
/// `1180591620717411303424`, `Inf` and `1`. `ABS(-2)` raises on every
/// release (`unknown math function "ABS"` under 8.4, `invalid command name
/// "tcl::mathfunc::ABS"` from 8.5): a math function's name is
/// case-sensitive. An infinity in the middle is the same tower: `(1e308 *
/// 10) > 0` is 1 from 8.5 and raises at the product under 8.4. None of them
/// folds under iRules — 8.4, the release its engine runs, folds none — nor
/// under the version-less `tcl` profile, whose releases disagree on each.
#[test]
fn o101_rewrites_only_what_the_route_proves() {
    let cases: [(&str, [Option<&str>; 5]); 5] = [
        (
            "1 << 70",
            [
                None,
                Some("1180591620717411303424"),
                Some("1180591620717411303424"),
                None,
                None,
            ],
        ),
        ("1e308 * 10", [None, Some("Inf"), Some("Inf"), None, None]),
        ("(1e308 * 10) > 0", [None, Some("1"), Some("1"), None, None]),
        ("min(1,2)", [None, Some("1"), Some("1"), None, None]),
        ("ABS(-2)", [None; 5]),
    ];
    for (expression, answers) in cases {
        let program =
            format!("proc p {{}} {{set r [expr {{{expression}}}]; return $r}}\nputs [p]\n");
        for (dialect, answer) in DIALECTS.into_iter().zip(answers) {
            let (rewritten, rewrites) = optimised(&program, dialect);
            let kept = rewritten.contains(&format!("expr {{{expression}}}"));
            match answer {
                None => assert!(
                    kept,
                    "{dialect}: `expr {{{expression}}}` must stay:\n{rewritten}\n{rewrites:#?}"
                ),
                Some(value) => assert!(
                    !kept && rewritten.contains(value),
                    "{dialect}: `expr {{{expression}}}` folds to {value}:\n{rewritten}\n{rewrites:#?}"
                ),
            }
        }
    }
}

/// O112 decided a condition with the old constant folder, which answered
/// a beyond-wide integer or an infinity under 8.4 as 8.5 does and read no
/// binding. tclsh 8.4: `1 << 70` wraps to 0, so `if {(1 << 70) == 0}`
/// takes its first branch and `while {(1 << 70) == 0}` runs; `1e308 * 10`
/// raises `floating-point value too large to represent`. From 8.5 the
/// shift is 1180591620717411303424 and the product `Inf`. With `proc
/// ::tcl::mathfunc::abs {x} {return 99}`, tclsh 8.5 to 9.1 print `other`
/// and `loop`, where O112 kept `puts two` alone (8.4 has no wrapper
/// namespace, so the `proc` fails and the builtin prints `two`). Each
/// program prints the same optimised as it does unoptimised under every
/// release.
#[test]
fn a_structure_fold_stays_within_the_targets_tower() {
    let rebound = "catch {proc ::tcl::mathfunc::abs {x} {return 99}}\n\
                   proc p {} {\n\
                   if {abs(-2) == 2} {puts two} else {puts other}\n\
                   while {abs(-2) == 99} {puts loop; break}\n\
                   }\np\n";
    let programs = [
        (
            "proc p {} {if {(1 << 70) == 0} {puts zero} else {puts big}}\np\n",
            &["tcl8.4"][..],
            &["if {"][..],
        ),
        (
            "proc p {} {while {(1 << 70) == 0} {puts once; break}; puts done}\np\n",
            &["tcl8.4"][..],
            &["while {"][..],
        ),
        (
            "proc p {} {if {1e308 * 10 > 0} {puts inf} else {puts finite}}\np\n",
            &["tcl8.4"][..],
            &["if {"][..],
        ),
        (
            rebound,
            &["tcl8.6", "tcl9.0"][..],
            &["if {abs(-2) == 2}", "while {"][..],
        ),
    ];
    for (program, undecided_under, kept) in programs {
        for dialect in undecided_under {
            let (rewritten, rewrites) = optimised(program, dialect);
            assert!(
                kept.iter().all(|structure| rewritten.contains(structure)),
                "{dialect} decides nothing:\n{rewritten}\n{rewrites:#?}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            let (rewritten, rewrites) = optimised(program, &dialect_of(series));
            assert_eq!(
                run_script(&tclsh, &rewritten),
                run_script(&tclsh, program),
                "tclsh{series}:\n{rewritten}\n{rewrites:#?}"
            );
        }
    }
}
