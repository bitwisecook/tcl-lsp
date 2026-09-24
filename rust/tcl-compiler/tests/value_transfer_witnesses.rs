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
use tcl_registry::value_transfer::{BindingKind, Existence};

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
/// direct one. No shipped command declares an `EvaluatorCapability`, so
/// `implementation` stays 0 throughout; a pack's declared implementation
/// counts there (`a_declared_implementation_folds_through_the_driver`).
/// An entry is counted only
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
/// Since VT5.7 the loop header answers each binder of the two-binder
/// source with the elements it takes (`a` is `{1 2}`, `b` is `{10 20}`),
/// so each quotient sees the page's two distinct `Finite` identities and
/// declines `CorrelatedSets`, the reason D64 deferred to this slice; `x`
/// and `y` never fold and neither branch decides.
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
        // The two-binder source is lowered (VT5.7): `a` and `b` are two
        // distinct finite inputs, so each quotient declines as correlated.
        let answers = answers_for(&unit, "::p", "expr");
        assert_eq!(
            answers,
            ["declined: correlated-sets", "declined: correlated-sets"],
            "{dialect}"
        );
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

/// The bounded host's stand-in for `tenant::label`'s body `{name} { fold
/// [string cat "tenant:" $name] }`: one exact argument in, `tenant:` before
/// it out, under the release the call is pinned to.
struct LabelHost;

impl tcl_registry::pack_hooks::PackHookHost for LabelHost {
    fn invoke(
        &self,
        _slot: tcl_registry::pack_hooks::HookSlot,
        call: &tcl_registry::pack_hooks::HookCall<'_>,
    ) -> tcl_registry::pack_hooks::HookAnswer {
        use tcl_registry::pack_hooks::{EvaluationAnswer, HookAnswer};
        match call.words {
            [name] if call.dialect == Some("tcl9.0") => HookAnswer::Evaluation(EvaluationAnswer {
                result: Some(format!("tenant:{}", name.value)),
                stores: Vec::new(),
            }),
            _ => HookAnswer::Abstain,
        }
    }
}

/// A `tcl9.0` registry holding `tenant::label` as a pack declares it —
/// `arity 1`, `evaluate -implementation tenant.label.v1 -host bounded_tcl`
/// reading `arg 0 exact` — its body bound to an `evaluate` slot.
fn tenant_label_registry() -> tcl_registry::CommandRegistry {
    use tcl_registry::pack_hooks::{self, HookFamily, HookInput, HookInputs};
    use tcl_registry::spec::CommandSpec;
    use tcl_registry::value_transfer::{
        CompletionSupport, ContextDependency, DeclaredEvaluation, DeclaredImplementation,
        DeclaredInput, DeclaredSemantics, DeclaredStructure, EvaluatorCapability, Exactness,
        HostKind, ImplementationBudget, ImplementationIdentity, Needs, SemanticsDeclaration,
    };

    let slot = pack_hooks::allocate(
        HookFamily::Evaluate,
        &HookInputs::declared([HookInput::Words]),
    )
    .expect("an evaluate slot");
    let label: &'static DeclaredSemantics = Box::leak(Box::new(DeclaredSemantics {
        scope: "tenant::label",
        structure: DeclaredStructure::default(),
        evaluation: DeclaredEvaluation::Implementation(DeclaredImplementation {
            capability: EvaluatorCapability {
                identity: ImplementationIdentity {
                    pack: "tenant",
                    id: "tenant.label.v1",
                    content_hash: 1,
                },
                host: HostKind::BoundedTcl,
                target: Needs::NONE,
                inputs: &[DeclaredInput::Operand {
                    index: 0,
                    exactness: Exactness::Exact,
                }],
                depends: &[
                    ContextDependency::TclProfile,
                    ContextDependency::ImplementationIdentity,
                ],
                budget: ImplementationBudget {
                    commands: Some(2000),
                    wall_clock_ms: Some(20),
                    value_bytes: Some(65536),
                },
                completion: CompletionSupport::NormalOnly,
            },
            slot: Some(slot),
        }),
        option_declines: &[],
    }));
    let mut base = tcl_registry::CommandRegistry::build_default();
    base.insert(CommandSpec {
        name: "tenant::label",
        semantics: SemanticsDeclaration::Declared(label),
        ..CommandSpec::DEFAULT
    });
    base.project_for_profile(tcl_dialect::DialectProfile::find("tcl9.0").expect("the profile"))
}

/// A declared implementation runs through the driver as a shipped route
/// does, from the pack's declaration alone: `tenant::label acme` folds to
/// `tenant:acme` through the body's `fold`, and the entry counts as an
/// implementation; an argument the analysis does not know declines
/// `NotExact` before any body runs; and a worker with no host declines
/// `Transient` — a state of the worker, never a verdict on the inputs. The
/// host here stands in for the bounded one, which runs the body in
/// `tcl-spec-hooks`' own witnesses.
#[test]
fn a_declared_implementation_folds_through_the_driver() {
    use tcl_registry::pack_hooks;

    let registry = tenant_label_registry();
    let source = "proc p {x} {\n\
                  set known [tenant::label acme]\n\
                  set unknown [tenant::label $x]\n\
                  return $known$unknown\n\
                  }\n";
    let routes = |unit: &CompilationUnit| -> Vec<String> {
        unit.procedures
            .get("::p")
            .expect("the procedure")
            .sccp
            .explanations
            .iter()
            .filter(|explanation| explanation.command == "tenant::label")
            .map(|explanation| explanation.route.clone())
            .collect()
    };

    pack_hooks::install_host(std::rc::Rc::new(LabelHost));
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(text("tenant:acme"))
    );
    assert_eq!(
        value_at(&unit, "::p", "unknown", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "tenant::label"),
        ["evaluated", "declined: not-exact"]
    );
    assert_eq!(
        routes(&unit),
        [
            "implementation tenant.label.v1",
            "implementation tenant.label.v1"
        ]
    );
    let tally = unit
        .procedures
        .get("::p")
        .expect("the procedure")
        .sccp
        .route_tally;
    assert_eq!(tally.implementation, 2, "both calls enter the route");
    assert_eq!(tally.direct, 0);

    // The same worker without its host: the known call cannot run, and
    // says so as a transient decline; the unknown one still declines on
    // its input first.
    pack_hooks::clear_host();
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(LatticeValue::Overdefined)
    );
    assert_eq!(
        answers_for(&unit, "::p", "tenant::label"),
        ["declined: transient", "declined: not-exact"]
    );
}

/// The value-transfer lane's executable example (VT4.13): a private command
/// a workspace pack declares under its own name, a second name, and a
/// subcommand form whose operand sits one word later.
const TENANT_PACK: &str = include_str!("fixtures/value_transfers/tenant.tclspec");

/// The example's three spellings, each taking the name as its last word.
const TENANT_SPELLINGS: [&str; 3] = ["tenant::label", "tenant::tag", "tenant label"];

/// What a vendor runtime provides for the example: the three spellings as
/// real commands, written for every release from 8.4 (no `string cat`, no
/// `namespace ensemble`), so a program the analysis never sees the
/// definitions of runs under each `tclsh`.
const TENANT_RUNTIME: &str = "namespace eval ::tenant {\n\
                              \x20   proc label {name} {return \"tenant:$name\"}\n\
                              \x20   proc tag {name} {return \"tenant:$name\"}\n\
                              }\n\
                              proc ::tenant {subcommand name} {\n\
                              \x20   if {$subcommand ne \"label\"} {error \"bad subcommand $subcommand\"}\n\
                              \x20   return [::tenant::label $name]\n\
                              }\n";

/// Serialises the tests that publish a hook plan: the published plan is the
/// process's, and a thread whose host was built from a plan follows it.
static PUBLISHED_PACKS: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A workspace pack of `source` loaded as the language server and the CLI
/// load one: the pack set, its hook plan published and this thread's host
/// built from it.
fn pack_workspace(name: &str, source: &str) -> tcl_spectcl::PackSet {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: std::path::PathBuf::from(format!("/workspace/.tcl-lsp/{name}.tclspec")),
            origin: tcl_spectcl::discovery::Origin::DotDir,
        },
        source.to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    tcl_spectcl::hooks::publish(&packs);
    tcl_spectcl::hooks::ensure_thread_host();
    packs
}

/// The example loaded as a workspace loads it: the pack set, its hook plan
/// published and this thread's host built from it, as the language server
/// and the CLI do on a pack load.
fn tenant_workspace() -> tcl_spectcl::PackSet {
    pack_workspace("tenant", TENANT_PACK)
}

/// Program (2) of the interface page (VT5.6): `binary format` declares a
/// registry-owned route, so `set h [binary format H* 414243444546]` is
/// `ABCDEF` in the shared lattice under every profile, typed a byte array by
/// construction, and neither S100 nor S110 reports a conversion for it. A
/// computed byte array has no lossless source spelling, so no rewrite writes
/// it into the program (`SccpResult::materialises`): the optimised program
/// keeps the command that builds it and prints what every release prints.
#[test]
fn program_two_folds_and_is_a_byte_array() {
    use tcl_compiler::shimmer::{find_byte_array_warnings_for_cu, find_shimmer_warnings_for_cu};
    use tcl_compiler::value_transfer::FoldedType;
    use tcl_registry::TclType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let source = "proc p {} {\n    set h [binary format H* 414243444546]\n    puts $h\n}\n\
                  proc b {} {\n    set g [binary format H* 4748]\n    return $g\n}\np\nputs [b]\n";
    let byte_array = FoldedType {
        intrep: Some(TclType::ByteArray),
        shape: None,
        representation: RepresentationEvidence::Constructed(TclType::ByteArray),
    };
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "h", 1),
            Some(text("ABCDEF")),
            "{dialect}"
        );
        assert_eq!(
            folded_at(&unit, "::p", "h", 1),
            Some(byte_array.clone()),
            "{dialect}"
        );
        let registry = static_context_for(dialect).commands();
        let shimmers = find_shimmer_warnings_for_cu(&unit, registry);
        assert!(shimmers.is_empty(), "{dialect}: {shimmers:?}");
        let damage = find_byte_array_warnings_for_cu(&unit, registry);
        assert!(damage.is_empty(), "{dialect}: {damage:?}");
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            !rewritten.contains("ABCDEF") && !rewritten.contains("GH"),
            "{dialect}: a byte array was written into the source:\n{rewritten}"
        );
    }
    prints_under_every_release(source, "ABCDEF\nGH\n");
}

/// The folded type of `var`'s version `version` in `proc`: the semantic
/// type and representation evidence the evaluation that produced it stated.
fn folded_at(
    unit: &CompilationUnit,
    proc: &str,
    var: &str,
    version: u32,
) -> Option<tcl_compiler::value_transfer::FoldedType> {
    let function = unit.procedures.get(proc).expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.folded_types.get(&(symbol, version)).cloned()
}

/// VT5.2: the shared lattice keeps what each evaluation states of its
/// value's type beside the value itself (`SccpResult::folded_types`). A
/// result and a write carry the type facts and the representation the route
/// constructed — `string length` and `incr` build an int, `list` a list,
/// `append` a string; a copy shares its source's; a φ keeps what its arms
/// state alike; a literal states nothing; a barrier widens every value and
/// forgets what they stated. A pack's declared implementation states its
/// `result -semantic` type and no representation, and the type lattice
/// takes it where the static typing knows nothing of a pack command's
/// result.
#[test]
fn folded_types_state_what_each_route_constructed() {
    use tcl_compiler::value_transfer::FoldedType;
    use tcl_registry::TclType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let built = |ty: TclType| {
        Some(FoldedType {
            intrep: Some(ty),
            shape: None,
            representation: RepresentationEvidence::Constructed(ty),
        })
    };
    let unit = unit_of(
        "proc p {c} {\n    set s abc\n    set n [string length $s]\n    set l [list a b]\n    \
         incr n\n    append s x\n    set m $n\n    \
         if {$c} {set k [llength $l]} else {set k [string length $s]}\n    \
         return $m$k$l$s\n}\n\
         proc b {} {\n    set s abc\n    set n [string length $s]\n    return -code ok $n\n}\n",
        "tcl9.0",
    );
    assert_eq!(folded_at(&unit, "::p", "s", 1), None, "a literal");
    assert_eq!(folded_at(&unit, "::p", "n", 1), built(TclType::Int));
    assert_eq!(folded_at(&unit, "::p", "l", 1), built(TclType::List));
    assert_eq!(folded_at(&unit, "::p", "n", 2), built(TclType::Int), "incr");
    assert_eq!(
        folded_at(&unit, "::p", "s", 2),
        built(TclType::String),
        "append"
    );
    assert_eq!(
        folded_at(&unit, "::p", "m", 1),
        built(TclType::Int),
        "a copy"
    );
    assert_eq!(
        folded_at(&unit, "::p", "k", 1),
        built(TclType::Int),
        "the φ"
    );
    assert_eq!(
        built(TclType::ByteArray)
            .map(|folded| folded.label())
            .as_deref(),
        Some("bytearray (constructed)")
    );
    let barrier = unit.procedures.get("::b").expect("the procedure");
    assert!(
        barrier.sccp.folded_types.is_empty(),
        "{:?}",
        barrier.sccp.folded_types
    );

    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = tenant_workspace();
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    let unit = CompilationUnit::build_for_dialect(
        "proc p {} {\n    set r [tenant::label acme]\n    return $r\n}\n",
        &registry,
        false,
        "tcl9.0",
    );
    let stated = Some(FoldedType {
        intrep: Some(TclType::String),
        shape: None,
        representation: RepresentationEvidence::Unknown,
    });
    assert_eq!(folded_at(&unit, "::p", "r", 1), stated);
    let function = unit.procedures.get("::p").expect("the procedure");
    let r = function.ssa.var_symbol("r").expect("the variable");
    assert_eq!(
        function.types.get(&(r, 1)),
        Some(&tcl_compiler::types::TypeLattice::of(TclType::String)),
        "the type lattice takes the declared result type"
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// The step-1 completion test (`docs/design/compiler/value-transfers.md`
/// § *The completion test*): one private command, renamed, and given a
/// subcommand form with its operand one word later, reaches the analysis
/// and the optimiser from its pack's declarations alone — the fixture is the
/// only file that names any of the three spellings.
///
/// - Each spelling folds `acme` to `tenant:acme` through the analysis, on
///   the implementation route, and an argument the analysis does not know
///   declines `not-exact`; without the pack the same program folds nothing,
///   so the answer is the declaration's.
/// - The body runs under the analysed release: it folds under 8.6, 9.0 and
///   9.1, and under 8.4, 8.5 and iRules' 8.4 base it raises and declines,
///   because `string cat` is 8.6's — the releases `tclsh` itself runs `puts
///   [string cat tenant: acme]` under. A profile naming no release has no
///   engine to run it on and declines.
/// - The optimiser forwards the folded constant (O100), and the vendor
///   runtime — [`TENANT_RUNTIME`], which the analysis never sees — prints
///   what the optimised program prints under every `tclsh` on `PATH`.
#[test]
fn the_completion_test_needs_no_consumer_edit() {
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = tenant_workspace();
    for (dialect, folds) in [
        ("tcl8.4", false),
        ("tcl8.5", false),
        ("tcl8.6", true),
        ("tcl9.0", true),
        ("tcl9.1", true),
        ("f5-irules", false),
        ("tcl", false),
    ] {
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, &packs);
        for spelling in TENANT_SPELLINGS {
            every_spelling_answers(&registry, dialect, spelling, folds);
        }
    }
    the_vendor_runtime_prints_what_the_optimiser_forwards(&packs);
}

/// `spelling acme` and `spelling $x` in one procedure under `dialect`: both
/// calls enter the implementation route; the known argument folds to
/// `tenant:acme` where the body runs (`folds`) and declines `unsupported`
/// where it cannot; the unknown one declines `not-exact`. The same program
/// against the registry without the pack folds nothing.
fn every_spelling_answers(
    registry: &tcl_registry::CommandRegistry,
    dialect: &str,
    spelling: &str,
    folds: bool,
) {
    let head = spelling.split(' ').next().expect("a head");
    let source = format!(
        "proc p {{x}} {{\n    set known [{spelling} acme]\n    set unknown [{spelling} $x]\n    return $known$unknown\n}}\n"
    );
    let unit = CompilationUnit::build_for_dialect(&source, registry, false, dialect);
    let function = unit.procedures.get("::p").expect("the procedure");
    let routes: Vec<&str> = function
        .sccp
        .explanations
        .iter()
        .filter(|explanation| explanation.command == head)
        .map(|explanation| explanation.route.as_str())
        .collect();
    assert_eq!(
        routes,
        [
            "implementation tenant.label.v1",
            "implementation tenant.label.v1"
        ],
        "{dialect} {spelling}"
    );
    let (value, answer) = if folds {
        (text("tenant:acme"), "evaluated")
    } else {
        (LatticeValue::Overdefined, "declined: unsupported")
    };
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(value),
        "{dialect} {spelling}"
    );
    assert_eq!(
        value_at(&unit, "::p", "unknown", 1),
        Some(LatticeValue::Overdefined),
        "{dialect} {spelling}"
    );
    assert_eq!(
        answers_for(&unit, "::p", head),
        [answer, "declined: not-exact"],
        "{dialect} {spelling}"
    );
    assert_eq!(
        function.sccp.route_tally.implementation, 2,
        "{dialect} {spelling}: both calls enter the route"
    );
    let bare = static_context_for(dialect).commands();
    let unit = CompilationUnit::build_for_dialect(&source, bare, false, dialect);
    assert_eq!(
        value_at(&unit, "::p", "known", 1),
        Some(LatticeValue::Overdefined),
        "{dialect} {spelling}: without the pack the name is nobody's"
    );
}

/// The optimiser forwards each spelling's folded constant (O100) where the
/// body runs and rewrites nothing where it cannot; `tclsh` runs the body's
/// `string cat` exactly where the analysis folds; and with the vendor
/// runtime ([`TENANT_RUNTIME`]) the original and the optimised program
/// print the same under every `tclsh` on `PATH`.
fn the_vendor_runtime_prints_what_the_optimiser_forwards(packs: &tcl_spectcl::PackSet) {
    let program = "proc p {} {\n    set a [tenant::label acme]\n    set b [tenant::tag acme]\n    set c [tenant label acme]\n    puts $a\n    puts $b\n    puts $c\n}\np\n";
    let optimise = |dialect: &str| {
        let registry = tcl_spectcl::install::registry_for_dialect_with_packs(dialect, packs);
        let profile = resolve_environment(dialect).analyser_profile();
        optimise_source_multipass(program, &registry, Some(profile), PASSES)
    };
    let (rewritten, rewrites) = optimise("tcl9.0");
    assert_eq!(
        rewritten.matches("puts tenant:acme").count(),
        3,
        "{rewritten}\n{rewrites:#?}"
    );
    assert!(
        rewrites
            .iter()
            .any(|rewrite| rewrite.code.as_str() == "O100"),
        "{rewrites:#?}"
    );
    let (unchanged, _) = optimise("tcl8.4");
    assert!(
        !unchanged.contains("puts tenant:acme"),
        "the body cannot run under 8.4:\n{unchanged}"
    );
    for (series, tclsh) in releases_on_path() {
        let body_runs = run_script(&tclsh, "puts [string cat tenant: acme]")
            == Some((true, "tenant:acme\n".to_owned()));
        assert_eq!(
            body_runs,
            matches!(series, "8.6" | "9.0" | "9.1"),
            "tclsh{series} runs the body exactly where the analysis folds"
        );
        let (rewritten, rewrites) = optimise(&dialect_of(series));
        for candidate in [program, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, &format!("{TENANT_RUNTIME}{candidate}")),
                Some((true, "tenant:acme\ntenant:acme\ntenant:acme\n".to_owned())),
                "tclsh{series}:\n{candidate}\n{rewrites:#?}"
            );
        }
    }
}

/// A pack command that writes the variable it names, from that variable's
/// incoming value: `acc::add VAR PIECE` appends `PIECE` to what `VAR`
/// holds. Its `write` needs the variable's prior value as an input
/// (`target 0 incoming`), which the analysis can read only where the
/// lowering records a use of it: the word must carry the `VarWrite` role
/// and the command the `READS_BEFORE_WRITE` trait. `acc::put` declares the
/// same body without the trait, and `acc::store VAR VALUE` writes its
/// argument without reading the variable.
const ACC_PACK: &str = "speclib acc 2.2 {
    command acc::add {
        arity 2
        arg 0 -role VarWrite
        traits {READS_BEFORE_WRITE}
        semantics {
            stores -targets {0} -outcome write
            result -semantic string
        }
        evaluate -implementation acc.add.v1 -host bounded_tcl {
            inputs {target 0 incoming arg 1 exact}
            body {prior piece} { set joined $prior$piece; write 0 $joined; fold $joined }
        }
    }
    command acc::put {
        arity 2
        arg 0 -role VarWrite
        semantics {
            stores -targets {0} -outcome write
            result -semantic string
        }
        evaluate -implementation acc.put.v1 -host bounded_tcl {
            inputs {target 0 incoming arg 1 exact}
            body {prior piece} { set joined $prior$piece; write 0 $joined; fold $joined }
        }
    }
    command acc::store {
        arity 2
        arg 0 -role VarWrite
        semantics {
            stores -targets {0} -outcome write
            result -semantic string
        }
        evaluate -implementation acc.store.v1 -host bounded_tcl {
            inputs {arg 1 exact}
            body {value} { write 0 $value; fold $value }
        }
    }
}
";

/// A pack's `write` through an incoming target reaches the real driver in
/// statement position: `acc::add s cd` over `s` = `ab` leaves `abcd` in the
/// next version of `s`, run by the tclvm host from the loaded pack. In value
/// position neither shape is a value: the read of `s` inside `[acc::add s
/// ef]` sits on the synthetic call ahead of its host, as a nested `[incr
/// x]`'s does, so the incoming target is not exact there; and `[acc::store u
/// xy]`, which reads nothing, evaluates but writes storage the substitution
/// has no definition for, so it is not substituted. Without
/// `READS_BEFORE_WRITE` nothing records the variable's prior value at the
/// call, so the incoming target is not exact and `acc::put` declines.
#[test]
fn a_pack_write_through_an_incoming_target_reaches_the_driver() {
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("acc", ACC_PACK);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    let source = "proc p {} {\n    set s ab\n    acc::add s cd\n    set r [acc::add s ef]\n    \
                  set w [acc::store u xy]\n    return $s$r$w\n}\n\
                  proc q {} {\n    set t ab\n    acc::put t cd\n    return $t\n}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    assert_eq!(
        value_at(&unit, "::p", "s", 2),
        Some(text("abcd")),
        "the statement's write lands on the next version of `s`"
    );
    for value in ["r", "w"] {
        assert_eq!(
            value_at(&unit, "::p", value, 1),
            Some(LatticeValue::Overdefined),
            "`{value}`: a writing call is never a value in value position"
        );
    }
    assert_eq!(
        answers_for(&unit, "::p", "acc::add"),
        ["evaluated", "declined: not-exact"]
    );
    assert_eq!(
        answers_for(&unit, "::p", "acc::store"),
        ["not substituted: the outcome writes storage"]
    );
    assert_eq!(
        value_at(&unit, "::q", "t", 2),
        Some(LatticeValue::Overdefined),
        "no use of `t` is recorded at the call without the trait"
    );
    assert_eq!(
        answers_for(&unit, "::q", "acc::put"),
        ["declined: not-exact"]
    );
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// The no-match preserve (VT5.11, #2051's program): a `regexp` that cannot
/// match leaves its match variables as they were, so the store feeding one
/// stays — no O109 deletes it, no W220 calls it unread, no W210 reports the
/// read — and the original and optimised programs print `before` under
/// every release. The same commands in a condition or a word keep the
/// stores their targets may preserve: `tcl opt` had deleted `set v before`
/// from both, and the optimised programs failed with `can't read "v": no
/// such variable`.
#[test]
fn a_no_match_keeps_the_store_it_preserves() {
    use tcl_compiler::analyser::Analyser;
    let programs = [
        (
            "proc p {} {\n    set a before\n    regexp {(x)(y)} zz a b\n    puts $a\n}\np\n",
            "a",
            "before\n",
        ),
        (
            "proc q {s} {\n    set v before\n    if {[regexp {(x)} $s -> v]} {\n        \
             puts matched\n    }\n    puts $v\n}\nq abc\n",
            "v",
            "before\n",
        ),
        (
            "proc r {} {\n    set v before\n    puts [regexp {x} y v]\n    puts $v\n}\nr\n",
            "v",
            "0\nbefore\n",
        ),
    ];
    for (source, kept, printed) in programs {
        let named = format!("'{kept}'");
        for dialect in DIALECTS {
            let rewrites = rewrites_of(source, dialect);
            assert!(
                !rewrites
                    .iter()
                    .any(|rewrite| rewrite.code == DiagCode::O109),
                "{dialect}: {source}{rewrites:#?}"
            );
            let reported: Vec<(DiagCode, String)> = Analyser::new()
                .analyse(source, dialect)
                .diagnostics
                .into_iter()
                .filter(|d| matches!(d.code, DiagCode::W210 | DiagCode::W220))
                .map(|d| (d.code, d.message))
                .collect();
            assert!(
                !reported.iter().any(|(_, message)| message.contains(&named)),
                "{dialect}: {source}{reported:?}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// A pack command's declared preserve is a preserved definition like a
/// builtin's (VT5.11): `keep::miss VAR PIECE` declares `write_or_preserve`
/// on its target and its body preserves it, so the definition holds the
/// version before the call — the undefined root in `p`, the `set` in `q` —
/// which is what W210 reads, though the command carries no trait that
/// records a use of that version.
#[test]
fn a_pack_declared_preserve_holds_the_prior_version() {
    const KEEP_PACK: &str = "speclib keep 2.2 {
    command keep::miss {
        arity 2
        arg 0 -role VarWrite
        semantics {
            stores -targets {0} -outcome write_or_preserve
            result -semantic int
        }
        evaluate -implementation keep.miss.v1 -host bounded_tcl {
            inputs {arg 1 exact}
            body {piece} { preserve 0; fold 0 }
        }
    }
}
";
    let _published = PUBLISHED_PACKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let packs = pack_workspace("keep", KEEP_PACK);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl9.0", &packs);
    let source = "proc p {} {\n    keep::miss v x\n    return $v\n}\n\
                  proc q {} {\n    set v before\n    keep::miss v x\n    return $v\n}\n";
    let unit = CompilationUnit::build_for_dialect(source, &registry, false, "tcl9.0");
    for (proc, prior) in [("::p", 0), ("::q", 1)] {
        let function = unit.procedures.get(proc).expect("the procedure");
        let symbol = function.ssa.var_symbol("v").expect("the variable");
        assert_eq!(
            function.sccp.preserved.get(&(symbol, prior + 1)),
            Some(&prior),
            "{proc}: {:?}",
            function.sccp.preserved
        );
        assert_eq!(answers_for(&unit, proc, "keep::miss"), ["evaluated"]);
    }
    tcl_spectcl::hooks::publish(&tcl_spectcl::PackSet::default());
}

/// A materialised child carries the span of the factory call that produced
/// it (VT5.9, #2143): `Configure port 8080 {the port}` materialises `proc
/// port {x} {return 8080}`, and the child's W214 for `x` anchors at that
/// call — line 4 — where, with no span of its own, it anchored at 1:1. The
/// factory's template is read through its template-word plan, and `port
/// ignored` is `8080` under tclsh 8.4 to 9.1 before and after `tcl opt`.
#[test]
fn a_materialised_child_carries_its_factory_call_span() {
    use tcl_compiler::analyser::Analyser;
    let source = "proc Configure {name default description} {\n    \
                  proc $name {x} [subst -nocommands {return $default}]\n}\n\
                  Configure port 8080 {the port}\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let reported = Analyser::new().analyse(source, dialect).diagnostics;
        let lines: Vec<usize> = reported
            .iter()
            .filter(|d| d.code == DiagCode::W214 && d.message.contains("'::port'"))
            .map(|d| source[..d.span.start() as usize].matches('\n').count() + 1)
            .collect();
        assert_eq!(lines, [4], "{dialect}: {reported:?}");
    }
    prints_under_every_release(&format!("{source}puts [port ignored]\n"), "8080\n");
}

/// A computed template that runs commands can read any variable (VT5.10,
/// D155): `subst -novariables $t` over `[set x]` reads `x`, so the store
/// before it stays — the original and optimised programs print `1` under
/// tclsh 8.4 to 9.1, where dropping `set x 1` as unused made them raise.
#[test]
fn a_computed_template_that_runs_commands_keeps_the_stores_it_reads() {
    let source = "proc f {t} {\n    set x 1\n    return [subst -novariables $t]\n}\n\
                  puts [f {[set x]}]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let (rewritten, _) = optimised(source, dialect);
        assert!(rewritten.contains("set x 1"), "{dialect}: {rewritten}");
    }
    prints_under_every_release(source, "1\n");
}

// VT5.19: the slice's exit witnesses (the interface page's § *Test
// anchors*, "fixed witnesses to add"), each read off the shared lattice
// through the memoised unit and checked against `tcl opt`'s rewritten
// program, printed under every release on `PATH`.

/// The no-match preserve witness: a `regexp` that cannot match leaves its
/// match variables exactly as they were — the lattice holds `a`'s prior
/// value at the call's own definition (a `Preserve`, not a manufactured
/// one), and `zz` never matches `(x)(y)`, so both the original and the
/// `tcl opt`-rewritten program print `before` on every release.
#[test]
fn the_no_match_preserve_witness() {
    let source = "proc p {} {\n    set a before\n    regexp {(x)(y)} zz a b\n    puts $a\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "a"),
            text("before"),
            "{dialect}: the no-match leaves $a at its prior value"
        );
    }
    prints_under_every_release(source, "before\n");
}

/// The partial-scan witness: `scan "12" "%d %d" a b` converts one field
/// from the two-integer format before the input is exhausted — `a` takes
/// the converted `12`, `b` is left exactly as it was (a `Preserve`, the
/// same outcome kind the no-match witness reads) — and both agree with
/// `tclsh` before and after `tcl opt`. Run as its own statement, not
/// nested in a `[…]` value position: a nested invocation's stores answer
/// under the effect-free policy `fold_cmd_subst_routes` documents, which
/// has no definition for them to land on, so `a` and `b` would be
/// `Overdefined` for a reason unrelated to what this witness proves.
#[test]
fn the_partial_scan_witness() {
    let source = "proc p {} {\n    set a before\n    set b before\n    \
                  scan \"12\" \"%d %d\" a b\n    puts \"$a $b\"\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "a"),
            LatticeValue::Const(ConstValue::Int(12)),
            "{dialect}: the converted field takes the scanned value, typed as %d built it"
        );
        assert_eq!(
            last_value(source, dialect, "::p", "b"),
            text("before"),
            "{dialect}: the field past the exhausted input is preserved"
        );
    }
    prints_under_every_release(source, "12 before\n");
}

/// The repeated-target witness: a call whose declared targets name the
/// same place twice composes in execution order, so the last position's
/// value wins. `lassign`'s repeated-target form (the interface page's own
/// "`lassign … a a`") is 8.5+ and raises under 8.4, so this reads the same
/// fact through `regexp`'s match-variable list, which every release
/// shares: `(a)(b)` against `ab` captures `a` into `x` and then `b` into
/// `x` again, and `x` ends the call as `b`.
#[test]
fn the_repeated_target_witness() {
    let source = "proc p {} {\n    regexp {(a)(b)} ab -> x x\n    puts $x\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(source, dialect, "::p", "x"),
            text("b"),
            "{dialect}: the second position's capture is the one that survives"
        );
    }
    prints_under_every_release(source, "b\n");
}

/// The existence fact `var`'s last version holds in `proc`: the fact the
/// solver established where the version was defined.
fn last_existence(unit: &CompilationUnit, proc: &str, var: &str) -> Option<Existence> {
    let function = unit.procedures.get(proc).expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function
        .sccp
        .existence
        .iter()
        .filter(|((sym, _), _)| *sym == symbol)
        .max_by_key(|((_, version), _)| *version)
        .map(|(_, fact)| *fact)
}

/// What `tclsh` reports of `places` after `body` runs in a procedure:
/// `None` when the body raises, else each place's `info exists` and
/// `array exists`.
fn oracle_existence(tclsh: &str, body: &str, places: &[&str]) -> Option<Vec<(bool, bool)>> {
    let mut probes = String::new();
    for place in places {
        probes.push_str(" [info exists ");
        probes.push_str(place);
        probes.push_str("] [array exists ");
        probes.push_str(place);
        probes.push(']');
    }
    let script = format!(
        "proc p {{}} {{\n{body}\nreturn [list{probes}]\n}}\n\
         if {{[catch p answer]}} {{puts -nonewline error}} else {{puts -nonewline $answer}}\n"
    );
    let (ok, output) = run_script(tclsh, &script)?;
    assert!(ok, "{tclsh} ran the probe for {body:?}");
    if output == "error" {
        return None;
    }
    let bits: Vec<bool> = output.split_whitespace().map(|bit| bit == "1").collect();
    Some(bits.chunks(2).map(|pair| (pair[0], pair[1])).collect())
}

/// Whether `fact` agrees with what `tclsh` reports of a place: a proven
/// binding exists, as an array exactly when it is one, and a proven
/// absence does not; a fact that proves neither claims nothing.
fn existence_agrees(fact: Option<Existence>, (exists, array): (bool, bool)) -> bool {
    match fact {
        Some(Existence::Unbound) => !exists && !array,
        Some(Existence::Bound(BindingKind::Scalar)) => exists && !array,
        Some(Existence::Bound(BindingKind::Array)) => exists && array,
        Some(Existence::Bound(BindingKind::Either)) => exists,
        Some(Existence::MayBound | Existence::Pending) | None => true,
    }
}

const SCALAR: Option<Existence> = Some(Existence::Bound(BindingKind::Scalar));
const ARRAY: Option<Existence> = Some(Existence::Bound(BindingKind::Array));
const UNBOUND: Option<Existence> = Some(Existence::Unbound);

/// The dialects the release table is analysed under: each release, and
/// the `tcl` profile that spans them all.
const RELEASE_DIALECTS: [&str; 6] = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "tcl"];

/// The release table's lines every release reads alike: each place's fact
/// after the line, under every dialect, and against every `tclsh` on
/// `PATH` wherever the line completes.
fn the_lines_every_release_reads_alike(releases: &[(&'static str, String)]) {
    type Row = (&'static str, &'static [(&'static str, Option<Existence>)]);
    let alike: [Row; 9] = [
        ("append s foo", &[("s", SCALAR)]),
        ("lappend l foo", &[("l", SCALAR)]),
        ("regexp {(x)(y)} zz a b", &[("a", UNBOUND), ("b", UNBOUND)]),
        (
            "scan {12 nope} {%d %d} a b",
            &[("a", SCALAR), ("b", UNBOUND)],
        ),
        ("unset nosuch", &[("nosuch", UNBOUND)]),
        ("unset -nocomplain nosuch", &[("nosuch", UNBOUND)]),
        (
            "set arr(k) 1; unset arr(k)",
            &[("arr", ARRAY), ("arr(k)", UNBOUND)],
        ),
        ("set x 1; unset x; set x 2", &[("x", SCALAR)]),
        ("foreach x {} {}", &[("x", UNBOUND)]),
    ];
    for (body, places) in alike {
        let source = format!("proc p {{}} {{{body}}}\n");
        for dialect in RELEASE_DIALECTS {
            let unit = unit_of(&source, dialect);
            for &(place, expected) in places {
                assert_eq!(
                    last_existence(&unit, "::p", place),
                    expected,
                    "{dialect}: `{place}` after `{body}`"
                );
            }
        }
        let names: Vec<&str> = places.iter().map(|(place, _)| *place).collect();
        for (series, tclsh) in releases {
            let Some(answers) = oracle_existence(tclsh, body, &names) else {
                continue;
            };
            let unit = unit_of(&source, &dialect_of(series));
            for (&(place, _), answer) in places.iter().zip(answers) {
                assert!(
                    existence_agrees(last_existence(&unit, "::p", place), answer),
                    "tclsh{series}: `{place}` after `{body}` is {answer:?}"
                );
            }
        }
    }
    for dialect in RELEASE_DIALECTS {
        assert_eq!(
            last_value("proc p {} {append s foo}\n", dialect, "::p", "s"),
            text("foo"),
            "{dialect}: `append` creates its cell in every release"
        );
        assert_eq!(
            last_value("proc p {} {lappend l foo}\n", dialect, "::p", "l"),
            text("foo"),
            "{dialect}: `lappend` creates its cell in every release"
        );
    }
}

/// The release table's `incr` lines: the cell is created from 8.5, with
/// the amount as its value, and the value declines as an unbound place
/// under 8.4 and under the spanning profile, where `tclsh8.4` raises.
fn the_increment_split(releases: &[(&'static str, String)]) {
    let increments: [(&str, &str, i64); 3] = [
        ("incr fresh", "fresh", 1),
        ("incr fresh 2", "fresh", 2),
        ("incr arr(k)", "arr(k)", 1),
    ];
    for (body, place, amount) in increments {
        let source = format!("proc p {{}} {{{body}}}\n");
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                last_value(&source, dialect, "::p", place),
                LatticeValue::Const(ConstValue::Int(amount)),
                "{dialect}: `{body}` creates its cell"
            );
            assert_eq!(last_existence(&unit, "::p", place), SCALAR, "{dialect}");
            if place == "arr(k)" {
                assert_eq!(last_existence(&unit, "::p", "arr"), ARRAY, "{dialect}");
            }
        }
        for dialect in ["tcl8.4", "tcl"] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                last_value(&source, dialect, "::p", place),
                LatticeValue::Overdefined,
                "{dialect}: `{body}` raises under 8.4, so its value declines"
            );
            let answers = answers_for(&unit, "::p", "incr");
            assert!(
                answers
                    .iter()
                    .any(|answer| answer == "declined: unbound-place"),
                "{dialect}: `{body}` declines as an unbound place: {answers:?}"
            );
        }
        for (series, tclsh) in releases {
            let answer = oracle_existence(tclsh, body, &[place]);
            if *series == "8.4" {
                assert_eq!(answer, None, "tclsh8.4 raises on `{body}`");
            } else {
                assert_eq!(answer, Some(vec![(true, false)]), "tclsh{series}: `{body}`");
            }
        }
    }
}

/// The page's release table for an absent cell
/// (`docs/design/compiler/value-transfers.md` § *Existence*), every line but
/// the `unset p nosuch q` prefix line, which is slice 10's: each place's
/// existence after the line, and the value a cell update leaves, under
/// each release's dialect and the `tcl` profile that spans them all. An
/// `incr` of an absent place binds from 8.5 and declines under 8.4 and
/// under the spanning profile; `append` and `lappend` bind in every
/// release; a no-match and an exhausted `scan` leave their places as they
/// were; an unbind leaves the place unbound, and an unset element its array
/// bound. Every fact is checked against `tclsh` wherever the line completes.
#[test]
fn the_absent_cell_release_table() {
    let releases = releases_on_path();
    the_lines_every_release_reads_alike(&releases);
    the_increment_split(&releases);
}
