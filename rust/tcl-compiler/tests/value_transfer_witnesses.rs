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
//! (`docs/design/compiler/value-transfers-examples.md`).
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
use tcl_compiler::optimiser::manager::{
    optimise_raw, optimise_source_multipass, optimise_with_dialect,
};
use tcl_compiler::static_loops::{
    DEFAULT_MAX_STATIC_LOOP_ITERS, LoopSemantics, StaticEnv, StaticValue, summarise_for_statement,
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

/// Every release with its reference interpreter, oldest first, as the shared
/// oracle lookup finds it ([`tcl_test_support::witness_tclsh`]): a release
/// with none fails the test where `TCL_LSP_REQUIRE_TCLSH` requires it, and
/// is reported as skipped otherwise.
fn releases_on_path() -> Vec<(&'static str, String)> {
    RELEASES
        .iter()
        .filter_map(|&series| {
            let version = tcl_dialect::TclVersion::from_version_string(series)?;
            let tclsh = tcl_test_support::witness_tclsh(version)?;
            Some((series, tclsh.path.to_string_lossy().into_owned()))
        })
        .collect()
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
    prints_under_releases_from(source, expected, "8.4");
}

/// [`prints_under_every_release`] for the releases from `first` on: a
/// program using an option a release lacks runs only where it exists.
fn prints_under_releases_from(source: &str, expected: &str, first: &str) {
    for (series, tclsh) in releases_on_path() {
        if series < first {
            continue;
        }
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

/// A callee that only reads a global keeps the store it reads: O109 does not
/// delete `set hits 0` ahead of `show` — the rewrite keeps both stores, and
/// tclsh 8.4 to 9.1 print 0 then 1 for both programs.
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
/// route (the Tier-1 list); `[set x 10]` writes it, so the statement's call
/// holds the write and its host the result: `x` is 10 after the statement and
/// so is `w`, which tclsh 8.4 to 9.1 print, before and after the optimiser.
#[test]
fn a_value_position_set_reads_the_variable() {
    let source = "proc p {} {set x hello; set r [set x]; set w [set x 10]; return $r$w}\n";
    let ten = Some(LatticeValue::Const(ConstValue::Int(10)));
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "f5-irules"] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "r", 1),
            Some(text("hello")),
            "{dialect}"
        );
        assert_eq!(value_at(&unit, "::p", "w", 1), ten, "{dialect}");
        assert_eq!(value_at(&unit, "::p", "x", 2), ten, "{dialect}");
    }
    prints_under_every_release(&format!("{source}puts [p]\n"), "hello10\n");
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

/// A seed as the literal ingress reads it: a canonical integer is the
/// integer, and any other text is its text.
fn seeded(text: &str) -> StaticValue {
    tcl_registry::value_transfer::ExactValue::from_literal(text)
        .as_int()
        .map_or_else(|| StaticValue::Str(text.to_owned()), StaticValue::Int)
}

/// The loop enumeration's answer for `x` after `statement` runs once as the
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
    seed.insert("x".to_owned(), seeded(init));
    seed.insert("n".to_owned(), seeded("3"));
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
/// statement reads no `n`; the route proves the error and answers it as the
/// completion it is, after no store. Read as a substitution it folded to 4.
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
            ["evaluated: error after 0 stores"],
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
/// bignums, and target release ambiguity.
/// Each case is `proc p {} {<prelude>; set r [<expr call>]}`, oracle
/// values checked against `tclsh8.4` to `tclsh9.1` directly.
/// `expression_witnesses_match_every_release_on_path` re-runs the same
/// programs against the real interpreters. `2**64` and `1 << 70` are
/// beyond-wide from 8.5: `tclsh8.4` raises for the first (`**` is not an
/// 8.4 operator) and wraps to 0 for the second, so both decline under
/// `tcl8.4` (`WrongRepresentation`) and under `f5-irules`, whose runtime
/// base is 8.4's; `"010" + 0` reads the leading zero as octal up to
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
/// unsound. The loop header answers each binder of the two-binder source
/// with the elements it takes (`a` is `{1 2}`, `b` is `{10 20}`), so each
/// quotient sees the page's two distinct `Finite` identities and declines
/// `CorrelatedSets`, and the version each loop leaves `x` and `y` at holds
/// no value of its own. Only ordered enumeration answers what the loops
/// leave — `x` is 10 and `y` is 5, each loop's last quotient — which the
/// solver states on each loop's exit, so each post-loop branch decides
/// false: tclsh 8.4 to 9.1 print `other` twice.
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
            let decided = function
                .sccp
                .constant_branches
                .iter()
                .find(|branch| branch.condition == condition)
                .unwrap_or_else(|| {
                    panic!(
                        "{dialect}: {condition} undecided: {:?}",
                        function.sccp.constant_branches
                    )
                });
            assert!(!decided.value, "{dialect}: {condition}");
        }
        // The two-binder source is lowered: `a` and `b` are two
        // distinct finite inputs, so each quotient declines as correlated.
        let answers = answers_for(&unit, "::p", "expr");
        assert_eq!(
            answers,
            ["declined: correlated-sets", "declined: correlated-sets"],
            "{dialect}"
        );
    }
    prints_under_every_release(&format!("{source}p\n"), "other\nother\n");
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
    for (series, tclsh) in releases_on_path() {
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

/// The value-transfer design's executable example: a private command
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
            dependency_tier: None,
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

/// Program (2) of the interface page: `binary format` declares a
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

/// The shared lattice keeps what each evaluation states of its
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

/// The no-match preserve (#2051's program): a `regexp` that cannot
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
/// builtin's: `keep::miss VAR PIECE` declares `write_or_preserve`
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
/// it (#2143): `Configure port 8080 {the port}` materialises `proc
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

/// A computed template that runs commands can read any variable:
/// `subst -novariables $t` over `[set x]` reads `x`, so the store
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

// The interface page's exit witnesses (its § *Test
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

/// The conversion-count witness: `%n` is a
/// conversion for `scan`'s underflow, as C's `nconversions` counts it, so
/// `scan "" %n%d n a` is 1 and writes `n` the characters consumed, 0, on
/// every release. The route had answered the underflow's -1 and preserved
/// `n`, so `tcl opt` rewrote `puts $n` to `puts 5` and W210 reported the
/// read of an `n` the call binds.
#[test]
fn the_percent_n_count_witness() {
    use tcl_compiler::analyser::Analyser;
    let kept = "proc p {} {\n    set n 5\n    scan \"\" %n%d n a\n    puts $n\n}\np\n";
    let bound = "proc q {} {\n    scan \"\" %n%d n a\n    puts $n\n}\nq\n";
    for dialect in DIALECTS {
        assert_eq!(
            last_value(kept, dialect, "::p", "n"),
            LatticeValue::Const(ConstValue::Int(0)),
            "{dialect}: `%n` writes the characters consumed"
        );
        let (rewritten, rewrites) = optimised(kept, dialect);
        assert!(
            !rewritten.contains("puts 5"),
            "{dialect}: the scan wrote `n`\n{rewritten}\n{rewrites:#?}"
        );
        for source in [kept, bound] {
            let reported: Vec<String> = Analyser::new()
                .analyse(source, dialect)
                .diagnostics
                .into_iter()
                .filter(|d| d.code == DiagCode::W210 && d.message.contains("'n'"))
                .map(|d| d.message)
                .collect();
            assert!(reported.is_empty(), "{dialect}: {source}{reported:?}");
        }
    }
    prints_under_every_release(kept, "0\n");
    prints_under_every_release(bound, "0\n");
}

/// `const` writes only an absent place: the first `const c 5`
/// writes 5 into the absent `c` and the second keeps it — tclsh 9.0 and
/// 9.1 print 5 twice — so the lattice holds 5 after the first and no value
/// after the second, and `tcl opt` never prints 7. `set x 1; const x 2`
/// raises on both releases, and the route declines it.
#[test]
fn const_writes_only_an_absent_place() {
    let source = "proc p {} {\n    const c 5\n    puts $c\n    const c 7\n    puts $c\n}\np\n";
    let raises = "proc q {} {\n    set x 1\n    const x 2\n}\nq\n";
    for dialect in ["tcl9.0", "tcl9.1"] {
        let unit = unit_of(source, dialect);
        assert_eq!(
            value_at(&unit, "::p", "c", 1).and_then(lattice_text),
            Some("5".to_owned()),
            "{dialect}: the absent place takes the value"
        );
        assert_eq!(
            value_at(&unit, "::p", "c", 2).and_then(lattice_text),
            None,
            "{dialect}: an existing constant keeps a value the route does not model"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts 7"),
            "{dialect}: {rewritten}\n{rewrites:#?}"
        );
        assert!(
            answers_for(&unit_of(raises, dialect), "::q", "const")
                .iter()
                .all(|answer| answer.starts_with("declined")),
            "{dialect}: an existing variable declines"
        );
    }
    for (series, tclsh) in releases_on_path() {
        if !series.starts_with('9') {
            continue;
        }
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, "5\n5\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
        assert_eq!(
            run_script(&tclsh, raises).map(|(ok, _)| ok),
            Some(false),
            "tclsh{series}: const over an existing variable raises"
        );
    }
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
/// the amount as its value; under 8.4, where `tclsh8.4` raises, the route
/// answers the error after no store, and under the spanning profile, which
/// cannot say which release runs, the value declines as an unbound place.
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
        for (dialect, answer) in [
            ("tcl8.4", "evaluated: error after 0 stores"),
            ("tcl", "declined: unbound-place"),
        ] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                last_value(&source, dialect, "::p", place),
                LatticeValue::Overdefined,
                "{dialect}: `{body}` raises under 8.4, so its value declines"
            );
            let answers = answers_for(&unit, "::p", "incr");
            assert!(
                answers.iter().any(|found| found == answer),
                "{dialect}: `{body}` answers {answer}: {answers:?}"
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
/// the `unset p nosuch q` prefix line, whose stores an error leaves behind
/// are not modelled: each place's
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

/// Whether one pass of the optimiser under `dialect` deletes the statement
/// spelled `store` in `source` (O108, O109 or O126).
fn removes_store(source: &str, dialect: &str, store: &str) -> bool {
    let at = u32::try_from(source.find(store).expect("the store")).expect("an offset");
    rewrites_of(source, dialect).iter().any(|rewrite| {
        matches!(
            rewrite.code,
            DiagCode::O108 | DiagCode::O109 | DiagCode::O126
        ) && rewrite.span.start() <= at
            && at < rewrite.span.end()
    })
}

/// O109 keeps a store an existence read observes (#2132): while the
/// read stands, no pass deletes the store behind it — the item's two
/// programs keep `set x 1` and `incr n` behind `[info exists …]`, and an
/// existence read in a bare statement, a `catch` body, a value word, a
/// `return`, an `expr` word and `array exists` keeps its store, as does an
/// unbind the store feeds: a nested `[unset x]` in an argument, a value
/// word, a condition or a `return` (it raises on an absent `x`), and an
/// `array unset` of a scalar, which leaves the scalar bound. An unbind
/// statement is never removed. Every program the multipass optimiser
/// rewrites, folding a decided read and then the store it no longer
/// needs, prints what the original does under every release on `PATH` —
/// the `incr` program from 8.5, where the original creates `n`; 8.4
/// raises there.
#[test]
fn o109_keeps_a_store_an_existence_read_observes() {
    let set_first = "proc p {} {set x 1; if {[info exists x]} {puts yes}}\np\n";
    let incr_first = "proc p {} {incr n; if {[info exists n]} {puts yes}}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        assert!(!removes_store(set_first, dialect, "set x 1"), "{dialect}");
    }
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(!removes_store(incr_first, dialect, "incr n"), "{dialect}");
    }
    prints_under_every_release(set_first, "yes\n");
    for (series, tclsh) in releases_on_path()
        .into_iter()
        .filter(|(series, _)| *series != "8.4")
    {
        let (rewritten, _) = optimised(incr_first, &dialect_of(series));
        for program in [incr_first, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, "yes\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
    let positions = "proc p1 {} {set x 1; info exists x}\n\
         proc p2 {} {set x 1; catch {info exists x} r; return $r}\n\
         proc p3 {} {set x 1; lappend l [info exists x]; return $l}\n\
         proc p4 {} {set x 1; return [info exists x]}\n\
         proc p5 {} {set x(a) 1; array exists x}\n\
         proc p6 {} {set x 1; set y [expr {[info exists x] ? \"yes\" : \"no\"}]; return $y}\n\
         proc p7 {} {set x 1; unset x; info exists x}\n\
         proc p8 {c} {set x 1; if {$c} {unset x}; info exists x}\n\
         proc p9 {} {set x 1; lappend l [unset x]; info exists x}\n\
         proc p10 {} {set x 1; set y [unset x]; info exists x}\n\
         proc p11 {} {set x 1; if {[unset x] eq \"\"} {info exists x}}\n\
         proc p12 {} {set x 1; return [unset x]}\n\
         proc p13 {} {set x 1; array unset x; return $x}\n\
         proc p14 {} {set x 1; lappend l [array unset x]; info exists x}\n\
         puts [list [p1] [p2] [p3] [p4] [p5] [p6] [p7] [p8 0] [p8 1] \
         [p9] [p10] [p11] [p12] [p13] [p14]]\n";
    let unbinds = positions.matches("unset x").count();
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        for line in positions.lines().filter(|line| line.starts_with("proc ")) {
            let store = if line.contains("set x(a) 1") {
                "set x(a) 1"
            } else {
                "set x 1"
            };
            let one = format!("{line}\n");
            assert!(!removes_store(&one, dialect, store), "{dialect}: {one}");
        }
        let (rewritten, _) = optimised(positions, dialect);
        assert_eq!(
            rewritten.matches("unset x").count(),
            unbinds,
            "{dialect}: an unbind is never removed:\n{rewritten}"
        );
    }
    prints_under_every_release(positions, "1 1 1 1 1 yes 0 1 0 0 0 0 {} 1 1\n");
}

/// The entry rule for a parameter and a never-assigned local (the
/// Existence row): a parameter enters `Bound(Scalar)`, so `[info exists
/// a]` decides true, and a local nothing ever assigns enters `Unbound`, so
/// `[info exists b]` decides false — both inside the fixed point, so
/// neither dead arm survives the optimiser.
#[test]
fn a_parameter_binds_and_a_never_assigned_local_stays_unbound() {
    let source = "proc p {a} {\n    if {[info exists a]} {puts yes} else {puts no}\n    \
                  if {[info exists b]} {puts wrong} else {puts right}\n}\np 1\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            last_existence(&unit, "::p", "a"),
            SCALAR,
            "{dialect}: a parameter binds at entry"
        );
        assert_eq!(
            last_existence(&unit, "::p", "b"),
            UNBOUND,
            "{dialect}: a never-assigned local is unbound at entry"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts no") && !rewritten.contains("puts wrong"),
            "{dialect}: both guards decide inside the fixed point:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "yes\nright\n");
}

/// `unset -nocomplain` never raises, whether the place was bound or
/// always absent — its completion domain has no error, unlike a plain
/// `unset` of an absent name (the release table's own `unset -nocomplain
/// nosuch` line, read here through the whole program rather than the
/// existence probe alone): a second, redundant `-nocomplain` and one over
/// a name never set both leave silently.
#[test]
fn unset_nocomplain_never_raises_bound_or_not() {
    let source = "proc p {} {\n    set x 1\n    unset -nocomplain x\n    \
                  unset -nocomplain x\n    unset -nocomplain never\n    \
                  puts done\n    puts [info exists x]\n}\np\n";
    for dialect in RELEASE_DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            last_existence(&unit, "::p", "x"),
            UNBOUND,
            "{dialect}: a repeated -nocomplain leaves the place unbound"
        );
    }
    prints_under_every_release(source, "done\n0\n");
}

/// A scope-alias local enters `MayBound`, never provably `Unbound`,
/// however little the visible source writes the linked global: the alias
/// tracks a cell other, unanalysed code may already have set, so the
/// guard on it never decides and both arms of `[info exists g]` survive
/// the optimiser — even though this program's own `::g` is never set
/// anywhere, so tclsh always takes the "no" arm.
#[test]
fn a_scope_alias_enters_maybound() {
    let source =
        "proc p {} {\n    global g\n    if {[info exists g]} {puts yes} else {puts no}\n}\np\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        assert_eq!(
            last_existence(&unit, "::p", "g"),
            Some(Existence::MayBound),
            "{dialect}: a scope alias is MayBound, not provably absent"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts yes") && rewritten.contains("puts no"),
            "{dialect}: a MayBound guard never folds away either arm:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "no\n");
}

/// A cross-event iRules variable enters `MayBound` (a superset of
/// `ConnectionScope::cross_event_defs`): `y` is bound in `CLIENT_ACCEPTED`
/// and read at `HTTP_REQUEST`'s own entry, so the guard there never
/// decides either — no oracle here, since `when` is not a command a plain
/// `tclsh` runs.
#[test]
fn a_cross_event_variable_enters_maybound() {
    let source = "when CLIENT_ACCEPTED {\n    set y 1\n}\n\
                  when HTTP_REQUEST {\n    if {[info exists y]} {puts yes} else {puts no}\n}\n";
    let unit = unit_of(source, "f5-irules");
    assert_eq!(
        last_existence(&unit, "::when::HTTP_REQUEST", "y"),
        Some(Existence::MayBound),
        "a cross-event variable is MayBound at another handler's entry"
    );
    let (rewritten, rewrites) = optimised(source, "f5-irules");
    assert!(
        rewritten.contains("puts yes") && rewritten.contains("puts no"),
        "a MayBound guard never folds away either arm:\n{rewritten}\n{rewrites:#?}"
    );
}

/// O130 folds a chain starting at an absent cell (the O104 / O130 row):
/// the release rule creates `l` in every release, so `lappend l a;
/// lappend l b` folds through the value at the last write exactly as a
/// `set`-anchored chain does. The single O130 rewrite is read here
/// (`rewrites_of`, matching `chain_fold.rs`'s own unit-test shape) since
/// the multipass optimiser goes on to propagate `l`'s now-known value
/// into `return $l` and drop the fold as unused in its turn — a further,
/// sound reduction the exit line's own program does not name.
#[test]
fn o130_folds_a_chain_from_an_absent_cell() {
    let source = "proc p {} {\n    lappend l a\n    lappend l b\n    return $l\n}\nputs [p]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let rewrites = rewrites_of(source, dialect);
        let fold = rewrites
            .iter()
            .find(|o| o.code == DiagCode::O130 && o.replacement.starts_with("set"))
            .unwrap_or_else(|| panic!("{dialect}: expected an O130 fold: {rewrites:#?}"));
        assert_eq!(fold.replacement, "set l {a b}", "{dialect}");
    }
    prints_under_every_release(source, "a b\n");
}

/// A failing dead write is retained (O108's totality proof, the Rewrites
/// row's "failing dead write retained"): `lappend l c` over the malformed
/// list `"a {b"` raises `unmatched open brace in list` in every release,
/// so deleting it — `l` is never read afterwards — would turn a raising
/// program into a silent one.
#[test]
fn a_failing_dead_lappend_is_retained() {
    let source = "proc p {} {\n    set l \"a {b\"\n    lappend l c\n}\np\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        assert!(!removes_store(source, dialect, "lappend l c"), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        assert_eq!(
            run_script(&tclsh, source).map(|(ok, _)| ok),
            Some(false),
            "tclsh{series}: a malformed list raises on `lappend`"
        );
    }
}

/// A failing dead `incr` is retained under 8.4 (O108's totality proof,
/// permission 3): `incr n` on the never-bound, never-read `n` raises
/// `can't read "n": no such variable` under 8.4, so removing it would turn
/// a raising program into a silent one there; a profile whose every
/// release creates the cell (8.5 onwards) still removes it.
#[test]
fn a_failing_dead_incr_is_retained_under_84() {
    let source = "proc p {} {\n    incr n\n    return\n}\np\n";
    for dialect in ["tcl8.4", "tcl", "f5-irules"] {
        assert!(!removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        let ok = run_script(&tclsh, source).map(|(ok, _)| ok);
        if series == "8.4" {
            assert_eq!(
                ok,
                Some(false),
                "tclsh8.4: `incr n` raises on an absent place"
            );
        } else {
            assert_eq!(ok, Some(true), "tclsh{series}: `incr n` creates the cell");
        }
    }
}

/// Whether the analyser reports `code` anywhere in `source` under
/// `dialect`.
fn reports(source: &str, dialect: &str, code: DiagCode) -> bool {
    tcl_compiler::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

/// An externally mutable place is never refined: a call the
/// module cannot see — here a computed head —
/// sets or unsets a global between the guard and the inner query, with no
/// barrier in between, so the inner `info exists` decides nothing. tclsh
/// 8.4 to 9.1 print `yes yes gone`; a refinement would fold the inner
/// conditions to `no no still`, with three false I230s.
#[test]
fn an_unseen_call_ends_no_refinement_because_none_is_made() {
    let source = "\
proc init {} { set ::x 1 }
proc cleanup {} { unset ::x }
set handlers {init cleanup}
proc p {} {
    if {![info exists ::x]} {
        [lindex $::handlers 0]
        if {[info exists ::x]} { puts yes } else { puts no }
    }
}
proc q {} {
    global x
    if {![info exists x]} {
        [lindex $::handlers 0]
        if {[info exists x]} { puts yes } else { puts no }
    }
}
proc r {} {
    set ::x 1
    if {[info exists ::x]} {
        [lindex $::handlers 1]
        if {[info exists ::x]} { puts still } else { puts gone }
    }
}
p; unset ::x; q; r
";
    for dialect in DIALECTS {
        assert!(
            !reports(source, dialect, DiagCode::I230),
            "{dialect}: no inner query is decided"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts yes")
                && rewritten.contains("puts no")
                && rewritten.contains("puts still")
                && rewritten.contains("puts gone"),
            "{dialect}: no arm is folded away:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "yes\nyes\ngone\n");
}

/// The absent-start chain anchor reads the fact at the statement: after a
/// non-lowered `switch` whose arm may bind
/// `l`, the per-version fact of `l`'s version 0 is still `Unbound`, but
/// the fact at `lappend l a` is `MayBound` — the arm's clobber — so no
/// chain anchors there. tclsh 8.4 to 9.1 print `z a b` and `a b`, before
/// and after the optimiser.
#[test]
fn an_absent_start_anchor_reads_the_fact_at_the_statement() {
    let source = "proc p {c} { switch -glob -- $c { a* { set l z } }; lappend l a; lappend l b; puts $l }\np abc\np q\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
        let rewrites = rewrites_of(source, dialect);
        assert!(
            !rewrites.iter().any(|o| o.code == DiagCode::O130),
            "{dialect}: {rewrites:#?}"
        );
    }
    prints_under_every_release(source, "z a b\na b\n");
}

/// A failing dead `incr` on a may-bound place is retained under 8.4:
/// `incr n` after `if {$c} {set n 1}` raises `can't
/// read "n"` under 8.4 when `c` is false, so removing it would silence a
/// raising program there; a profile whose every release creates the cell
/// still removes it. tclsh 8.4 prints `1` (the call raised) and 8.5 to 9.1
/// print `0`.
#[test]
fn a_failing_dead_incr_on_a_maybound_place_is_retained_under_84() {
    let source =
        "proc p {c} {\n    if {$c} {set n 1}\n    incr n\n    return\n}\nputs [catch {p 0} msg]\n";
    for dialect in ["tcl8.4", "tcl", "f5-irules"] {
        assert!(!removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        assert!(removes_store(source, dialect, "incr n"), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        let expected = if series == "8.4" { "1\n" } else { "0\n" };
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

/// A script body nested in a substitution clobbers what it may unset
/// (#2231's consequence): `[catch {unset x}]` in a
/// condition leaves `x` may-bound, so the later `[info exists x]` decides
/// nothing — a fold would give `1` with an I230. tclsh 8.4 to 9.1 print
/// `no`, before and after the optimiser.
#[test]
fn a_substituted_body_clobbers_what_it_unsets() {
    let source = "set x 1\nif {[catch {unset x}]} { puts err }\nif {[info exists x]} {puts yes} else {puts no}\n";
    for dialect in DIALECTS {
        assert!(!reports(source, dialect, DiagCode::I230), "{dialect}");
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            rewritten.contains("puts no"),
            "{dialect}: the else arm stays:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "no\n");
}

/// A whole-variable `switch` subject resolves from the lattice (§ `switch`
/// of the interface page), so program (4)'s flattened form decides per arm: I231
/// on the dead arm's pattern and O107 on its body, beside O112 on the whole
/// statement, which subsumes O107's rewrite when the findings are applied
/// together — so O107 is read from the passes' raw findings and the
/// applied program keeps neither the arm nor the `switch`, printing
/// `always` under 8.4 to 9.1. A `${…}` subject whose name carries a
/// backslash stays `Raw` and decides nothing — no I231 and no O107 — and
/// tclsh prints `hit` before and after the optimiser.
#[test]
fn the_flattened_form_yields_o107() {
    let source = "set acc \"\"; append acc foo; append acc bar\nswitch -- $acc {\n    baz     { puts never }\n    default { puts always }\n}\n";
    let pattern = u32::try_from(source.find("baz").expect("the dead arm")).expect("an offset");
    for dialect in DIALECTS {
        let diagnostics = tcl_compiler::analyser::Analyser::new()
            .analyse(source, dialect)
            .diagnostics;
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code == DiagCode::I231 && d.span.start() == pattern),
            "{dialect}: I231 on the dead arm's pattern: {diagnostics:#?}"
        );
        let registry = static_context_for(dialect).commands();
        let raw = optimise_raw(source, registry, Some(dialect));
        for code in [DiagCode::O107, DiagCode::O112] {
            assert!(
                raw.iter().any(|o| o.code == code),
                "{dialect}: {code:?} is a finding:\n{raw:#?}"
            );
        }
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts never") && !rewritten.contains("switch"),
            "{dialect}: the applied program keeps neither the arm nor the switch:\n{rewritten}\n{rewrites:#?}"
        );
    }
    prints_under_every_release(source, "always\n");

    let negative = "set {a\\b} baz\nswitch -- ${a\\b} {\n    baz     { puts hit }\n    default { puts miss }\n}\n";
    for dialect in DIALECTS {
        assert!(!reports(negative, dialect, DiagCode::I231), "{dialect}");
        let rewrites = rewrites_of(negative, dialect);
        assert!(
            !rewrites.iter().any(|o| o.code == DiagCode::O107),
            "{dialect}: {rewrites:#?}"
        );
    }
    prints_under_every_release(negative, "hit\n");
}

/// Program (4) — `switch` over the string two appends build — yields O112
/// and I231 on the dead arm in every form. The exact form decides through its
/// dispatch chain, which also drops the arm's body (O107); the `-glob`,
/// `-regexp`, `-nocase` and fall-through forms decide through the selection
/// record the solver makes at the statement, where the subject has the version
/// the statement reads, and report the arm as a selection fact: no block is
/// dropped, O107 does not fire and no O100 hints at a branch. `-nocase` is
/// 8.5's, so a profile that may be 8.4 leaves it alone. The optimised program
/// prints `always` under tclsh 8.4 to 9.1 (`-nocase` from 8.5).
#[test]
fn program_four_yields_o112_and_i231_for_every_form() {
    let build = "set acc \"\"; append acc foo; append acc bar\n";
    let arms = "{\n    baz     { puts never }\n    default { puts always }\n}\n";
    // `baz -` shares `qux`'s body; the `-` is spelled bare, the one spelling
    // every release reads alike.
    let shared = "{\n    baz     -\n    qux     { puts never }\n    default { puts always }\n}\n";
    let forms = [
        (format!("{build}switch -- $acc {arms}"), "8.4", true),
        (format!("{build}switch -glob -- $acc {arms}"), "8.4", false),
        (
            format!("{build}switch -regexp -- $acc {arms}"),
            "8.4",
            false,
        ),
        (
            format!("{build}switch -nocase -- $acc {arms}"),
            "8.5",
            false,
        ),
        (format!("{build}switch -- $acc {shared}"), "8.4", false),
    ];
    for (source, first, flattened) in forms {
        let dead = |word: &str| u32::try_from(source.find(word).expect("an arm")).expect("offset");
        let mut dead_arms = vec![dead("baz")];
        if source.contains("qux") {
            dead_arms.push(dead("qux"));
        }
        for dialect in DIALECTS {
            let has_the_form = first == "8.4" || !matches!(dialect, "tcl8.4" | "f5-irules" | "tcl");
            let diagnostics = tcl_compiler::analyser::Analyser::new()
                .analyse(&source, dialect)
                .diagnostics;
            let registry = static_context_for(dialect).commands();
            let raw = optimise_raw(&source, registry, Some(dialect));
            if !has_the_form {
                assert!(
                    !raw.iter().any(|o| o.code == DiagCode::O112)
                        && !diagnostics.iter().any(|d| d.code == DiagCode::I231),
                    "{dialect}: no selection is made\n{source}"
                );
                continue;
            }
            for &arm in &dead_arms {
                assert!(
                    diagnostics
                        .iter()
                        .any(|d| d.code == DiagCode::I231 && d.span.start() == arm),
                    "{dialect}: I231 on the dead arm at {arm}\n{source}\n{diagnostics:#?}"
                );
            }
            let (rewritten, rewrites) = optimised(&source, dialect);
            assert!(
                rewrites.iter().any(|o| o.code == DiagCode::O112)
                    && !rewritten.contains("puts never")
                    && !rewritten.contains("switch"),
                "{dialect}: O112 leaves `puts always`\n{source}\n{rewritten}\n{rewrites:#?}"
            );
            assert_eq!(
                raw.iter().any(|o| o.code == DiagCode::O107),
                flattened,
                "{dialect}: O107 on the dead body only where the CFG has the arm's block\n{source}\n{raw:#?}"
            );
            if !flattened {
                let unit = unit_of(&source, dialect);
                let top = &unit.top_level;
                assert_eq!(
                    top.sccp.executable_blocks.len(),
                    top.cfg.blocks.len(),
                    "{dialect}: no block is dropped\n{source}"
                );
                let profile = resolve_environment(dialect).analyser_profile();
                let checks =
                    tcl_compiler::compiler_checks::run_all_checks(&unit, registry, Some(profile));
                assert!(
                    checks.iter().all(|check| check.code != DiagCode::O100),
                    "{dialect}: a selection fact hints at no branch\n{source}\n{checks:#?}"
                );
            }
        }
        prints_under_releases_from(&source, "always\n", first);
    }
}

/// A selection fact names an arm no branch leads to, so it folds no
/// condition: the statement's block also ends in the `if`'s branch, and a
/// fact keyed by that block had rewritten `if {$x}` to `if {0}`. The procedure
/// is called with both a true and a false argument, so `x` is no constant.
/// tclsh 8.4 to 9.1 print `A`, `X` and `A` for the program, before and after
/// the optimiser.
#[test]
fn a_selection_fact_folds_no_condition_beside_it() {
    let source = "proc p {x} {\n    set s abc\n    switch -glob -- $s {a* {puts A} b* {puts B}}\n    if {$x} {puts X}\n}\np 1\np 0\n";
    for dialect in DIALECTS {
        let registry = static_context_for(dialect).commands();
        let raw = optimise_raw(source, registry, Some(dialect));
        assert!(
            !raw.iter().any(|o| o.code == DiagCode::O101),
            "{dialect}: no condition folds\n{raw:#?}"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("if {$x}") && !rewritten.contains("switch"),
            "{dialect}: the switch folds, the condition stays\n{rewritten}"
        );
    }
    prints_under_every_release(source, "A\nX\nA\n");
}

/// A loop whose header the solver decides is reported from that fact: W240
/// where the header is false at entry, W241 where it is true at every test
/// and nothing leaves the loop, and either replaces W242's hint that a
/// counter is never modified. Each W240 program prints only `done` under
/// tclsh 8.4 to 9.1, before and after the optimiser: the body never runs.
/// The infinite loops are not run. A header nothing decides keeps W242, and
/// one with an exit the flow graph or the body's text finds draws none.
#[test]
fn a_decided_loop_header_gives_w240_or_w241() {
    let codes = |source: &str, dialect: &str| -> Vec<String> {
        let mut found: Vec<String> = tcl_compiler::analyser::Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .filter(|code| matches!(code.as_str(), "W240" | "W241" | "W242"))
            .collect();
        found.sort();
        found
    };
    let never = [
        "set n 0\nwhile {$n} {puts never}\nputs done\n",
        "proc p {} {set n 0; while {$n} {puts never}; return done}\nputs [p]\n",
        "for {set i 0} {$i < 0} {incr i} {puts never}\nputs done\n",
        "set n 0\nwhile 0 {puts never}\nputs done\n",
    ];
    for source in never {
        for dialect in DIALECTS {
            assert_eq!(codes(source, dialect), ["W240"], "{dialect}\n{source}");
        }
        prints_under_every_release(source, "done\n");
    }
    let infinite = [
        "set go 1\nwhile {$go} {puts x}\n",
        "proc p {} {set go 1; while {$go} {puts x}}\n",
        "for {set i 0} {$i < 10} {} {puts hi}\n",
        "while 1 {puts x}\n",
    ];
    for source in infinite {
        for dialect in DIALECTS {
            assert_eq!(codes(source, dialect), ["W241"], "{dialect}\n{source}");
        }
    }
    let undecided = "proc p {n} {\n    while {$n} {puts x}\n}\n";
    let left =
        "proc p {} {\n    set go 1\n    while {$go} {if {[gets stdin] eq \"q\"} {break}}\n}\n";
    for dialect in DIALECTS {
        assert_eq!(codes(undecided, dialect), ["W242"], "{dialect}");
        assert!(codes(left, dialect).is_empty(), "{dialect}");
    }
}

/// `case` lowers as an opaque glob selection over its own contract: `a*` and
/// a literal pattern each select the first arm, in the one-word form and in
/// the separate-words form with `in` alike — the selection record says so
/// under 8.4, 8.6 and the iRules profile — no I231 claims the live arm is
/// dead, and the optimiser leaves `puts yes`, which tclsh 8.4 to 8.6 print
/// before and after it. From 9.0 there is no `case`: nothing lowers it, so
/// nothing is recorded or folded.
#[test]
fn case_selects_its_glob_arm() {
    for clauses in [
        "in a* {puts yes} default {puts no}",
        "in abc {puts yes} default {puts no}",
        "{a* {puts yes} default {puts no}}",
    ] {
        let source = format!("proc p {{}} {{\n    case abc {clauses}\n}}\np\n");
        for dialect in ["tcl8.4", "tcl8.6", "f5-irules"] {
            let unit = unit_of(&source, dialect);
            let records = &unit
                .procedures
                .get("::p")
                .expect("the procedure")
                .sccp
                .selections;
            assert_eq!(records.len(), 1, "{dialect}: {source}");
            assert_eq!(records[0].fact.selected, [Some(0)], "{dialect}: {source}");
            assert!(
                !reports(&source, dialect, DiagCode::I231),
                "{dialect}: {source}"
            );
            let (rewritten, rewrites) = optimised(&source, dialect);
            assert!(
                rewritten.contains("puts yes") && !rewritten.contains("puts no"),
                "{dialect}: {source}\n{rewritten}\n{rewrites:#?}"
            );
        }
        let unit = unit_of(&source, "tcl9.0");
        let function = unit.procedures.get("::p").expect("the procedure");
        assert!(function.sccp.selections.is_empty(), "tcl9.0: {source}");
        assert!(
            !function
                .ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .any(|statement| matches!(statement.statement, Statement::Switch { .. })),
            "tcl9.0: {source}"
        );
        for (series, tclsh) in releases_on_path() {
            if !series.starts_with("8.") {
                continue;
            }
            let (rewritten, _) = optimised(&source, &dialect_of(series));
            for program in [source.as_str(), rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some((true, "yes\n".to_owned())),
                    "tclsh{series}:\n{program}"
                );
            }
        }
    }
}

/// A write an arm of a `switch` the flow graph keeps as one statement makes
/// is never folded away: tclsh 8.4 to 9.1 print `b` and `0` for each of these
/// programs, before and after the optimiser, where taking the earlier `go` as
/// the value after the switch printed `a` and `1`. `-nocase` is from 8.5.
#[test]
fn a_write_an_opaque_switch_arm_makes_is_never_folded_away() {
    for (arm, first) in [
        ("-glob -- $s { q* { set go 0 } }", "8.4"),
        ("-nocase -- $s { Q1 { set go 0 } }", "8.5"),
        ("-regexp -- $s { {^q} { set go 0 } }", "8.4"),
        ("-glob -- $s { x - q* { set go 0 } }", "8.4"),
        (
            "-glob -- $s { z* { set other 1 } default { set go 0 } }",
            "8.4",
        ),
    ] {
        let source = format!(
            "set go 1\nset s [string tolower Q1]\nswitch {arm}\nif {{$go}} {{puts a}} else {{puts b}}\nputs $go\n"
        );
        prints_under_releases_from(&source, "b\n0\n", first);
        let in_proc = format!(
            "proc p {{}} {{\n set go 1\n set s [string tolower Q1]\n switch {arm}\n if {{$go}} {{puts a}} else {{puts b}}\n puts $go\n}}\np\n"
        );
        prints_under_releases_from(&in_proc, "b\n0\n", first);
    }
}

/// What a command an arm of such a `switch` runs does to the frame is never
/// folded away either: a callee that writes the caller's `go` through `upvar`,
/// `namespace eval` at the global level and `dict with` (8.5 on) each leave
/// `b` and `0` at the top level, where the earlier `go` printed `a` and `1`;
/// in a procedure `namespace eval ::` writes the global, and the local keeps
/// its value before and after the optimiser.
#[test]
fn a_write_a_command_an_opaque_switch_arm_runs_is_never_folded_away() {
    for (arm, first, in_proc) in [
        ("zero go", "8.4", "b\n0\n"),
        ("namespace eval :: {set go 0}", "8.4", "a\n1\n"),
        ("set d {}; dict with d {set go 0}", "8.5", "b\n0\n"),
    ] {
        let source = format!(
            "proc zero {{v}} {{upvar 1 $v x; set x 0}}\nset go 1\nset s [string tolower Q1]\n\
             switch -glob -- $s {{ q* {{ {arm} }} }}\nif {{$go}} {{puts a}} else {{puts b}}\nputs $go\n"
        );
        prints_under_releases_from(&source, "b\n0\n", first);
        let in_proc_source = format!(
            "proc zero {{v}} {{upvar 1 $v x; set x 0}}\nproc p {{}} {{\n set go 1\n set s [string tolower Q1]\n \
             switch -glob -- $s {{ q* {{ {arm} }} }}\n if {{$go}} {{puts a}} else {{puts b}}\n puts $go\n}}\np\n"
        );
        prints_under_releases_from(&in_proc_source, in_proc, first);
    }
}

/// A write a callback script makes — an `after` handler, a variable trace's
/// callback, a procedure named as a callback — is never folded away either:
/// each program prints the callback's value, `1` and `b`, under every
/// release, before and after the optimiser.
#[test]
fn a_write_a_callback_script_makes_is_never_folded_away() {
    for source in [
        "set done 0\nafter 10 { set done 1 }\nafter 50\nupdate\nputs $done\n",
        "proc tick {} { set ::done 1 }\nset done 0\nafter 10 tick\nafter 50\nupdate\nputs $done\n",
        "set done 0\nafter 10 { set ::done 1 }\nafter 50\nupdate\nif {$done} {puts 1} else {puts 0}\n",
        "set done 0\nafter 10 \"set ::done 1\"\nafter 50\nupdate\nputs $done\n",
        "set done 0\nafter 10 [list set ::done 1]\nafter 50\nupdate\nputs $done\n",
        "proc tick {n} { set ::done $n }\nset done 0\nafter 10 [list tick 1]\nafter 50\nupdate\nputs $done\n",
    ] {
        prints_under_every_release(source, "1\n");
    }
    prints_under_every_release(
        "set go 1\ntrace add variable x write { set ::go 0 ;# }\nset x 1\nif {$go} {puts a} else {puts b}\n",
        "b\n",
    );
}

/// A command the module cannot see may write a plain top-level name as it
/// writes `::g`, so the name is never folded across the call: `foo` here is
/// defined at run time, from a file the program writes and sources, and sets
/// the global — tclsh 8.4 to 9.1 print `six` and `6`, where taking `5` across
/// the call printed `other` and `5`.
#[test]
fn a_write_a_command_the_module_cannot_see_makes_is_never_folded_away() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {} {set ::g 6}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    for tail in [
        "set g 5\nfoo\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set ::g 5\nfoo\nif {$::g == 6} {puts six} else {puts other}\nputs $::g\n",
        "set g 5\nwhile {$g != 6} { foo }\nputs six\nputs $g\n",
    ] {
        prints_under_every_release(&format!("{define}{tail}"), "six\n6\n");
    }
}

/// A sourced file runs in the frame of the call, so it writes a procedure's
/// local as well as a global: the file here sets `g` to 6, and tclsh 8.4 to 9.1
/// print `six` and `6` at the top level and in a procedure, where taking `5`
/// across the `source` printed `other` and `5`.
#[test]
fn a_write_a_sourced_file_makes_is_never_folded_away() {
    let write_file = "set f [file join [file dirname [info script]] vt-sourced-[pid].tcl]\n\
                      set fh [open $f w]\nputs $fh {set g 6}\nclose $fh\n";
    let top = format!(
        "{write_file}set g 5\nsource $f\nfile delete $f\nif {{$g == 6}} {{puts six}} else {{puts other}}\nputs $g\n"
    );
    prints_under_every_release(&top, "six\n6\n");
    let in_proc = format!(
        "proc p {{f}} {{\n set g 5\n source $f\n if {{$g == 6}} {{puts six}} else {{puts other}}\n puts $g\n}}\n{write_file}p $f\nfile delete $f\n"
    );
    prints_under_every_release(&in_proc, "six\n6\n");
}

/// A command the module cannot see may write a procedure's local as well as a
/// global: a callee an autoloader or the unresolved-command handler brings in
/// runs `upvar 1` into the frame that called it. `missing` here is defined by
/// `auto_index` when Tcl's `unknown` first looks for it, and sets the
/// caller's `g` to 6 — tclsh 8.4 to 9.1 print `six` and `6` from each
/// procedure, where taking `5` across the call printed `other` and `5`, and the
/// loop the call ends never stopped.
#[test]
fn a_write_an_autoloaded_command_makes_to_a_procedure_local_is_never_folded_away() {
    let define = "set auto_index(missing) {proc missing {} {upvar 1 g g; set g 6}}\n";
    for body in [
        " set g 5\n missing\n if {$g == 6} {puts six} else {puts other}\n puts $g\n",
        " set g 5\n set r [missing]\n if {$g == 6} {puts six} else {puts other}\n puts $g\n",
        " set g 5\n while {$g != 6} { missing }\n puts six\n puts $g\n",
    ] {
        prints_under_every_release(
            &format!("{define}proc p {{}} {{\n{body}}}\np\n"),
            "six\n6\n",
        );
    }
}

/// Each program selects its `hit` arm of a `switch` whose subject and pattern
/// are the same characters spelled two ways: a bare or quoted word is its
/// escapes decoded, a braced word its content, and an element of a braced arm
/// list either. The flattened dispatch compared the subject's spelling — `a\nb`
/// is four characters there — to the decoded pattern, so the analyser called
/// the `hit` arm unreachable (I231) and the optimiser rewrote the program to
/// its `miss` default, where tclsh 8.4 to 9.1 print `hit`.
const SAME_CHARACTERS: [&str; 18] = [
    r#"switch -- a\nb {"a\nb" {puts hit} default {puts miss}}"#,
    r#"switch -exact -- "a\tb" {a\tb {puts hit} default {puts miss}}"#,
    r#"switch a\nb {"a\nb" {puts hit} default {puts miss}}"#,
    r#"switch "a\nb" {a\nb {puts hit} default {puts miss}}"#,
    r#"switch "a\tb" {a\tb {puts hit} default {puts miss}}"#,
    r#"switch a\tb {"a\tb" {puts hit} default {puts miss}}"#,
    r#"switch "a\\b" {{a\b} {puts hit} default {puts miss}}"#,
    r#"switch {a\b} {"a\\b" {puts hit} default {puts miss}}"#,
    "switch \"a\\nb\" {{a\nb} {puts hit} default {puts miss}}",
    "switch {a\nb} {\"a\\nb\" {puts hit} default {puts miss}}",
    "switch {a\\\nb} {{a b} {puts hit} default {puts miss}}",
    r#"switch "a\tb" a\tb {puts hit} default {puts miss}"#,
    r#"switch a\tb "a\tb" {puts hit} default {puts miss}"#,
    r"switch a\$b {a\$b {puts hit} default {puts miss}}",
    r"switch a\[b {a\[b {puts hit} default {puts miss}}",
    "set s \"a\\nb\"\nswitch $s {a\\nb {puts hit} default {puts miss}}",
    r#"switch -glob -- a\nb {"a\nb" {puts hit} default {puts miss}}"#,
    r"switch -glob -- a\$b {a\$b {puts hit} default {puts miss}}",
];

/// A `switch` compares the values of its words however they are spelled: every
/// program of [`SAME_CHARACTERS`] prints `hit` under every release, before and
/// after the optimiser, which keeps the matching arm and nothing else, and no
/// O107 rewrite removes the arm that runs.
#[test]
fn a_switch_compares_the_values_of_its_words_however_they_are_spelled() {
    for source in SAME_CHARACTERS {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.trim().ends_with("puts hit")
                    && !rewritten.contains("switch")
                    && !rewritten.contains("miss"),
                "{dialect}: {source}\n{rewritten}"
            );
            for dead in rewrites_of(source, dialect)
                .iter()
                .filter(|rewrite| rewrite.code == DiagCode::O107)
            {
                let removed = &source[dead.span.start() as usize..dead.span.end() as usize];
                assert!(
                    !removed.contains("hit"),
                    "{dialect}: O107 removes the arm that runs: {source}\n{removed}"
                );
            }
        }
        prints_under_every_release(source, "hit\n");
    }
}

/// Before 8.5 `switch` reads every leading word that starts with `-` as an
/// option, however many words follow, so a subject holding `-glob` is one:
/// tclsh 8.4 rejects the program with `bad option`, where 8.5 to 9.1 select
/// the `-glob` arm and print `G`. A release that may be 8.4 — `tcl8.4`, a
/// profile that names none — keeps the statement as it is, and from 8.5 the
/// optimiser keeps the arm. `--` ends the run, so the subject is one under
/// every release and the statement folds under all of them; a subject that
/// does not start with `-` is selected through the statement's own record
/// where the flattened chain is not built; and a literal subject is read by
/// its decoded value, so `\x2dglob` is an option as `-glob` is.
#[test]
fn a_subject_a_release_may_read_as_an_option_is_not_folded_before_8_5() {
    let bare = "set x -glob\nswitch $x {-glob {puts G} default {puts D}}\n";
    let escaped = "switch \\x2dglob {-glob {puts G} default {puts D}}\n";
    for source in [bare, escaped] {
        for (series, tclsh) in releases_on_path() {
            let before_85 = series == "8.4";
            let expected = if before_85 {
                (false, String::new())
            } else {
                (true, "G\n".to_owned())
            };
            let (rewritten, _) = optimised(source, &dialect_of(series));
            for program in [source, rewritten.as_str()] {
                assert_eq!(
                    run_script(&tclsh, program),
                    Some(expected.clone()),
                    "tclsh{series}:\n{program}"
                );
            }
            assert_eq!(
                rewritten.contains("switch"),
                before_85,
                "{series}: {rewritten}"
            );
        }
        let (kept, _) = optimised(source, "tk");
        assert!(kept.contains("switch"), "a profile with no release: {kept}");
    }
    let ended = "set x -glob\nswitch -- $x {-glob {puts G} default {puts D}}\n";
    for dialect in ["tcl8.4", "tcl8.6", "tk"] {
        let (rewritten, _) = optimised(ended, dialect);
        assert!(
            rewritten.contains("puts G") && !rewritten.contains("switch"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(ended, "G\n");
    let plain = "set x a\nswitch $x {a {puts A} default {puts D}}\n";
    for dialect in ["tcl8.4", "tk", "tcl9.0"] {
        let (rewritten, _) = optimised(plain, dialect);
        assert!(
            rewritten.contains("puts A") && !rewritten.contains("switch"),
            "{dialect}: {rewritten}"
        );
    }
    prints_under_every_release(plain, "A\n");
}

/// With pattern and body words the subject is inside the option scan on every
/// release — 8.5 to 9.1 stop the scan with two words left, and a pattern and
/// its body are two words — so a variable holding `-glob` is an option there
/// too: every tclsh rejects the program below with `extra switch pattern with
/// no body`, where the default arm would print `D`. The statement is kept
/// under every release's profile and under one that names none.
#[test]
fn a_subject_inside_the_scan_of_the_arms_as_words_is_not_folded_on_any_release() {
    let source = "set x -glob\nswitch $x a {puts A} default {puts D}\n";
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, String::new())),
                "tclsh{series}:\n{program}"
            );
        }
        assert!(rewritten.contains("switch"), "{series}: {rewritten}");
    }
    let (kept, _) = optimised(source, "tk");
    assert!(kept.contains("switch"), "a profile with no release: {kept}");
}

/// A read inside a braced `expr` is a use of the version it reads, and a read
/// beside a write the same statement's substitutions make is read before,
/// between and after it: the store feeding the word that runs first stays,
/// and no word is forwarded the earlier value. Each program prints what
/// tclsh 8.4 to 9.1 print, before and after the optimiser — where the
/// statement was a `puts`, a `set` of an expression or of a string, an `expr`,
/// a `return`, an `incr`, a condition, or a call to a procedure, the optimiser
/// had deleted `set x 1` and the program raised `can't read "x"`, or forwarded
/// `1` past the increment and printed `1 2 1`.
#[test]
fn a_braced_expr_read_keeps_its_store() {
    // The two programs the examples page gives.
    prints_under_every_release(
        "proc p {} {\n set n 1\n set r [expr {$n + [incr n]}]\n return $r\n}\nputs [p]\n",
        "3\n",
    );
    prints_under_every_release(
        "proc q {} {\n set n 1\n set r [expr {[incr n] + [incr n]}]\n return $r\n}\nputs [q]\n",
        "5\n",
    );
    for (body, expected) in [
        ("puts [expr {$x + [set x 10] + $x}]\n puts $x", "21\n10\n"),
        (
            "set r [expr {$x + [set x 10] + $x}]\n puts $r\n puts $x",
            "21\n10\n",
        ),
        ("expr {$x + [set x 10] + $x}\n puts $x", "10\n"),
        ("if {$x + [set x 10] > 3} {puts yes}\n puts $x", "yes\n10\n"),
        ("while {$x + [set x 10] < 3} {break}\n puts $x", "10\n"),
        (
            "set y 5\n incr y [expr {$x + [set x 10]}]\n puts \"$y $x\"",
            "16 10\n",
        ),
        (
            "set r \"$x [set x 10] $x\"\n puts \"$r|$x\"",
            "1 10 10|10\n",
        ),
        ("foo $x [incr x] $x\n puts $x", "1 2 2\n2\n"),
        (
            "set l {}\n lappend l $x [incr x] $x\n puts \"$l|$x\"",
            "1 2 2|2\n",
        ),
        (
            "set r [expr {[string length $x] + [set x 10]}]\n puts \"$r $x\"",
            "11 10\n",
        ),
        (
            "set r [expr {[expr {$x + 1}] + [set x 10]}]\n puts \"$r $x\"",
            "12 10\n",
        ),
    ] {
        let source = format!(
            "proc foo {{a b c}} {{puts \"$a $b $c\"}}\nproc p {{}} {{\n set x 1\n {body}\n}}\np\n"
        );
        prints_under_every_release(&source, expected);
    }
    prints_under_every_release(
        "proc p {} {\n set x 1\n return [expr {$x + [set x 10]}]\n}\nputs [p]\n",
        "11\n",
    );
    // At the top level the same statements print the same.
    prints_under_every_release(
        "set x 1\nputs [expr {$x + [set x 10] + $x}]\nputs $x\n",
        "21\n10\n",
    );
}

/// A write a statement makes that none of its words read overwrites the store
/// before it, as it did: the call that carries the write reads the place only
/// where a word does. `puts [set x 2]` and a `gets` that fills a loop's
/// variable leave the stores before them dead, so O109 and O126 still remove
/// them, and each program prints the same afterwards.
#[test]
fn a_store_a_nested_write_overwrites_unread_is_still_dead() {
    let overwritten = "proc p {} {\n set x 1\n puts [set x 2]\n return $x\n}\nputs [p]\n";
    let loop_variable = "proc p {fd} {\n set line {}\n set n 0\n \
                         while {[gets $fd line] >= 0} {incr n}\n return $n\n}\n\
                         set f [open /dev/null r]\nputs [p $f]\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        assert!(removes_store(overwritten, dialect, "set x 1"), "{dialect}");
        assert!(
            removes_store(loop_variable, dialect, "set line {}"),
            "{dialect}"
        );
    }
    prints_under_every_release(overwritten, "2\n2\n");
    prints_under_every_release(loop_variable, "0\n");
}

/// The programs of [`a_nested_write_in_a_host_prints_what_tclsh_prints`], each with what
/// tclsh 8.4 to 9.1 print.
const NESTED_WRITE_PROGRAMS: &[(&str, &str)] = &[
    // a loop carries the write round
    (
        "set x 0\nfor {set i 0} {$i < 3} {incr i} { set r [expr {[incr x] * 2}] }\nputs \"$r $x\"\n",
        "6 3\n",
    ),
    (
        "set x 0\nset i 0\nwhile {$i < 4} { expr {[incr x 2] + 0}; incr i }\nputs $x\n",
        "8\n",
    ),
    (
        "proc p {n} {set x 0; for {set i 0} {$i < $n} {incr i} {set r [expr {$x + [incr x]}]}; return \"$r $x\"}\nputs [p 3]\nputs [p 1]\n",
        "5 3\n1 1\n",
    ),
    // a branch the engine takes or skips
    (
        "proc p {c} { set x 1; if {$c} { set r [expr {[incr x] + 1}] } else { set r 0 }; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "3 2\n0 1\n",
    ),
    (
        "proc p {c} { set x 1; set r [expr {$c ? [incr x] : 0}]; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "2 2\n0 1\n",
    ),
    (
        "proc p {c} { set x 1; set r [expr {$c && [incr x]}]; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "1 2\n0 1\n",
    ),
    (
        "proc p {c} { set x 1; set r [expr {$c || [incr x]}]; return \"$r $x\" }\nputs [p 1]\nputs [p 0]\n",
        "1 1\n1 2\n",
    ),
    (
        "set x 1\nset r [expr {1 ? [incr x] : [incr x 10]}]\nputs \"$r $x\"\n",
        "2 2\n",
    ),
    (
        "set x 1\nset r [expr {0 || [incr x]}]\nputs \"$r $x\"\n",
        "1 2\n",
    ),
    (
        "set x 1\nset r [expr {1 || [incr x]}]\nputs \"$r $x\"\n",
        "1 1\n",
    ),
    // an input the engine takes on one member and not on another
    (
        "proc p {n} {set x 1; if {$n} {set c 0} else {set c 1}; set r [expr {$c ? [incr x] : 7}]; return \"$r $x\"}\nputs [p 0]\nputs [p 1]\n",
        "2 2\n7 1\n",
    ),
    // an element of an array
    (
        "set a(1) 5\nset r [expr {$a(1) + [incr a(1)]}]\nputs \"$r $a(1)\"\n",
        "11 6\n",
    ),
    (
        "set a(k) 1\nset r [expr {[incr a(k)] + [incr a(k)]}]\nputs \"$r $a(k)\"\n",
        "5 3\n",
    ),
    (
        "array set a {1 5 2 6}\nset r [expr {$a(1) + [set a(2) 9] + $a(2)}]\nputs \"$r $a(1) $a(2)\"\n",
        "23 5 9\n",
    ),
    // a place the frame does not own is left alone
    (
        "set ::g 5\nset r [expr {$::g + [incr ::g]}]\nputs \"$r $::g\"\n",
        "11 6\n",
    ),
    (
        "proc p {} {global g; set g 1; set r [expr {$g + [incr g]}]; return \"$r $g\"}\nputs [p]\n",
        "3 2\n",
    ),
    (
        "proc p {} {upvar 1 v w; set w 1; set r [expr {$w + [incr w]}]; return \"$r $w\"}\nset v 0\nputs [p]\nputs $v\n",
        "3 2\n2\n",
    ),
    // a write a word's own command makes
    (
        "set s a\nset r [expr {[string length [append s bc]] + [string length $s]}]\nputs \"$r $s\"\n",
        "6 abc\n",
    ),
    (
        "set s a\nset r [expr {[string length $s] + [string length [append s bc]]}]\nputs \"$r $s\"\n",
        "4 abc\n",
    ),
    (
        "set l {}\nset r [expr {[llength [lappend l a]] + [llength $l]}]\nputs \"$r $l\"\n",
        "2 a\n",
    ),
    (
        "set x 5\nset r [expr {$x * [set x 2] + $x}]\nputs \"$r $x\"\n",
        "12 2\n",
    ),
    (
        "set s a\nset r [expr {[string length [append s [append s b]]]}]\nputs \"$r $s\"\n",
        "4 abab\n",
    ),
    (
        "set x 1\nset r [expr {$x} + [incr x]]\nputs \"$r $x\"\n",
        "4 2\n",
    ),
    (
        "unset -nocomplain u\nset r [expr {[string length [append u [set u 3]]]}]\nputs \"$r $u\"\n",
        "2 33\n",
    ),
    // substitutions inside substitutions
    (
        "set x 1\nset r [expr {[expr {$x + [incr x]}] + [incr x]}]\nputs \"$r $x\"\n",
        "6 3\n",
    ),
    (
        "set x 1\nset r [expr {[set y [incr x]] + $y + $x}]\nputs \"$r $x $y\"\n",
        "6 2 2\n",
    ),
    (
        "set x 1\nset r [expr {[incr x] * [incr x] * [incr x]}]\nputs \"$r $x\"\n",
        "24 4\n",
    ),
    // the statements a substitution sits in
    ("set x 1\nexpr {[incr x] + [incr x]}\nputs $x\n", "3\n"),
    (
        "set x 1\nif {[incr x] == 2} { puts yes } else { puts no }\nputs $x\n",
        "yes\n2\n",
    ),
    ("set x 1\nwhile {[incr x] < 5} { }\nputs $x\n", "5\n"),
    (
        "proc p {} {set x 1; return [expr {[incr x] + $x}]}\nputs [p]\n",
        "4\n",
    ),
    (
        "proc p {} {set x 1; set y [expr {[incr x] + $x}]; return \"$y $x\"}\nputs [p]\n",
        "4 2\n",
    ),
    ("set x 1\nputs [expr {$x + [incr x]}]\nputs $x\n", "3\n2\n"),
    (
        "set x 1\nset y [list [expr {[incr x] + 1}] $x]\nputs $y\n",
        "3 2\n",
    ),
    ("set x 1\nputs \"[expr {[incr x] + 1}] $x\"\n", "3 2\n"),
    (
        "set x 1\nset r [expr \"$x + [incr x] + $x\"]\nputs \"$r $x\"\n",
        "5 2\n",
    ),
    (
        "set x 1\nset r [expr {\"$x [incr x]\" eq {1 2}}]\nputs \"$r $x\"\n",
        "1 2\n",
    ),
    // a quoted or unbraced operand
    (
        "set x 1\nset r [expr $x + [incr x]]\nputs \"$r $x\"\n",
        "3 2\n",
    ),
    (
        "set x 1\nset r [expr $x \"+\" [incr x]]\nputs \"$r $x\"\n",
        "3 2\n",
    ),
    (
        "set x 1\nset r [expr {[incr x] + [info exists x]}]\nputs \"$r $x\"\n",
        "3 2\n",
    ),
    (
        "unset -nocomplain x\nset r [expr {[info exists x] + [set x 3]}]\nputs \"$r $x\"\n",
        "3 3\n",
    ),
    (
        "unset -nocomplain x\nset r [expr {[set x 3] + [info exists x]}]\nputs \"$r $x\"\n",
        "4 3\n",
    ),
    // existence
    (
        "set x 1\ncatch {set r [expr {[incr x] + [error mid]}]}\nputs $x\n",
        "2\n",
    ),
    (
        "set x 1\ncatch {set r [expr {1 / 0 + [incr x]}]}\nputs $x\n",
        "1\n",
    ),
    (
        "set x 1\ncatch {set r [expr {[incr x] + 1 / 0}]}\nputs $x\n",
        "2\n",
    ),
    // a value that is not an integer
    (
        "set x 1.5\nset r [expr {$x + [set x 2.5] + $x}]\nputs \"$r $x\"\n",
        "6.5 2.5\n",
    ),
];

/// A write a substitution inside an expression makes is a definition of the
/// statement that holds the expression, made before the statement takes the
/// result: each program prints what tclsh 8.4 to 9.1 print, before and after
/// the optimiser forwards the values the shared lattice now knows. They cover
/// a loop that carries the write round, a branch or a short-circuit that
/// skips it, an element of an array, a place the frame does not own, a write a
/// word's own command makes, a substitution inside a substitution, each kind
/// of statement that holds an expression, an operand that is quoted or
/// unbraced, a place the write creates, and a value that is not an integer.
#[test]
fn a_nested_write_in_a_host_prints_what_tclsh_prints() {
    for &(source, expected) in NESTED_WRITE_PROGRAMS {
        prints_under_every_release(source, expected);
    }
}

/// A command the module cannot see — here `foo`, defined at run time from a
/// file the program writes and sources — writes a plain top-level name as it
/// writes `::g` when it runs inside the body of a `catch` the flow graph
/// keeps as one statement, or through a computed head: tclsh 8.4 to 9.1 print
/// `six` and `6` for each program, where taking `5` across the call printed
/// `other` and `5` and a rewrite followed.
#[test]
fn a_write_a_catch_body_or_a_computed_head_runs_is_never_folded_away() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {} {set ::g 6}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    for tail in [
        "set g 5\ncatch {foo}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set g 5\ncatch {foo} msg\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set g 5\ncatch {if {1} {foo}}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set ::g 5\ncatch {foo}\nif {$::g == 6} {puts six} else {puts other}\nputs $::g\n",
        "set g 5\nif {[catch {foo}]} {puts bad}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set cmd foo\nset g 5\n$cmd\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set cmd foo\nset g 5\nwhile {[$cmd] != 7} {break}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
    ] {
        prints_under_every_release(&format!("{define}{tail}"), "six\n6\n");
    }
    for tail in [
        "set g 5\ncatch {puts [foo]}\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
        "set cmd foo\nset g 5\nputs [$cmd]\nif {$g == 6} {puts six} else {puts other}\nputs $g\n",
    ] {
        prints_under_every_release(&format!("{define}{tail}"), "6\nsix\n6\n");
    }
    // A command in the body that may write any name.
    for program in [
        "set go 1\ncatch { namespace eval :: {set go 0} }\nif {$go} {puts a} else {puts b}\n",
        "set go 1\nset script {set go 0}\ncatch { eval $script }\nif {$go} {puts a} else {puts b}\n",
    ] {
        prints_under_every_release(program, "b\n");
    }
}

/// A command the module cannot see — `foo` again, defined at run time — runs
/// inside a body that is no body of the frame the substitution is written in:
/// a lambda `apply` runs, a `namespace eval` or `uplevel` body, the text a
/// `subst` substitutes, an expression word inside a body. It writes `::g`
/// there as it does anywhere, so tclsh 8.4 to 9.1 print `six` and `6` for each
/// program, where the value before the call was taken across it and the
/// condition decided. A lambda body that writes a name of its own, and a
/// `subst` of text that runs nothing unseen, leave the condition decided.
#[test]
fn a_substitution_runs_the_unseen_code_in_every_body_it_holds() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {} {set ::g 6}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    let check = "if {$g == 6} {puts six} else {puts other}\nputs $g\n";
    for form in [
        "set x [namespace eval ns {foo}]",
        "set x [uplevel #0 {foo}]",
        "set x [subst {[foo]}]",
        "set x [catch {expr {[foo] + 1}}]",
        "set x [catch {if {[foo]} {set y 1}}]",
    ] {
        prints_under_every_release(&format!("{define}set g 5\n{form}\n{check}"), "six\n6\n");
    }
    for form in [
        "set x [apply {{} {foo}}]",
        "set x [catch {apply {{} {foo}}}]",
        "set x [apply {{} {if {[foo]} {set y 1}}}]",
        "set x [apply {{n} {foo}} 1]",
    ] {
        prints_under_releases_from(
            &format!("{define}set g 5\n{form}\n{check}"),
            "six\n6\n",
            "8.5",
        );
    }
    // A body with nothing unseen in it leaves the condition decided: the
    // optimiser folds it, and the program prints the same.
    let decided = "set g 5\nset x [namespace eval ns {set y 1}]\n\
                   if {$g == 5} {puts five} else {puts other}\nputs $g\n";
    let lambda = "set g 5\nset x [apply {{} {set y 1}}]\n\
                  if {$g == 5} {puts five} else {puts other}\nputs $g\n";
    let text = "set g 5\nset x [subst {[set y 1]}]\n\
                if {$g == 5} {puts five} else {puts other}\nputs $g\n";
    for (source, first) in [(decided, "8.4"), (lambda, "8.5"), (text, "8.4")] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(!optimised(source, dialect).0.contains("other"), "{source}");
        }
        prints_under_releases_from(source, "five\n5\n", first);
    }
}

/// A name the body of a `catch` writes on one path only keeps the value it
/// held before, so the store before the `catch` is read: tclsh prints `5` for
/// each program, where taking the body's write as the one every path makes
/// deleted `set g 5` and the read raised `can't read "g"`.
#[test]
fn a_store_a_catch_body_may_leave_untouched_is_not_dead() {
    let top = "set g 5\ncatch { if {[expr {[clock seconds] < 0}]} { set g 0 } }\nputs $g\n";
    let in_proc = "proc p {} {\n set g 5\n catch { if {[expr {[clock seconds] < 0}]} { set g 0 } }\n puts $g\n}\np\n";
    let with_result =
        "set g 5\ncatch { if {[expr {[clock seconds] < 0}]} { set g 0 } } msg\nputs $g\n";
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        for source in [top, in_proc, with_result] {
            assert!(
                !removes_store(source, dialect, "set g 5"),
                "{dialect}: {source}"
            );
        }
    }
    for source in [top, in_proc, with_result] {
        prints_under_every_release(source, "5\n");
    }
}

/// A program, what tclsh prints for it, and the one condition it holds, if any:
/// its truth, and whether the analysis decides it.
type BodyProgram = (&'static str, &'static str, Option<(bool, bool)>);

/// Programs whose `catch` stands in a `[…]` substitution, with what tclsh
/// 8.4 to 9.1 print and the one condition each holds, if any: its truth, and
/// whether the analysis decides it. They are the three of #2231 (the second is
/// the statement form), one in a procedure, a result variable beside the
/// substitution, a `catch` in a `catch`, one reached through an alias of
/// `catch` (whose head the module rebinds, so nothing is decided), and a body
/// that sets a name, unsets one, substitutes a command, or writes through an
/// expression word.
const CATCH_BODY_PROGRAMS: &[BodyProgram] = &[
    (
        "set x 1\nset c [catch {incr x}]\nif {$x == 2} {puts two} else {puts \"not two: $x\"}\n",
        "two\n",
        Some((true, true)),
    ),
    ("set x 5\ncatch {incr x} m\nputs \"$x $m\"\n", "6 6\n", None),
    (
        "set x 1\nset c [catch {append x y}]\nputs $x\n",
        "1y\n",
        None,
    ),
    (
        "proc p {} {\n set x 1\n set c [catch {incr x}]\n if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        Some((true, true)),
    ),
    (
        "set x 5\nputs [catch {incr x} m]\nputs \"$x $m\"\n",
        "0\n6 6\n",
        None,
    ),
    (
        "set x 1\nset c [catch {catch {incr x}}]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        Some((true, true)),
    ),
    (
        "set g 5\nset rc [catch {set g 0}]\nif {$g} {puts a} else {puts b}\n",
        "b\n",
        Some((false, true)),
    ),
    (
        "set x 1\nputs [catch {unset x}]\nputs [info exists x]\n",
        "0\n0\n",
        None,
    ),
    (
        "set x 1\nputs [catch {expr {[incr x] + [error mid]}}]\nputs $x\n",
        "1\n2\n",
        None,
    ),
    (
        "set x 5\nset c [catch {puts $x}]\nputs $c\n",
        "5\n0\n",
        None,
    ),
    ("set x 5\nset c [catch {incr x}]\nputs $c\n", "0\n", None),
    (
        "set x 1\nset c [catch {set y [incr x]}]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        Some((true, true)),
    ),
    (
        "interp alias {} c {} catch\nset x 1\nset r [c {incr x}]\n\
         if {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        Some((true, false)),
    ),
];

/// The truths the diagnostics claim for the conditions of `source` under
/// `dialect`: `true` for an `if` that is always true, `false` for one that
/// is always false.
fn condition_claims(source: &str, dialect: &str) -> Vec<bool> {
    tcl_compiler::analyser::Analyser::new()
        .analyse(source, dialect)
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::I230)
        .filter_map(|diagnostic| {
            if diagnostic.message.contains("is always true") {
                Some(true)
            } else if diagnostic.message.contains("is always false") {
                Some(false)
            } else {
                None
            }
        })
        .collect()
}

/// The body of a `catch`, or of a `try`, inside a `[…]` substitution runs once
/// in the frame the substitution is written in, whatever it completes with,
/// so what it writes and what it reads are the statement's own effect: the
/// writes are definitions after the statement and the reads are uses of the
/// definitions before it (#2231). Each program prints what it did before the
/// multipass optimiser rewrote it — where the lattice held the value before
/// the body, the stores the body reads were deleted — and the condition it
/// holds is claimed by no diagnostic but with the truth tclsh gives it; a
/// closed body under the builtin head decides it. A `try` body does the same
/// from 8.6.
#[test]
fn a_nested_catch_body_is_the_statements_effect() {
    for &(source, expected, condition) in CATCH_BODY_PROGRAMS {
        prints_under_every_release(source, expected);
        let truth = condition.map(|(truth, _)| truth);
        for dialect in DIALECTS {
            assert!(
                condition_claims(source, dialect)
                    .iter()
                    .all(|claimed| Some(*claimed) == truth),
                "{dialect}: a condition is decided as tclsh does not:\n{source}"
            );
            for store in ["set x 1", "set x 5"] {
                if source.contains(store) {
                    assert!(
                        !removes_store(source, dialect, store),
                        "{dialect}: {store} is read:\n{source}"
                    );
                }
            }
        }
        for dialect in ["tcl8.6", "tcl9.0"] {
            let decided = condition
                .filter(|(_, decided)| *decided)
                .map(|(truth, _)| truth);
            assert_eq!(
                condition_claims(source, dialect),
                decided.into_iter().collect::<Vec<_>>(),
                "{dialect}: the condition is decided exactly where the body is closed:\n{source}"
            );
        }
    }
    let in_try = "set x 1\nset r [try {incr x} on error {} {set y 0}]\n\
                  if {$x == 2} {puts two} else {puts other}\n";
    prints_under_releases_from(in_try, "two\n", "8.6");
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            condition_claims(in_try, dialect)
                .iter()
                .all(|claimed| *claimed),
            "{dialect}"
        );
    }
}

/// The scripts of the `catch` code table that read nothing the program has
/// not set, with what the result variable holds after `catch {script} m`
/// under tclsh 8.4 to 9.1.
const CATCH_RESULTS: &[(&str, &str)] = &[
    ("error boom", "boom"),
    ("return 5", "5"),
    ("break", ""),
    ("continue", ""),
    ("set v 1", "1"),
    ("expr {1/0}", "divide by zero"),
    ("set y 5; error $y", "5"),
    ("set y 5; set z $y; error $z", "5"),
    ("error a; error b", "a"),
    ("", ""),
    ("catch {error z}", "1"),
    ("set x 1; incr x", "2"),
    ("string length abc", "3"),
];

/// A closed `catch` — brace-quoted, every command with a route of its own and
/// every value exact — is evaluated to the code and the result its script
/// completes with, so the result variable holds what the script returned: the
/// last command's result, the message of the error it stopped at (the first
/// completion that is not a normal one ends the script, whatever follows), or
/// what `return` carried. The lattice holds it, the optimiser forwards it, and
/// tclsh 8.4 to 9.1 print the same before and after.
///
/// What the program does not prove stays unknown — an error whose message
/// reads a variable never set — and the variable is still bound.
#[test]
fn a_closed_catch_script_gives_its_result_variable_what_it_returned() {
    for &(script, expected) in CATCH_RESULTS {
        let source = format!("catch {{{script}}} m\nputs \"<$m>\"\n");
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                top_value_at(&unit, "m", 1).and_then(lattice_text),
                Some(expected.to_owned()),
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(&format!("puts \"<{expected}>\"")),
                "{dialect}:\n{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("<{expected}>\n"));
    }

    let unknown = "catch {error $nothing} m\nputs \"<$m>\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            top_value_at(&unit_of(unknown, dialect), "m", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        let unbound_reads: Vec<String> = tcl_compiler::analyser::Analyser::new()
            .analyse(unknown, dialect)
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W210)
            .map(|diagnostic| diagnostic.message.clone())
            .collect();
        assert!(
            unbound_reads.iter().all(|message| !message.contains("'m'")),
            "{dialect}: the result variable is bound: {unbound_reads:?}"
        );
    }
    prints_under_every_release(unknown, "<can't read \"nothing\": no such variable>\n");

    // A script that is not brace-quoted text is not known: `"error $e"` runs
    // `error a b`, whose message is `a`, where its spelling would give `a b`.
    let quoted = "set e {a b}\ncatch \"error $e\" m\nputs \"<$m>\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            top_value_at(&unit_of(quoted, dialect), "m", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
    prints_under_every_release(quoted, "<a>\n");
}

/// A word of a command inside a `catch` script that raises is the script's
/// error, which the `catch` absorbs: the statement itself does not raise, so
/// what follows it in a `try` body runs. tclsh 8.6 to 9.1 print `1`; taking
/// the word's error for the statement's ended the block and folded `after` to
/// the handler's `0`.
#[test]
fn an_error_a_catch_script_raises_is_not_the_statements_own() {
    // The script's `return` keeps the `catch` one opaque call, and without it
    // the script is lowered into the procedure's blocks.
    for script in ["set a [error boom]; return x", "set a [error boom]"] {
        let source = format!(
            "proc p {{}} {{\n    try {{\n        catch {{{script}}} m\n        \
             set after 1\n    }} on error {{}} {{\n        set after 0\n    }}\n    \
             return $after\n}}\nputs [p]\n"
        );
        prints_under_releases_from(&source, "1\n", "8.6");
        let unit = unit_of(&source, "tcl8.6");
        let function = unit.procedures.get("::p").expect("the procedure");
        let executable: Vec<&str> = function
            .sccp
            .executable_blocks
            .iter()
            .filter_map(|id| function.cfg.blocks.get(id))
            .map(|block| block.name.as_str())
            .collect();
        assert!(
            executable.iter().any(|name| name.starts_with("try_ok")),
            "the body completes normally: {executable:?}"
        );
    }
}

/// A result the program does not prove is still bound, and the options
/// dictionary beside it is a dictionary whose text the analysis never has:
/// `catch {error $x} m o` evaluates to the code 1, binds `m` as a string and
/// `o` as a dictionary, each with a value that is not available, and draws
/// no W210 where they are read.
#[test]
fn an_unproven_result_and_the_options_dictionary_are_bound_and_typed() {
    use tcl_compiler::value_transfer::FoldedType;
    use tcl_registry::TclType;
    use tcl_registry::value_transfer::RepresentationEvidence;
    let source = "proc p {x} {\n    catch {error $x} m o\n    return [list $m $o]\n}\n";
    let stated = |ty| {
        Some(FoldedType {
            intrep: Some(ty),
            shape: None,
            representation: RepresentationEvidence::Unknown,
        })
    };
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        for var in ["m", "o"] {
            assert_eq!(
                value_at(&unit, "::p", var, 1),
                Some(LatticeValue::Overdefined),
                "{dialect}: {var}"
            );
        }
        assert_eq!(folded_at(&unit, "::p", "m", 1), stated(TclType::String));
        assert_eq!(folded_at(&unit, "::p", "o", 1), stated(TclType::Dict));
        assert_eq!(answers_for(&unit, "::p", "catch"), ["evaluated"]);
        assert!(!reports(source, dialect, DiagCode::W210), "{dialect}");
    }
}

/// The options dictionary of a `catch` starts `-code N -level L` for every
/// completion but an error (`error msg info code` lists `-errorinfo` first),
/// so that is all the route states of its text: tclsh 8.5 to 9.1 give each
/// row below that prefix. It is never an exact value, because a success is
/// not always `-code 0 -level 0` alone: from 8.6 `incr` and `lappend` of an
/// absent variable leave a stale `-errorcode {TCL READ VARNAME}` in the
/// options of a normal completion, and `-errorinfo`, `-errorline` and
/// `-errorstack` are the interpreter's own text.
#[test]
fn the_options_dictionary_starts_with_the_code_and_level_the_route_states() {
    let rows = [
        ("set v 1", "-code 0 -level 0"),
        ("return 5", "-code 0 -level 1"),
        ("break", "-code 3 -level 0"),
        ("continue", "-code 4 -level 0"),
        ("return -code 5 custom", "-code 5 -level 1"),
        ("incr absent", "-code 0 -level 0"),
        ("lappend absent x", "-code 0 -level 0"),
    ];
    let stale = "catch {incr absent} m o\nputs $o\n";
    let mut ran = 0;
    for (series, tclsh) in releases_on_path() {
        if series < "8.5" {
            continue;
        }
        ran += 1;
        for (script, prefix) in rows {
            let program = format!(
                "catch {{{script}}} m o\nputs [expr {{[string first {{{prefix}}} $o] == 0}}]\n"
            );
            assert_eq!(
                run_script(&tclsh, &program),
                Some((true, "1\n".to_owned())),
                "tclsh{series}: {program}"
            );
        }
        let expected = if series >= "8.6" {
            "-code 0 -level 0 -errorcode {TCL READ VARNAME}\n"
        } else {
            "-code 0 -level 0\n"
        };
        assert_eq!(
            run_script(&tclsh, stale),
            Some((true, expected.to_owned())),
            "tclsh{series}"
        );
    }
    assert!(ran > 0 || releases_on_path().is_empty());
    let source = "proc p {} {\n    catch {incr absent} m o\n    return $o\n}\n";
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0"] {
        assert_eq!(
            value_at(&unit_of(source, dialect), "::p", "o", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
}

/// Programs whose `catch`, or `try`, runs a script that is not brace-quoted
/// text, in a substitution, with what tclsh prints and the first release that
/// has the command: a quoted word that substitutes nothing, a script held in a
/// variable, a quoted word that substitutes, one with an escape, one that
/// writes what the script reads, a `catch` in a `catch`, a condition, and a
/// `try` body.
const UNBRACED_SCRIPT_PROGRAMS: &[(&str, &str, &str)] = &[
    (
        "set x 1\nset c [catch \"incr x\"]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        "8.4",
    ),
    (
        "set x 1\nset c [catch \"incr x; set y 2\" m]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch \"incr x\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "set x 1\nset s {incr x}\nset c [catch $s]\nif {$x == 2} {puts two} else {puts other}\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set s {incr x}\n    set c [catch $s]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set v x\n    set c [catch \"incr $v\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch \"incr x\\n\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch \"catch {incr x}\"]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set s {incr x}\n    if {[catch $s]} {puts bad}\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    if {[catch \"incr x\"]} {puts bad}\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a 1\n    set b 1\n    set c [catch \"incr a; incr b\"]\n    \
         return \"$a$b\"\n}\nputs [p]\n",
        "22\n",
        "8.4",
    ),
    (
        "set x 5\nset s {puts $x}\nset c [catch $s]\nputs $c\n",
        "5\n0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set r [try \"incr x\" on error {} {}]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x 1\n    set s {incr x}\n    set r [try $s on error {} {}]\n    \
         if {$x == 2} {return two} else {return other}\n}\nputs [p]\n",
        "two\n",
        "8.6",
    ),
];

/// A script a substitution runs in this frame, once, whatever it completes
/// with, is the statement's effect wherever its text is known: a quoted word
/// that substitutes nothing is read as the brace-quoted one is, and a script
/// that is run-time data — held in a variable, substituted, escaped, expanded
/// — may write any name, so nothing after the statement is decided on a value
/// from before it, as after the statement form's barrier. Each program prints
/// what tclsh prints, before and after the optimiser; no diagnostic decides
/// its condition as tclsh does not, and the store a script reads stays.
#[test]
fn a_quoted_or_computed_script_is_the_statements_effect() {
    for &(source, expected, first) in UNBRACED_SCRIPT_PROGRAMS {
        prints_under_releases_from(source, expected, first);
        let dialects: &[&str] = if first == "8.4" {
            &DIALECTS
        } else {
            &["tcl8.6", "tcl9.0"]
        };
        for &dialect in dialects {
            assert!(
                condition_claims(source, dialect)
                    .iter()
                    .all(|claimed| *claimed),
                "{dialect}: a condition is decided as tclsh does not:\n{source}"
            );
        }
    }
    let reads = "set x 5\nset s {puts $x}\nset c [catch $s]\nputs $c\n";
    for dialect in DIALECTS {
        assert!(
            !removes_store(reads, dialect, "set x 5"),
            "{dialect}: the script reads x"
        );
    }
}

/// The rows of the `catch` code table that read nothing the program has not
/// set: the script, the code `catch` returns for it and what its result
/// variable holds, the same under tclsh 8.4 to 9.1. `return -code 5 custom`
/// completes with the code 2 where it stands in a `catch` — the code 5 is the
/// one a procedure's caller sees.
const CATCH_CODES: &[(&str, i64, &str)] = &[
    ("return 5", 2, "5"),
    ("break", 3, ""),
    ("continue", 4, ""),
    ("error boom", 1, "boom"),
    ("set v 1", 0, "1"),
    ("expr {1/0}", 1, "divide by zero"),
    ("return -code 5 custom", 2, "custom"),
    ("string length abc", 0, "3"),
];

/// The program one row of the code table runs: a procedure that keeps the
/// code of a `catch` in a substitution and the result beside it, and prints
/// both.
fn catch_program(script: &str) -> String {
    format!("proc p {{}} {{\n    set c [catch {{{script}}} m]\n    puts \"$c|$m\"\n}}\np\n")
}

/// The rows every release reads alike, under each release's dialect and the
/// `tcl` profile that spans them: the code, the result, the `evaluated`
/// answer and the optimiser's forwarding, and the lines tclsh 8.4 to 9.1
/// print before and after.
fn the_catch_rows_every_release_reads_alike() {
    for &(script, code, result) in CATCH_CODES {
        let source = catch_program(script);
        for dialect in RELEASE_DIALECTS {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                value_at(&unit, "::p", "c", 1),
                Some(LatticeValue::Const(ConstValue::Int(code))),
                "{dialect}: the code of {script}"
            );
            assert_eq!(
                value_at(&unit, "::p", "m", 1).and_then(lattice_text),
                Some(result.to_owned()),
                "{dialect}: the result of {script}"
            );
            assert_eq!(
                answers_for(&unit, "::p", "catch"),
                ["evaluated"],
                "{dialect}: {script}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(&format!("{code}|{result}")),
                "{dialect}: {script} is forwarded:\n{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("{code}|{result}\n"));
    }
}

/// `catch {incr absent}`: the error of 8.4 and of the iRules dialect on its
/// 8.4 base, the created cell from 8.5, and nothing under the spanning
/// profile, which cannot say which release runs.
fn the_catch_row_that_splits_on_an_absent_cell() {
    let absent = catch_program("incr absent");
    let message = "can't read \"absent\": no such variable";
    let created = Some((0, "1"));
    let raised = Some((1, message));
    for (dialect, expected) in [
        ("tcl8.4", raised),
        ("f5-irules", raised),
        ("tcl8.5", created),
        ("tcl8.6", created),
        ("tcl9.0", created),
        ("tcl9.1", created),
        ("tcl", None),
    ] {
        let unit = unit_of(&absent, dialect);
        let code_fact = value_at(&unit, "::p", "c", 1);
        let result_fact = value_at(&unit, "::p", "m", 1).and_then(lattice_text);
        if let Some((code, result)) = expected {
            assert_eq!(
                code_fact,
                Some(LatticeValue::Const(ConstValue::Int(code))),
                "{dialect}: the code"
            );
            assert_eq!(
                result_fact.as_deref(),
                Some(result),
                "{dialect}: the result"
            );
        } else {
            assert_eq!(code_fact, Some(LatticeValue::Overdefined), "{dialect}");
            assert_eq!(result_fact, None, "{dialect}");
        }
    }
    for (series, tclsh) in releases_on_path() {
        let expected = if series == "8.4" {
            format!("1|{message}\n")
        } else {
            "0|1\n".to_owned()
        };
        for text in [absent.clone(), optimised(&absent, &dialect_of(series)).0] {
            assert_eq!(
                run_script(&tclsh, &text),
                Some((true, expected.clone())),
                "tclsh{series}:\n{text}"
            );
        }
    }
}

/// A `catch` in a `[…]` substitution — where the code is the value of the
/// statement and the message or result a variable beside it — is evaluated to
/// the code its script completes with: the code table, forwarded by the
/// optimiser and printed by tclsh 8.4 to 9.1 before and after, with the
/// release split of `incr` on a cell that is absent as the one row that
/// differs. A procedure's own completion is a callee's summary, so `catch
/// {q}` over a `proc q {} {return -code 5 custom}` states nothing of the code
/// 5 tclsh prints.
#[test]
fn the_catch_code_table() {
    the_catch_rows_every_release_reads_alike();
    the_catch_row_that_splits_on_an_absent_cell();
    let callee = "proc q {} {return -code 5 custom}\n\
                  proc p {} {\n    set c [catch {q} m]\n    puts \"$c|$m\"\n}\np\n";
    for dialect in RELEASE_DIALECTS {
        assert_eq!(
            value_at(&unit_of(callee, dialect), "::p", "c", 1),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
    }
    prints_under_every_release(callee, "5|custom\n");
}

/// A script that is not brace-quoted text is not known: `"error $e"` runs
/// `error a b`, whose message is `a`, where its spelling would give `a b`. A
/// `catch` in a substitution has no barrier ahead of it, so the script word is
/// the only thing between the lattice and a message that was never raised:
/// the result and the code stay unknown under every dialect, and tclsh 8.4 to
/// 9.1 print `1|a`.
#[test]
fn a_catch_script_that_is_not_brace_quoted_text_is_not_the_script_it_runs() {
    let quoted = "proc p {} {\n    set e {a b}\n    set c [catch \"error $e\" m]\n    \
                  puts \"$c|$m\"\n}\np\n";
    for dialect in RELEASE_DIALECTS {
        let unit = unit_of(quoted, dialect);
        for var in ["c", "m"] {
            assert_eq!(
                value_at(&unit, "::p", var, 1),
                Some(LatticeValue::Overdefined),
                "{dialect}: {var}"
            );
        }
    }
    prints_under_every_release(quoted, "1|a\n");
}

/// A command substitution that is the whole value of a statement is run in
/// order with its own writes whichever registry-owned route the command
/// declares: the statement's call holds what the command stored and the
/// statement what it returned, so `set a [incr n]` leaves `n` 2 as well as
/// `a`. Each program prints what tclsh 8.5 to 9.1 print before and after the
/// optimiser, and the values the lattice holds are the ones printed.
#[test]
fn a_command_substitution_lands_its_writes_beside_its_result() {
    type Program = (
        &'static str,
        &'static str,
        &'static [(&'static str, &'static str)],
    );
    let programs: [Program; 7] = [
        (
            "proc p {} {set n 1; set a [incr n]; return \"$a $n\"}\nputs [p]\n",
            "2 2\n",
            &[("a", "2"), ("n", "2")],
        ),
        (
            "proc p {} {set a [set b 5]; return \"$a $b\"}\nputs [p]\n",
            "5 5\n",
            &[("a", "5"), ("b", "5")],
        ),
        (
            "proc p {} {set s abc; set a [append s def]; return \"$a|$s\"}\nputs [p]\n",
            "abcdef|abcdef\n",
            &[("a", "abcdef"), ("s", "abcdef")],
        ),
        (
            "proc p {} {set l {}; set a [lappend l x y]; return \"$a|$l\"}\nputs [p]\n",
            "x y|x y\n",
            &[("a", "x y"), ("l", "x y")],
        ),
        (
            "proc p {} {set a [scan {7 8} {%d %d} x y]; return \"$a $x $y\"}\nputs [p]\n",
            "2 7 8\n",
            &[("a", "2"), ("x", "7"), ("y", "8")],
        ),
        (
            "proc p {} {set a [lassign {1 2 3} u v]; return \"$a|$u|$v\"}\nputs [p]\n",
            "3|1|2\n",
            &[("a", "3"), ("u", "1"), ("v", "2")],
        ),
        (
            "proc p {} {set d {}; set a [dict set d k v]; return \"$a|$d\"}\nputs [p]\n",
            "k v|k v\n",
            &[("a", "k v"), ("d", "k v")],
        ),
    ];
    for (source, expected, last) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            for &(var, text) in last {
                assert_eq!(
                    lattice_text(last_value(source, dialect, "::p", var)).as_deref(),
                    Some(text),
                    "{dialect}: {var} after\n{source}"
                );
            }
        }
        prints_under_releases_from(source, expected, "8.5");
    }
}

/// The scripts of a procedure's `catch` that are straight-line statements —
/// which the flow graph lowers into blocks, ending at a statement that defines
/// the result variable — with what the result variable holds after `catch
/// {script} m` under tclsh 8.4 to 9.1.
const FLATTENED_CATCH_RESULTS: &[(&str, &str)] = &[
    ("set v 1", "1"),
    ("expr {1/0}", "divide by zero"),
    ("set y 5; set z $y", "5"),
    ("string length abc", "3"),
    ("set x 1; incr x", "2"),
    ("set v [string length abc]; set w [expr {$v * 2}]", "6"),
];

/// The number of flattened `catch` regions of `proc`.
fn flattened_catches(unit: &CompilationUnit, proc: &str) -> usize {
    unit.procedures
        .get(proc)
        .expect("the procedure")
        .cfg
        .catch_ends
        .len()
}

/// A `catch` the flow graph flattens into the procedure's blocks has no words
/// where its region ends: the statement there only defines the result and
/// options variables. The graph keeps the `catch` as written beside it, and
/// the solver evaluates it as it does the statement the graph leaves whole,
/// so the result variable holds what the script returned — the last
/// command's result, or the message of the error it stopped at — under every
/// dialect. The lattice holds it, the optimiser forwards it, and tclsh 8.4 to
/// 9.1 print the same before and after.
#[test]
fn a_flattened_catch_gives_its_result_variable_what_its_script_returned() {
    for &(script, expected) in FLATTENED_CATCH_RESULTS {
        let source =
            format!("proc p {{}} {{\n    catch {{{script}}} m\n    puts \"<$m>\"\n}}\np\n");
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            assert_eq!(flattened_catches(&unit, "::p"), 1, "{dialect}: {source}");
            assert_eq!(
                value_at(&unit, "::p", "m", 1).and_then(lattice_text),
                Some(expected.to_owned()),
                "{dialect}: {source}"
            );
            assert_eq!(
                answers_for(&unit, "::p", "catch"),
                ["evaluated"],
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(&format!("puts \"<{expected}>\"")),
                "{dialect}:\n{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("<{expected}>\n"));
    }
}

/// The script of a flattened `catch` runs over the state before its body, not
/// the state where its region ends, where its own writes have already been
/// made: over `x` = 1, `catch {incr x} m` leaves `m` 2 and `x` 2, where the
/// state after the body would give 3. A script that reads what it writes
/// finds it as it was, and a result variable the body also writes is stored
/// last. tclsh 8.4 to 9.1 print what the analysis holds, before and after the
/// optimiser.
#[test]
fn a_flattened_catch_runs_its_script_over_the_state_before_it() {
    let int = |n: i64| Some(LatticeValue::Const(ConstValue::Int(n)));
    let programs: [(&str, &str, i64); 4] = [
        (
            "proc p {} {\n    set x 1\n    catch {incr x} m\n    puts \"$m|$x\"\n}\np\n",
            "2|2\n",
            2,
        ),
        (
            "proc p {} {\n    set x 1\n    catch {incr x; incr x} m\n    puts \"$m|$x\"\n}\np\n",
            "3|3\n",
            3,
        ),
        (
            "proc p {} {\n    set x 1\n    catch {set x [expr {$x + 10}]; set x} m\n    \
             puts \"$m|$x\"\n}\np\n",
            "11|11\n",
            11,
        ),
        (
            "proc p {} {\n    catch {set m 5} m\n    puts $m\n}\np\n",
            "5\n",
            5,
        ),
    ];
    for (source, printed, result) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let unit = unit_of(source, dialect);
            assert_eq!(flattened_catches(&unit, "::p"), 1, "{dialect}: {source}");
            assert_eq!(
                Some(last_value(source, dialect, "::p", "m")),
                int(result),
                "{dialect}: m in\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// What a flattened `catch` cannot prove stays unknown, and the variables it
/// defines stay bound: a script that raises from a word it substitutes, a
/// parameter the procedure was called with, and a result variable a write
/// trace watches. Each program prints under tclsh 8.4 to 9.1 what the
/// original prints, before and after the optimiser.
#[test]
fn a_flattened_catch_the_solver_cannot_prove_stays_unknown() {
    let programs = [
        (
            "proc p {} {\n    catch {puts $undefined} m\n    puts \"<$m>\"\n}\np\n",
            "<can't read \"undefined\": no such variable>\n",
            "8.4",
        ),
        (
            "proc p {x} {\n    catch {incr x} m\n    return \"$m|$x\"\n}\nputs [p 5]\nputs [p abc]\n",
            "6|6\nexpected integer but got \"abc\"|abc\n",
            "8.4",
        ),
        (
            "proc tr {args} {puts T}\nproc p {} {\n    trace add variable m write tr\n    \
             catch {set v 1} m\n    puts $m\n}\np\n",
            "T\n1\n",
            "8.5",
        ),
    ];
    for (source, printed, first) in programs {
        let dialects: &[&str] = if first == "8.4" {
            &DIALECTS
        } else {
            &["tcl8.6", "tcl9.0"]
        };
        for &dialect in dialects {
            let unit = unit_of(source, dialect);
            assert_eq!(
                value_at(&unit, "::p", "m", 1),
                Some(LatticeValue::Overdefined),
                "{dialect}: {source}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// `catch` stores the value of its script's last command in the result
/// variable, so a flattened body's last store is observed where the region
/// ends: deleting it as a store nothing reads changes what the result variable
/// holds. Each program prints under tclsh 8.4 to 9.1 what the original prints,
/// before and after the optimiser, with the store still there — where the
/// result is not one the lattice proves, a parameter, a global, a place a
/// `catch` in a branch or an `upvar` alias names — and so does the program
/// that stores to the same name earlier, which the pass leaves alone.
#[test]
fn a_store_the_flattened_catch_returns_is_not_dead() {
    // The statement stays whatever a word of it is folded to, so each program
    // names the store by what leads it.
    let programs = [
        (
            "proc p {c} {\n    catch {set v $c} m\n    puts \"<$m>\"\n}\np 7\n",
            "<7>\n",
            "set v ",
            "8.4",
        ),
        (
            "proc p {c} {\n    catch {set v $c; set w $v} m\n    puts \"<$m>\"\n}\np 7\n",
            "<7>\n",
            "set w ",
            "8.4",
        ),
        (
            "proc p {c} {\n    catch {set v [string length $c]} m o\n    puts \"<$m>\"\n}\np 7\n",
            "<1>\n",
            "set v ",
            "8.5",
        ),
        (
            "set g 0\nproc p {c} {\n    global g\n    catch {set v $c} g\n}\np 7\nputs $g\n",
            "7\n",
            "set v ",
            "8.4",
        ),
        (
            "proc p {c} {\n    if {$c} {catch {set v 1} m} else {set m other}\n    return $m\n}\n\
             puts \"[p 1] [p 0]\"\n",
            "1 other\n",
            "set v ",
            "8.4",
        ),
        (
            "set g 0\nproc p {} {\n    upvar 1 g m\n    catch {set v 1} m\n}\np\nputs $g\n",
            "1\n",
            "set v ",
            "8.4",
        ),
    ];
    for (source, printed, store, first) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !removes_store(source, dialect, store),
                "{dialect}: {store} is the result:\n{source}"
            );
            assert!(
                optimised(source, dialect).0.contains(store),
                "{dialect}: {store} stays:\n{source}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// A call to a command the module cannot see reads its words before its head
/// runs and may read or write any global afterwards: `foo` here is defined at
/// run time, reads `g` and sets `g` and `m`, and tclsh 8.4 to 9.1 print what each
/// program says. The operand keeps the value it read; the store the callee reads
/// stays, whether a later read follows it or a later store overwrites it —
/// deleting it made the callee raise `can't read "g"`; and a read after the
/// call is never taken for the value before it, of the name the call's words
/// read or of one they do not.
#[test]
fn a_call_the_module_cannot_see_reads_its_words_first_and_may_read_and_write_after() {
    let define = "set f [file join [file dirname [info script]] vt-unseen-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc foo {n} {global g; puts \"n=$n g=$g\"; set ::g 6; set ::m 4}}\n\
                  close $fh\nsource $f\nfile delete $f\n";
    for (tail, expected) in [
        ("set g 5\nfoo $g\nputs $g\n", "n=5 g=5\n6\n"),
        ("set g 5\nset cmd foo\n$cmd $g\nputs $g\n", "n=5 g=5\n6\n"),
        (
            "set cmd foo\nset g 5\n$cmd $g\nputs [expr {$g + 0}]\n",
            "n=5 g=5\n6\n",
        ),
        (
            "set g 5\nset m 3\nfoo $g\nputs $g\nputs $m\n",
            "n=5 g=5\n6\n4\n",
        ),
        ("set g 5\nfoo 1\nputs $g\n", "n=1 g=5\n6\n"),
        ("set cmd foo\nset g 5\n$cmd 1\nputs $g\n", "n=1 g=5\n6\n"),
        ("set g 5\nfoo 1\nset g 7\nputs $g\n", "n=1 g=5\n7\n"),
        (
            "set cmd foo\nset g 5\n$cmd 1\nset g 7\nputs $g\n",
            "n=1 g=5\n7\n",
        ),
    ] {
        prints_under_every_release(&format!("{define}{tail}"), expected);
    }
}

/// A callback spelled as several words, which `after` joins as `concat` does,
/// or one the scan cannot read — a computed script, a command the module never
/// defines, an alias, an expansion — writes the variable a loop waits on, or
/// the one a later read folds: tclsh prints `1` for each program, where
/// taking `0` across the `update` printed `0`. A callback that only cancels,
/// or stores a script that writes another name, leaves the value alone.
#[test]
fn a_write_a_callback_the_scan_could_not_read_makes_is_never_folded_away() {
    let define = "set f [file join [file dirname [info script]] vt-callback-[pid].tcl]\n\
                  set fh [open $f w]\nputs $fh {proc finish {} {set ::done 1}}\nclose $fh\n\
                  source $f\nfile delete $f\n";
    // Two rounds, so a callback that stores another still runs.
    let wait = "after 30\nupdate\nafter 30\nupdate\nputs $done\n";
    for head in [
        "after 10 set done 1",
        "after idle set done 1",
        "after 10 incr done",
        "after 10 {set done} 1",
        "set script {set done 1}\nafter 10 $script",
        "after 10 finish",
        "interp alias {} fin {} set done 1\nafter 10 fin",
        "proc build {} {return {set done 1}}\nafter 10 [build]",
        "after 10 {after 10 set done 1}",
    ] {
        prints_under_every_release(&format!("{define}set done 0\n{head}\n{wait}"), "1\n");
    }
    prints_under_releases_from(
        &format!("set done 0\nafter 10 {{*}}[list set done 1]\n{wait}"),
        "1\n",
        "8.5",
    );
    for head in [
        "after 10 {set done 1}\nafter cancel {set done 1}",
        "set id [after 10 {set done 1}]\nafter cancel $id",
        "after 10 {set other 1}",
        "after 10 set other 1",
    ] {
        prints_under_every_release(&format!("set done 0\n{head}\n{wait}"), "0\n");
    }
}

/// An alias's target word and the words after it are the one command a call
/// to the alias runs, so an alias to `string length` stores no callback that
/// writes a variable, however `string` is spelled: the top-level branch on `x`
/// is decided and folded, and tclsh 8.4 to 9.1 print `kept` before and after
/// the optimiser. Read alone, the target word is `string` with no subcommand,
/// a script that may write any variable, and the branch was left undecided.
#[test]
fn an_alias_to_a_command_that_writes_nothing_leaves_its_branch_decided() {
    for target in ["string length", "::string length"] {
        let source = format!(
            "interp alias {{}} safe {{}} {target}\nset x 5\nsafe {{set x 6}}\n\
             if {{$x == 5}} {{puts kept}} else {{puts changed}}\n"
        );
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl"] {
            let unit = unit_of(&source, dialect);
            assert!(
                !unit.top_level.sccp.constant_branches.is_empty(),
                "{dialect}: {source}"
            );
            let (rewritten, rewrites) = optimised(&source, dialect);
            assert!(
                !rewritten.contains("changed"),
                "{dialect}: {source}\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_every_release(&source, "kept\n");
    }
}

/// A braced arm list is a list: a bare or quoted element's escapes collapse
/// under the release's grammar, and before 8.6 a `\x` takes every hex digit
/// that follows and keeps the last two, so `a\x41b` is not `aAb` there. tclsh
/// prints `miss` from 8.4 and 8.5 and `hit` from 8.6 for each program, before
/// and after the optimiser, which folded the arm to `hit` under all of them.
#[test]
fn a_braced_arm_list_decodes_its_elements_under_the_releases_escapes() {
    for (series, tclsh) in releases_on_path() {
        let expected = if series < "8.6" { "miss\n" } else { "hit\n" };
        for source in [
            r#"switch -- aAb {"a\x41b" {puts hit} default {puts miss}}"#,
            r"switch -- aAb {a\x41b {puts hit} default {puts miss}}",
            r#"switch -glob -- aAb {"a\x41b" {puts hit} default {puts miss}}"#,
            r"switch -glob -- aAb {a\x41b {puts hit} default {puts miss}}",
        ] {
            let dialect = dialect_of(series);
            let (rewritten, _) = optimised(source, &dialect);
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

/// The shared lattice's value for `var`'s version `version` at the top level.
fn top_value_at(unit: &CompilationUnit, var: &str, version: u32) -> Option<LatticeValue> {
    let function = &unit.top_level;
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.values.get(&(symbol, version)).cloned()
}

/// The interface page's ordered-state programs, each over `set x 1`: the
/// expression, the value it has and what `x` holds after it. The seventh
/// program stops at an error in the middle of its expression and is read
/// apart.
const ORDERED_STATE: [(&str, &str, &str); 6] = [
    ("{$x + [incr x] + $x}", "5", "2"),
    ("{0 && [incr x]}", "0", "1"),
    ("{$x + [set x 10] + $x}", "21", "10"),
    ("{[incr x] + [incr x]}", "5", "3"),
    ("{$x ? [incr x] : [incr x 10]}", "2", "2"),
    ("\"$x + [incr x]\"", "3", "2"),
];

/// The seven programs of the interface page's ordered-state paragraph reach
/// every consumer: the shared lattice holds the expression's value in `r#1`
/// and the last write the expression made in `x#2`, in a procedure and at the
/// top level and under every dialect; the optimiser forwards both into the
/// reads after it; and each optimised program prints what tclsh 8.4 to 9.1
/// print. Where the expression is a `puts` argument the statement is not
/// evaluated, no nested write is folded and `puts $x` still reads `x`, and the
/// program prints the same. The seventh program, an error between a write and
/// the end of its expression inside a `catch`, leaves `x` at 2: in a procedure
/// the lattice holds 2 in `x#2`, at the top level, where the `catch` is one
/// call, `x` is unknown, and nothing forwards the earlier value to the read.
#[test]
fn the_seven_ordered_state_witnesses() {
    for (expression, value, after) in ORDERED_STATE {
        let int = |text: &str| Some(LatticeValue::Const(ConstValue::Int(text.parse().unwrap())));
        let top = format!("set x 1\nset r [expr {expression}]\nputs $r\nputs $x\n");
        let in_proc = format!(
            "proc p {{}} {{\n    set x 1\n    set r [expr {expression}]\n    puts $r\n    puts $x\n}}\np\n"
        );
        let shown = format!("{value}\n{after}\n");
        for dialect in DIALECTS {
            let unit = unit_of(&in_proc, dialect);
            assert_eq!(
                value_at(&unit, "::p", "r", 1),
                int(value),
                "{dialect}: {expression}"
            );
            assert_eq!(
                value_at(&unit, "::p", "x", 2),
                int(after),
                "{dialect}: {expression}"
            );
            let unit = unit_of(&top, dialect);
            assert_eq!(
                top_value_at(&unit, "r", 1),
                int(value),
                "{dialect}: {expression}"
            );
            assert_eq!(
                top_value_at(&unit, "x", 2),
                int(after),
                "{dialect}: {expression}"
            );
            for source in [&top, &in_proc] {
                let (rewritten, rewrites) = optimised(source, dialect);
                assert!(
                    rewrites.iter().any(|o| o.code == DiagCode::O100)
                        && rewritten.contains(&format!("puts {value}"))
                        && rewritten.contains(&format!("puts {after}"))
                        && !rewritten.contains("puts $"),
                    "{dialect}: {expression}\n{rewritten}"
                );
            }
        }
        prints_under_every_release(&top, &shown);
        prints_under_every_release(&in_proc, &shown);

        let argument = format!("set x 1\nputs [expr {expression}]\nputs $x\n");
        for dialect in DIALECTS.iter().filter(|_| value != "0") {
            let (rewritten, _) = optimised(&argument, dialect);
            assert!(
                rewritten.contains("puts $x") && rewritten.contains("expr"),
                "{dialect}: {expression}\n{rewritten}"
            );
        }
        prints_under_every_release(&argument, &shown);
    }

    let caught = [
        "set x 1\ncatch {expr {[incr x] + [error mid]}} msg\nputs $msg\nputs $x\n",
        "proc p {} {\n    set x 1\n    catch {expr {[incr x] + [error mid]}} msg\n    puts $msg\n    puts $x\n}\np\n",
    ];
    for dialect in DIALECTS {
        // A `catch` at the top level is one call whose body writes are
        // may-definitions, so `x` is unknown after it; in a procedure its
        // body is blocks of its own, the error leaves from the statement
        // that raised it, and the handler is thrown to with the write the
        // expression had made.
        assert_eq!(
            top_value_at(&unit_of(caught[0], dialect), "x", 2),
            Some(LatticeValue::Overdefined),
            "{dialect}"
        );
        assert_eq!(
            value_at(&unit_of(caught[1], dialect), "::p", "x", 2),
            Some(LatticeValue::Const(ConstValue::Int(2))),
            "{dialect}"
        );
        for source in caught {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("set x 1") && rewritten.contains("puts $x"),
                "{dialect}: nothing forwards the earlier x\n{rewritten}"
            );
        }
    }
    for source in caught {
        prints_under_every_release(source, "mid\n2\n");
    }
    // The completion code, 1, is read from the options.
    let coded = "set x 1\ncatch {expr {[incr x] + [error mid]}} msg opts\nputs $msg\n\
                 puts [dict get $opts -code]\nputs $x\n";
    assert!(optimised(coded, "tcl8.6").0.contains("puts $x"));
    prints_under_releases_from(coded, "mid\n1\n2\n", "8.5");
}

/// A nested write to a place the ordered state cannot own — a qualified
/// global, a `global` or an `upvar` alias — declines the expression whatever
/// the write's own answer: the statement is not folded, its nested write stays,
/// and the optimised program prints what tclsh 8.4 to 9.1 print. Where the
/// write also reads a place nothing proves (`incr ::g`) the read is what
/// declines, and a branch the expression never reaches owns nothing, so
/// `0 && [set ::g 10]` is 0.
#[test]
fn a_nested_write_outside_the_state_declines() {
    let programs = [
        (
            "proc p {} {\n set x 1\n set r [expr {$x + [set ::g 10]}]\n puts \"$r $::g\"\n}\np\n",
            "11 10\n",
            "declined: stateful-nested",
        ),
        (
            "proc p {} {\n global g\n set x 1\n set r [expr {$x + [set g 10]}]\n puts \"$r $g\"\n}\np\n",
            "11 10\n",
            "declined: stateful-nested",
        ),
        (
            "proc p {} {\n upvar 1 v w\n set x 1\n set r [expr {$x + [set w 10]}]\n puts \"$r $w\"\n}\nset v 0\np\n",
            "11 10\n",
            "declined: stateful-nested",
        ),
        (
            "set ::g 5\nproc p {} {\n set x 1\n set r [expr {$x + [incr ::g]}]\n puts \"$r $::g\"\n}\np\n",
            "7 6\n",
            "declined: not-exact",
        ),
    ];
    for (source, printed, answer) in programs {
        for dialect in DIALECTS {
            let unit = unit_of(source, dialect);
            assert_eq!(
                answers_for(&unit, "::p", "expr"),
                [answer],
                "{dialect}:\n{source}"
            );
            assert_eq!(
                value_at(&unit, "::p", "r", 1),
                Some(LatticeValue::Overdefined),
                "{dialect}:\n{source}"
            );
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("expr {$x + [") && rewritten.contains("puts \"$r "),
                "{dialect}: the statement is kept\n{rewritten}"
            );
        }
        prints_under_every_release(source, printed);
    }

    let unreached = "proc p {} {\n set r [expr {0 && [set ::g 10]}]\n puts $r\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(
            value_at(&unit_of(unreached, dialect), "::p", "r", 1),
            Some(LatticeValue::Const(ConstValue::Int(0))),
            "{dialect}"
        );
    }
    prints_under_every_release(unreached, "0\n");
}

/// What a prefix-rule program leaves that the analysis proves.
enum PrefixFact {
    /// The place's last version holds this text.
    Text(&'static str, &'static str),
    /// The place's last version holds this number, whichever way a route
    /// writes it.
    Number(&'static str, i64),
    /// The place's last version is unbound.
    Unbound(&'static str),
}

/// One of the interface page's nine programs for the prefix rule
/// (§ *`catch`, `try`, and completion*): the statements before the `catch`,
/// the body it runs — an error after some of its stores — what is printed
/// after it, what every release prints, the first release with the command,
/// and what the analysis proves of a body its blocks hold. A body that is one
/// straight-line block is lowered into blocks in a procedure; the rest, and
/// every body at the top level, stay one call.
struct PrefixProgram {
    name: &'static str,
    before: &'static str,
    body: &'static str,
    after: &'static str,
    printed: &'static str,
    first: &'static str,
    flattened: &'static [PrefixFact],
}

const PREFIX_PROGRAMS: [PrefixProgram; 10] = [
    PrefixProgram {
        name: "lassign stops at the array",
        before: "set a old\narray set b {k keep}",
        body: "lassign {new second} a b",
        after: "puts \"$a $b(k)\"",
        printed: "new keep\n",
        first: "8.5",
        flattened: &[
            PrefixFact::Text("a", "new"),
            PrefixFact::Text("b(k)", "keep"),
        ],
    },
    PrefixProgram {
        name: "lassign leaves the places after the array",
        before: "array set b {k keep}\nset a old\nset c old",
        body: "lassign {x y z} a b c",
        after: "puts \"$a $c\"",
        printed: "x old\n",
        first: "8.5",
        flattened: &[PrefixFact::Text("a", "x"), PrefixFact::Text("c", "old")],
    },
    PrefixProgram {
        name: "foreach binds in order",
        before: "array set b {k keep}\nset a old",
        body: "foreach {a b} {new second} {set inside 1}",
        after: "puts \"$a [info exists inside]\"",
        printed: "new 0\n",
        first: "8.4",
        flattened: &[],
    },
    PrefixProgram {
        name: "scan writes the place before the array",
        before: "array set b {k keep}\nset a old",
        body: "scan {1 2} {%d %d} a b",
        after: "puts $a",
        printed: "1\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("a", 1)],
    },
    PrefixProgram {
        name: "scan goes on past the array",
        before: "array set a {k keep}\nset b old",
        body: "scan {1 2} {%d %d} a b",
        after: "puts \"$a(k) $b\"",
        printed: "keep 2\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("b", 2)],
    },
    PrefixProgram {
        name: "regexp stops at the array",
        before: "array set b {k keep}\nset a old",
        body: "regexp {(x)(y)} xy a b",
        after: "puts $a",
        printed: "xy\n",
        first: "8.4",
        flattened: &[PrefixFact::Text("a", "xy")],
    },
    PrefixProgram {
        name: "unset stops at the absent name",
        before: "set p 1\nset q 2",
        body: "unset p nosuch q",
        after: "puts \"[info exists p] $q\"",
        printed: "0 2\n",
        first: "8.4",
        flattened: &[PrefixFact::Unbound("p"), PrefixFact::Number("q", 2)],
    },
    PrefixProgram {
        name: "an error in a word is before the command",
        before: "set x 1",
        body: "append x 2 [error boom]",
        after: "puts $x",
        printed: "1\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("x", 1)],
    },
    PrefixProgram {
        name: "an error in the value is before the store",
        before: "",
        body: "set r [expr {1 + [error mid]}]",
        after: "puts [info exists r]",
        printed: "0\n",
        first: "8.4",
        flattened: &[PrefixFact::Unbound("r")],
    },
    PrefixProgram {
        name: "an error in an expression keeps its writes",
        before: "set x 1",
        body: "expr {[incr x] + [error mid]}",
        after: "puts $x",
        printed: "2\n",
        first: "8.4",
        flattened: &[PrefixFact::Number("x", 2)],
    },
];

impl PrefixProgram {
    /// The program as a script: the `catch` at the top level is one call.
    fn at_the_top_level(&self) -> String {
        format!(
            "{}\ncatch {{{}}} m\n{}\n",
            self.before, self.body, self.after
        )
    }

    /// The program in a procedure, where a one-block body is lowered.
    fn in_a_procedure(&self) -> String {
        format!(
            "proc p {{}} {{\n{}\ncatch {{{}}} m\n{}\n}}\np\n",
            self.before, self.body, self.after
        )
    }

    /// The program with `try … on error` for its `catch` (8.6 on).
    fn with_a_handler(&self) -> String {
        format!(
            "proc p {{}} {{\n{}\ntry {{{}}} on error {{m}} {{}}\n{}\n}}\np\n",
            self.before, self.body, self.after
        )
    }

    /// The program with a store after its body in the `try` (8.6 on): the
    /// body's statements are a block each, the handler is thrown to from the
    /// state the failing command leaves, and the store after it never runs.
    fn with_a_store_after_in_the_try(&self) -> String {
        format!(
            "proc p {{}} {{\n{}\ntry {{{}; set w 0}} on error {{m}} {{}}\n{}\n}}\np\n",
            self.before, self.body, self.after
        )
    }

    /// What the analysis of `function` claims of `fact`: that it holds, a
    /// different value or binding, or nothing.
    fn claim(fact: &PrefixFact, function: &tcl_compiler::compilation_unit::FunctionUnit) -> Claim {
        let last = |name: &str| {
            let symbol = function.ssa.var_symbol(name).expect("the variable");
            function
                .sccp
                .values
                .iter()
                .filter(|((sym, _), _)| *sym == symbol)
                .max_by_key(|((_, version), _)| *version)
                .map(|(_, value)| value.clone())
        };
        let value = |name: &str, texts: &[String]| match last(name) {
            Some(LatticeValue::Const(ConstValue::String(held))) if texts.contains(&held) => {
                Claim::Holds
            }
            Some(LatticeValue::Const(ConstValue::Int(held)))
                if texts.contains(&held.to_string()) =>
            {
                Claim::Holds
            }
            Some(LatticeValue::Const(_)) => Claim::Contradicts,
            _ => Claim::Silent,
        };
        match fact {
            PrefixFact::Text(name, text) => value(name, &[(*text).to_owned()]),
            PrefixFact::Number(name, number) => value(name, &[number.to_string()]),
            PrefixFact::Unbound(name) => {
                let symbol = function.ssa.var_symbol(name).expect("the variable");
                let held = function
                    .sccp
                    .existence
                    .iter()
                    .filter(|((sym, _), _)| *sym == symbol)
                    .max_by_key(|((_, version), _)| *version)
                    .map(|(_, fact)| *fact);
                match held {
                    Some(Existence::Unbound) => Claim::Holds,
                    Some(Existence::Bound(_)) => Claim::Contradicts,
                    _ => Claim::Silent,
                }
            }
        }
    }
}

/// What an analysis says of a fact tclsh bears out.
#[derive(Debug, PartialEq, Eq)]
enum Claim {
    /// It proves the fact.
    Holds,
    /// It proves something else, which tclsh does not do.
    Contradicts,
    /// It proves nothing of the place.
    Silent,
}

/// The prefix rule holds in the default build — a `catch` at the top level, or
/// one whose body is not a straight line, is one call whose body writes are
/// may-definitions, and a `catch` in a procedure with a one-block body is
/// lowered into blocks — and in the faithful-exceptions build, which gives a
/// `try` handler blocks and exception edges: an error after `k` stores leaves
/// those `k` and nothing else, so the handler is thrown to with them, and so
/// it is where a store after the body in the `try` splits it into blocks: the
/// state before the failing command reaches the handler only where that
/// command can fail before it stores. Each
/// program prints what tclsh 8.4 to 9.1 print, before and after `tcl opt`, in
/// every shape. Where a body's blocks are the analysis's own, what it proves
/// is exact: the place written before the error holds the value it was given
/// and the places after it hold what they did. A body that is one call
/// proves no such thing, and the places it may write hold no value.
#[test]
fn the_prefix_rule_holds_in_both_builds() {
    for program in &PREFIX_PROGRAMS {
        let (top, in_proc, handled, split) = (
            program.at_the_top_level(),
            program.in_a_procedure(),
            program.with_a_handler(),
            program.with_a_store_after_in_the_try(),
        );
        prints_under_releases_from(&top, program.printed, program.first);
        prints_under_releases_from(&in_proc, program.printed, program.first);
        let handler_first = if program.first < "8.6" {
            "8.6"
        } else {
            program.first
        };
        prints_under_releases_from(&handled, program.printed, handler_first);
        prints_under_releases_from(&split, program.printed, handler_first);

        for dialect in ["tcl8.6", "tcl9.0"] {
            for fact in program.flattened {
                // The body's blocks are the analysis's own: it proves the fact.
                for source in [&in_proc, &handled, &split] {
                    let unit = unit_of(source, dialect);
                    let function = unit.procedures.get("::p").expect("the procedure");
                    assert_eq!(
                        PrefixProgram::claim(fact, function),
                        Claim::Holds,
                        "{}: {dialect}\n{source}",
                        program.name
                    );
                }
                // One call whose body writes are may-definitions proves
                // nothing of a place it may write, and never the value the
                // place held before it.
                let unit = unit_of(&top, dialect);
                assert_ne!(
                    PrefixProgram::claim(fact, &unit.top_level),
                    Claim::Contradicts,
                    "{}: {dialect}\n{top}",
                    program.name
                );
            }
        }
    }
}

/// A command of a `catch` body may fail wherever it stands, and the handler is
/// thrown to with what the body has stored by then: after `set x 1; catch
/// {set x 2; foo; set x 1}` the handler may see `x` at 2 — `foo` raises
/// between the stores — so the lattice holds no constant for it, the store of
/// 2 is not dead, and the program prints what tclsh prints. The optimiser
/// had deleted `set x 2` and `set a 2`, which print `1` and `a=3` where tclsh
/// 8.4 to 9.1 print `2` and `a=2`, and the handler's state was joined from the
/// state before the body and the state at its end alone.
#[test]
fn a_throw_between_two_writes_is_a_state_the_handler_sees() {
    let programs = [
        (
            "proc foo {} {error x}\nproc p {} {\n    set x 1\n    catch {set x 2; foo; set x 1}\n    puts $x\n}\np\n",
            "2\n",
            "set x 2",
        ),
        (
            "proc foo {} {error x}\nproc p {} {\n    set a 1\n    catch {set a 2; foo; set a 3}\n    puts \"a=$a\"\n}\np\n",
            "a=2\n",
            "set a 2",
        ),
    ];
    for (source, printed, store) in programs {
        let name = if source.contains("set a 1") { "a" } else { "x" };
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains(store),
                "{dialect}: {store} stays\n{rewritten}"
            );
            let held = last_value(source, dialect, "::p", name);
            assert!(
                !matches!(held, LatticeValue::Const(ConstValue::Int(1 | 3))),
                "{dialect}: the handler may see 2, not a constant 1 or 3: {held:?}"
            );
        }
        prints_under_releases_from(source, printed, "8.4");
    }
}

/// A statement after one that certainly raises never runs: after `array set b
/// {k v}`, the body `lassign {x y} a b; set z 1` stops at the array, so `z`
/// is never set where the handler runs, in a `catch` lowered into blocks and
/// in the body of a `try`, which stays one block.
#[test]
fn a_statement_after_a_certain_error_never_runs() {
    let in_catch = "proc p {} {\n    array set b {k v}\n    catch {lassign {x y} a b; set z 1} m\n    puts [info exists z]\n}\np\n";
    let in_try = "proc p {} {\n    array set b {k v}\n    try {lassign {x y} a b; set z 1} on error {} {}\n    puts [info exists z]\n}\np\n";
    for (source, first) in [(in_catch, "8.5"), (in_try, "8.6")] {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let unit = unit_of(source, dialect);
            assert_eq!(
                last_existence(&unit, "::p", "z"),
                UNBOUND,
                "{dialect}: z is never set\n{source}"
            );
        }
        prints_under_releases_from(source, "0\n", first);
    }
}

/// An error outside any `catch` or `try` is the procedure's own, with no
/// handler to be thrown to, and nothing that follows it is claimed: after
/// `lassign {x y} a b` over an array `b` a `catch` the procedure goes on to
/// run is still reached, as it was before the prefix rule.
#[test]
fn a_certain_error_outside_a_handler_claims_nothing() {
    let source = "proc p {} {\n    array set b {k v}\n    lassign {x y} a b\n    catch {set z 1}\n    return $z\n}\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        let reached = function
            .sccp
            .executable_blocks
            .iter()
            .any(|block| function.ssa.block_name(*block).starts_with("catch_body"));
        assert!(reached, "{dialect}: the body of the catch is reached");
    }
}

/// A store in a `catch` body is taken as run only where its place is proved
/// to take it. Tcl raises on a scalar store into an array and on an element
/// store into a scalar, and `catch` observes the error, so where the analysis
/// does not prove a variable's kind the script's completion is not exact and
/// the `catch` is left unevaluated. The kind is read from before the body in
/// every form: at the statement in the value form, before the marker that
/// states what the body may write in the statement form, and at the exit of
/// the block before a flattened body. The route had taken `set y 1` over a `y`
/// that may be an array as run, so `p 1` printed `zero`, and at the top level
/// the marker had made the array `b` either kind, so the program printed
/// `empty`, where tclsh prints `c=1` and `error`. Where the kind is proved, the
/// `catch` is still evaluated in each form, and the body's own earlier store
/// decides the kind a later one needs.
#[test]
fn a_catch_body_store_is_run_only_where_its_place_is_proved_to_take_it() {
    let value_form = "proc p {n} {\n    if {$n} {array set y {a 1}}\n    set c [catch {set y 1}]\n    if {$c == 0} {return zero}\n    return \"c=$c\"\n}\nputs [p 1]\nputs [p 0]\n";
    let flattened = "proc p {n} {\n    if {$n} {array set y {a 1}}\n    catch {set y 1} m\n    if {$m eq \"1\"} {return one}\n    return other\n}\nputs [p 1]\nputs [p 0]\n";
    let top_level = "set a old\narray set b {k keep}\ncatch {lassign {new second} a b} m2\nif {$m2 eq \"\"} {puts empty} else {puts error}\nputs $a\n";
    for dialect in DIALECTS {
        assert_ne!(
            last_value(value_form, dialect, "::p", "c"),
            LatticeValue::Const(ConstValue::Int(0)),
            "{dialect}: `y` may be an array"
        );
        let held = last_value(flattened, dialect, "::p", "m");
        assert!(
            !matches!(held, LatticeValue::Const(_)),
            "{dialect}: `y` may be an array: {held:?}"
        );
        assert_ne!(
            top_value_at(&unit_of(top_level, dialect), "m2", 1),
            Some(text("")),
            "{dialect}: `b` is an array"
        );
    }
    prints_under_every_release(value_form, "c=1\nzero\n");
    prints_under_every_release(flattened, "other\none\n");
    prints_under_releases_from(top_level, "error\nnew\n", "8.5");

    // The value form: the code each body completes with, where the kinds
    // its stores need are proved (`Some`) or not (`None`), and what tclsh
    // prints for it. An element store needs an array or nothing, which an
    // earlier element store of the body proves; a scalar store after one
    // raises.
    let value_forms = [
        ("set y 1", Some(0), "0"),
        ("set y(a) 1; set y(b) 2", Some(0), "0"),
        ("set y(a) 1; set y 2", None, "1"),
    ];
    for (body, code, printed) in value_forms {
        let source = format!(
            "proc p {{}} {{\n    set c [catch {{{body}}} m]\n    return $c\n}}\nputs [p]\n"
        );
        for dialect in DIALECTS {
            let held = value_at(&unit_of(&source, dialect), "::p", "c", 1);
            match code {
                Some(code) => assert_eq!(
                    held,
                    Some(LatticeValue::Const(ConstValue::Int(code))),
                    "{dialect}: {body}"
                ),
                None => assert!(
                    !matches!(held, Some(LatticeValue::Const(_))),
                    "{dialect}: {body}: {held:?}"
                ),
            }
        }
        prints_under_every_release(&source, &format!("{printed}\n"));
    }
    let into_an_array = "proc p {} {\n    array set y {k v}\n    set c [catch {set y(a) 1} m]\n    return $c\n}\nputs [p]\n";
    let flattened_proved = "proc p {} {\n    catch {set v 1} r\n    return $r\n}\nputs [p]\n";
    let top_proved = "set w 0\ncatch {set v 1; set w 2} r\nif {[info exists v]} {puts yes} else {puts no}\nputs \"$r $w\"\n";
    for dialect in DIALECTS {
        assert_eq!(
            value_at(&unit_of(into_an_array, dialect), "::p", "c", 1),
            Some(LatticeValue::Const(ConstValue::Int(0))),
            "{dialect}"
        );
        assert_eq!(
            last_value(flattened_proved, dialect, "::p", "r"),
            LatticeValue::Const(ConstValue::Int(1)),
            "{dialect}"
        );
        assert_eq!(
            top_value_at(&unit_of(top_proved, dialect), "r", 1),
            Some(LatticeValue::Const(ConstValue::Int(2))),
            "{dialect}"
        );
    }
    prints_under_every_release(into_an_array, "0\n");
    prints_under_every_release(flattened_proved, "1\n");
    prints_under_every_release(top_proved, "yes\n2 2\n");

    // The body's own earlier store decides the kind a later one needs: a
    // scalar then an element store raises, and an element then a scalar one.
    let scalar_then_element =
        "proc p {} {\n    catch {set y 1; set y(a) 2} m\n    return \"$m $y\"\n}\nputs [p]\n";
    let element_then_scalar = "catch {set y(a) 1; set y 2} m\nputs \"$m [array exists y]\"\n";
    for dialect in DIALECTS {
        let held = last_value(scalar_then_element, dialect, "::p", "m");
        assert!(
            !matches!(held, LatticeValue::Const(_)),
            "{dialect}: `y` is a scalar: {held:?}"
        );
        let held = top_value_at(&unit_of(element_then_scalar, dialect), "m", 1);
        assert!(
            !matches!(held, Some(LatticeValue::Const(_))),
            "{dialect}: `y` is an array: {held:?}"
        );
    }
    prints_under_every_release(
        scalar_then_element,
        "can't set \"y(a)\": variable isn't array 1\n",
    );
    prints_under_every_release(
        element_then_scalar,
        "can't set \"y\": variable is array 1\n",
    );
}

/// The loops a `catch` body holds, each in the statement form in a procedure:
/// the body of the loop, a command after the loop in the same script, what
/// tclsh 8.4 to 9.1 leave in the result variable, and the first release with
/// what the body uses. A loop absorbs its body's `break` and `continue`, so
/// the script goes on past the loop; every other completion is the loop's
/// own, so the script ends there with it.
const LOOP_BODIES: [(&str, &str, &str, &str); 12] = [
    (
        "foreach x {1 2 3} {set y $x; break}",
        "set z $y",
        "1",
        "8.4",
    ),
    (
        "foreach x {1 2 3} {set y $x; continue; set y no}",
        "set z $y",
        "3",
        "8.4",
    ),
    (
        "foreach x {1 2} {set y $x; error boom}",
        "set z done",
        "boom",
        "8.4",
    ),
    (
        "foreach x {1 2} {set y $x; return r}",
        "set z done",
        "r",
        "8.4",
    ),
    (
        "foreach x {1 2} {foreach y {a b} {break}; set z $x}",
        "set w $z",
        "2",
        "8.4",
    ),
    (
        "set z none; foreach x {} {set z ran}",
        "set z",
        "none",
        "8.4",
    ),
    ("foreach x {1 2} {set x}", "", "", "8.4"),
    (
        "foreach {a b} {1 2 3} {set z \"$a:$b\"}",
        "set z",
        "3:",
        "8.4",
    ),
    ("lmap x {1 2 3 4} {set x}", "", "1 2 3 4", "8.6"),
    ("lmap x {1 2 3} {set y $x; continue}", "", "", "8.6"),
    ("lmap x {1 2 3} {set y $x; break}", "", "", "8.6"),
    (
        "foreach x {1 2 3} {set y $x; break;}",
        "set z $y",
        "1",
        "8.4",
    ),
];

/// A loop in a `catch` body runs as its iteration plan says
/// (`IterationPlan::step`, `LoopResult::of`): `break` ends the loop and
/// `continue` the iteration, each absorbed, and an error, a `return`, a code of
/// the body's own and a `break` at level 1, which the loop sees as a
/// `return`, leave it. `foreach` yields the empty string and `lmap` the
/// results of the iterations that completed normally. The loop's variables
/// are stores of the script, taken where their kind is proved: an array
/// raises after the stores before it, and a variable of unproven kind, like a
/// list the analysis does not know, leaves the `catch` unevaluated. Each
/// program prints what tclsh prints before and after `tcl opt`.
#[test]
fn a_loop_absorbs_break_and_continue() {
    for (body, after, held, first) in LOOP_BODIES {
        let script = if after.is_empty() {
            body.to_owned()
        } else {
            format!("{body}; {after}")
        };
        let source = format!(
            "proc p {{}} {{\n    catch {{{script}}} m\n    return \"<$m>\"\n}}\nputs [p]\n"
        );
        let dialects: &[&str] = if first == "8.6" {
            &["tcl8.6", "tcl9.0"]
        } else {
            &DIALECTS
        };
        for dialect in dialects {
            assert_eq!(
                lattice_text(last_value(&source, dialect, "::p", "m")).as_deref(),
                Some(held),
                "{dialect}: {script}"
            );
        }
        prints_under_releases_from(&source, &format!("<{held}>\n"), first);
    }

    // The value form, the loop variable also set where the script starts, so
    // it is among what the statement writes: the code and the message.
    let value_forms = [
        ("foreach x {1 2} {break}", 0, "", "0 {}", "8.4"),
        ("foreach x {1 2} {error boom}", 1, "boom", "1 boom", "8.4"),
        (
            "foreach x {1 2} {return -code break b}",
            2,
            "b",
            "2 b",
            "8.4",
        ),
        (
            "foreach x {1 2} {return -level 0 -code 5 v}",
            5,
            "v",
            "5 v",
            "8.5",
        ),
    ];
    for (body, code, held, printed, first) in value_forms {
        let source = format!(
            "proc p {{}} {{\n    set c [catch {{set x 0; {body}}} m]\n    return [list $c $m]\n}}\nputs [p]\n"
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let unit = unit_of(&source, dialect);
            assert_eq!(
                value_at(&unit, "::p", "c", 1),
                Some(LatticeValue::Const(ConstValue::Int(code))),
                "{dialect}: {body}"
            );
            assert_eq!(
                value_at(&unit, "::p", "m", 1)
                    .and_then(lattice_text)
                    .as_deref(),
                Some(held),
                "{dialect}: {body}"
            );
        }
        prints_under_releases_from(&source, &format!("{printed}\n"), first);
    }

    // A loop variable the script proved an array raises there, after the
    // variables bound before it: the code is 1 and `a` is `new`.
    let raises = "proc p {} {\n    set c [catch {set a old; set b(k) keep; foreach {a b} {new second} {set inside 1}} m]\n    return [list $c $a [info exists inside]]\n}\nputs [p]\n";
    for dialect in DIALECTS {
        let unit = unit_of(raises, dialect);
        assert_eq!(
            value_at(&unit, "::p", "c", 1),
            Some(LatticeValue::Const(ConstValue::Int(1))),
            "{dialect}"
        );
        assert_eq!(
            last_value(raises, dialect, "::p", "a"),
            text("new"),
            "{dialect}"
        );
    }
    prints_under_every_release(raises, "1 new 0\n");

    // What leaves the `catch` unevaluated, and the variable that raises: an
    // unknown list, a loop variable that may be an array, and one that is.
    let undecided = [
        "proc p {l} {\n    catch {foreach x $l {break}; set z done} m\n    return [string equal $m done]\n}\nputs [p {1 2}]\n",
        "proc p {n} {\n    if {$n} {array set x {a 1}}\n    catch {foreach x {1 2} {break}; set z done} m\n    return [string equal $m done]\n}\nputs [p 1]\nputs [p 0]\n",
        "proc p {} {\n    array set b {k keep}\n    set a old\n    catch {foreach {a b} {new second} {set inside 1}; set z done} m\n    return \"[string equal $m done] $a [info exists inside]\"\n}\nputs [p]\n",
    ];
    let printed = ["1\n", "0\n1\n", "0 new 0\n"];
    for (source, printed) in undecided.into_iter().zip(printed) {
        for dialect in DIALECTS {
            let held = last_value(source, dialect, "::p", "m");
            assert_ne!(held, text("done"), "{dialect}:\n{source}");
        }
        prints_under_every_release(source, printed);
    }
}

/// A store ahead of a write that may stop part-way is dead only where every
/// path through the body overwrites it. `lassign` assigns its targets in
/// order and raises at the first it cannot write, so a store into a target it
/// reaches before the one that may fail is overwritten on every path, and a
/// store into a target after it is not. O109 keeps `set b old` ahead of
/// `lassign {new second} a b` where `a` may be or is an array, which leaves
/// `b` old, keeps `set c old` ahead of `lassign {x y z} a b c` where `b` is an
/// array, in a `try` and in `[catch {…}]`, since the command that raised
/// preserved `c`, and deletes `set a old` ahead of `lassign {new second} a b`
/// in a `try` where
/// `b` is an array: the body's first command certainly raises after it stores
/// `a`, so the solver leaves the edge from the block before the body to the
/// handler closed, and no handler reads `old`. Every program prints what
/// tclsh 8.5 (8.6 for `try`) to 9.1 prints before and after `tcl opt`.
#[test]
fn o109_refuses_the_store_ahead_of_a_partial_lassign() {
    let kept = [
        (
            "proc p {c} {\n    if {$c} {array set a {k v}}\n    set b old\n    catch {lassign {new second} a b}\n    return $b\n}\nputs [p 0]\nputs [p 1]\n",
            "second\nold\n",
            "8.5",
        ),
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    try {lassign {new second} a b} on error {} {}\n    return $b\n}\nputs [p]\n",
            "old\n",
            "8.6",
        ),
        (
            "proc p {c} {\n    if {$c} {array set a {k v}}\n    set b old\n    try {lassign {new second} a b} on error {} {}\n    return $b\n}\nputs [p 0]\nputs [p 1]\n",
            "second\nold\n",
            "8.6",
        ),
        (
            "proc p {} {\n    array set b {k v}\n    set c old\n    try {lassign {x y z} a b c} on error {} {}\n    return [list $c [info exists a]]\n}\nputs [p]\n",
            "old 1\n",
            "8.6",
        ),
        (
            "proc p {} {\n    array set b {k v}\n    set c old\n    set r [catch {lassign {x y z} a b c} m]\n    if {[info exists c]} {lappend r $c}\n    return $r\n}\nputs [p]\n",
            "1 old\n",
            "8.5",
        ),
    ];
    for (source, printed, first) in kept {
        for dialect in ["tcl8.6", "tcl9.0"] {
            // The store stays while its variable is read: a read the
            // optimiser forwards `old` into reads no store.
            let (rewritten, rewrites) = optimised(source, dialect);
            let kept = ["b", "c"].iter().all(|name| {
                !source.contains(&format!("set {name} old"))
                    || rewritten.contains(&format!("set {name} old"))
                    || !rewritten.contains(&format!("${name}"))
            });
            assert!(
                kept,
                "{dialect}: the store `lassign` may never reach stays\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
    // Deleted: a store every path overwrites, and one a raise preserved where
    // nothing reads what it preserved.
    let deleted = [
        (
            "proc p {} {\n    set a old\n    array set b {k keep}\n    try {lassign {new second} a b} on error {m} {}\n    return $a\n}\nputs [p]\n",
            "set a old",
            "new\n",
        ),
        (
            "proc p {} {\n    array set b {k v}\n    set c old\n    try {lassign {x y z} a b c} on error {} {}\n    return done\n}\nputs [p]\n",
            "set c old",
            "done\n",
        ),
    ];
    for (source, store, printed) in deleted {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                !rewritten.contains(store)
                    && rewrites
                        .iter()
                        .any(|rewrite| matches!(rewrite.code, DiagCode::O109 | DiagCode::O126)),
                "{dialect}: `{store}` is dead\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// A write that may not run leaves the store ahead of it to the reads after
/// it, in the three shapes a sweep of generated bodies found folding or
/// deleting that store. `scan` makes every store it can and raises after the
/// last, so with `a` an array `scan {1 2} {%d %d} a b` leaves `b` 2, which
/// the route proves in a `catch`, a `try` and `[catch {…}]`. A write in a
/// `catch` or `try` body inside a substitution happens only where the body
/// has not stopped first, so `set c old` stays ahead of `[catch {lassign {x
/// y z} a b c} m]` where `b` may be an array — as a value, in a condition,
/// and nested in a word or an expression of the body — and `set b old`
/// ahead of `[catch {set a x; set b [error mid]} m]`, whose word raises. And
/// a `try` body that ends in a raise may fail at its first command before
/// it stores, so `set b old` stays ahead of `try {lassign {x y} a b; error
/// boom} on error {} {}` where `a` may be an array. Every program prints
/// what tclsh prints before and after `tcl opt`, from the first release
/// that has its commands.
#[test]
fn a_store_ahead_of_a_write_that_may_not_run_stays() {
    let scans = [
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    catch {scan {1 2} {%d %d} a b} m\n    return $b\n}\nputs [p]\n",
            "8.4",
        ),
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    try {scan {1 2} {%d %d} a b} on error {m} {}\n    return $b\n}\nputs [p]\n",
            "8.6",
        ),
        (
            "proc p {} {\n    array set a {k v}\n    set b old\n    set r [catch {scan {1 2} {%d %d} a b} m]\n    return $b\n}\nputs [p]\n",
            "8.4",
        ),
    ];
    for (source, first) in scans {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert_eq!(
                lattice_text(last_value(source, dialect, "::p", "b")),
                Some("2".to_owned()),
                "{dialect}: scan writes b after failing on a\n{source}"
            );
        }
        prints_under_releases_from(source, "2\n", first);
    }
    let kept = [
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    set r [catch {lassign {x y z} a b c} m]\n    if {[info exists c]} {lappend r $c}\n    return $r\n}\nputs [p 0]\nputs [p 1]\n",
            "0 z\n1 old\n",
            "8.5",
        ),
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    if {[catch {lassign {x y z} a b c} m]} {return [list raised $c]}\n    return [list ok $c]\n}\nputs [p 0]\nputs [p 1]\n",
            "ok z\nraised old\n",
            "8.5",
        ),
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    set r [catch {set q [lassign {x y z} a b c]} m]\n    return [list $r $c]\n}\nputs [p 0]\nputs [p 1]\n",
            "0 z\n1 old\n",
            "8.5",
        ),
        (
            "proc p {n} {\n    if {$n} {array set b {k v}}\n    set c old\n    set r [catch {expr {[lassign {x y z} a b c] eq {}}} m]\n    return [list $r $c]\n}\nputs [p 0]\nputs [p 1]\n",
            "0 z\n1 old\n",
            "8.5",
        ),
        (
            "proc p {} {\n    set b old\n    set r [catch {set a x; set b [error mid]} m]\n    return [list $r $a $b]\n}\nputs [p]\n",
            "1 x old\n",
            "8.4",
        ),
        (
            "proc p {n} {\n    if {$n} {array set a {k v}}\n    set b old\n    try {lassign {x y} a b; error boom} on error {m} {}\n    return $b\n}\nputs [p 0]\nputs [p 1]\n",
            "y\nold\n",
            "8.6",
        ),
    ];
    for (source, printed, first) in kept {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            let kept = ["b", "c"].iter().all(|name| {
                !source.contains(&format!("set {name} old"))
                    || rewritten.contains(&format!("set {name} old"))
                    || !rewritten.contains(&format!("${name}"))
            });
            assert!(
                kept,
                "{dialect}: the store a write may not reach stays\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// The programs of [`a_raising_store_is_never_dead`]: each source, the raising
/// store it keeps, what it prints and the first release that runs it.
const RAISING_STORES: [(&str, &str, &str, &str); 16] = [
    (
        "proc p {} {\n    set x 1\n    catch {set x [expr {1/0}]; set y 2} m\n    return [list $m [info exists y]]\n}\nputs [p]\n",
        "set x [expr {1/0}]",
        "{divide by zero} 0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    catch {set x [lindex {a b} 1.5]; set y 2} m\n    return [info exists y]\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    catch {set x [lindex {a b} 1.5]; set y 2}\n    return [info exists y]\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x abc\n    catch {incr x; set y 2} m\n    return [list [info exists y] $x]\n}\nputs [p]\n",
        "incr x",
        "0 abc\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a 1\n    catch {unset a b; set y 2} m\n    return [list [info exists y] [info exists a]]\n}\nputs [p]\n",
        "unset a b",
        "0 0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    set c [catch {foreach i {1 2 3} {set x [lindex {a b} 1.5]; set y 2}}]\n    return [list $c [info exists y]]\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "1 0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    try {set x [lindex {a b} 1.5]; set y 2} on error {m} {return [info exists y]}\n    return none\n}\nputs [p]\n",
        "set x [lindex {a b} 1.5]",
        "0\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x 1\n    try {set x [expr {1/0}]; set y 2} finally {puts \"fin [info exists y]\"}\n}\ncatch {p} m\nputs $m\n",
        "set x [expr {1/0}]",
        "fin 0\ndivide by zero\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x [format %d abc]\n    return 1\n}\nputs [catch {p}]\n",
        "set x [format %d abc]",
        "1\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set x 1\n    try {set x [lindex {a b} 1.5]; set y 2} finally {puts \"fin [info exists y]\"}\n}\nputs [catch {p}]\n",
        "set x [lindex {a b} 1.5]",
        "fin 0\n1\n",
        "8.6",
    ),
    (
        "proc p {} {\n    set x 1\n    catch {set x [string range abc 1 x]; set y 2} m\n    return [info exists y]\n}\nputs [p]\n",
        "set x [string range abc 1 x]",
        "0\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a old; set c old; array set b {k v}\n    catch {scan {1 2 3} {%d %d %d} a b c; set y 2} m\n    return [list [info exists y] $a $c]\n}\nputs [p]\n",
        "scan {1 2 3} {%d %d %d} a b c",
        "0 1 3\n",
        "8.4",
    ),
    (
        "proc p {} {\n    set a old; set c old; array set b {k v}\n    catch {lassign {1 2 3} a b c; set y 2} m\n    return [list [info exists y] $a $c]\n}\nputs [p]\n",
        "lassign {1 2 3} a b c",
        "0 1 old\n",
        "8.5",
    ),
    (
        "proc q {x} {\n    catch {set x [expr {1/0}]; set y 2} m\n    puts $x\n    puts [list $m [info exists y]]\n}\nq 5\n",
        "set x [expr {1/0}]",
        "5\n{divide by zero} 0\n",
        "8.4",
    ),
    (
        "proc p {i} {\n    set a(k) 1\n    catch {set a($i) [expr {1/0}]; set y 2} m\n    return [list $m [info exists y]]\n}\nputs [p j]\n",
        "set a($i) [expr {1/0}]",
        "{divide by zero} 0\n",
        "8.4",
    ),
    (
        "proc add {a b} {\n    expr {$a + $b}\n}\nproc f {} {\n    set unused [add x 1]\n    puts done\n}\nputs [catch f]\n",
        "set unused [add x 1]",
        "1\n",
        "8.4",
    ),
];

/// A store whose value raises is never dead, and the solver's proof that it
/// raises is no proof that it evaluated cleanly. Inside a `catch` or `try`
/// body the raise is the store's effect: `set x [expr {1/0}]` ahead of `set
/// y 2` stops the body there, so `y` is never set, though nothing reads the
/// `x` it would have stored and the solver gives that definition the value
/// `x` held before it. The same holds for a command substitution that raises
/// on its own words (`[lindex {a b} 1.5]`, `[string range abc 1 x]`), an
/// `incr` of a value that is no integer, an `unset` of an absent name, a
/// `scan` or `lassign` that stops at an array, a value-position `catch` over
/// a loop, a store to a parameter whose constant argument is propagated past
/// it, and a store to an element whose key is not known, and a command
/// substitution that raises outside any body stops the procedure (#2346's
/// arm: `set x [format %d abc]` unread, and a call to a pure procedure that
/// raises, `set unused [add x 1]`). Every program prints what tclsh
/// prints before and after `tcl opt`, from the first release that has its
/// commands, and keeps its raising store.
#[test]
fn a_raising_store_is_never_dead() {
    for (source, store, printed, first) in RAISING_STORES {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(source, dialect);
            assert!(
                rewritten.contains(store),
                "{dialect}: `{store}` raises and stays\n{rewritten}\n{rewrites:#?}"
            );
        }
        prints_under_releases_from(source, printed, first);
    }
}

/// One program of [`a_level_zero_return_completes_where_it_stands`].
struct LevelZeroReturn {
    source: &'static str,
    /// Statements a level-0 `return` leaves to run, which `tcl opt` keeps.
    kept: &'static [&'static str],
    /// Statements after a level-0 jump or error, which never run and O107
    /// removes.
    gone: &'static [&'static str],
    printed: &'static str,
    /// The first release that runs it.
    first: &'static str,
}

/// The programs of [`a_level_zero_return_completes_where_it_stands`].
const LEVEL_ZERO_RETURNS: [LevelZeroReturn; 5] = [
    LevelZeroReturn {
        source: "proc p {} {return -level 0 -code ok x; puts \"after in p\"; return done}\nputs [p]\nreturn -level 0 -code ok top\nputs \"after at top\"\n",
        kept: &["puts \"after in p\"", "puts \"after at top\""],
        gone: &[],
        printed: "after in p\ndone\nafter at top\n",
        first: "8.5",
    },
    LevelZeroReturn {
        source: "try {return -level 0 -code 0 x} on ok {v} {puts \"ok $v\"}\ntry {return -level 0 -code 2 x} on return {v} {puts \"ret $v\"}\n",
        kept: &["puts \"ok $v\"", "puts \"ret $v\""],
        gone: &[],
        printed: "ok x\nret x\n",
        first: "8.6",
    },
    LevelZeroReturn {
        source: "proc p {} {\n    set n 0\n    foreach i {1 2 3 4} {\n        if {$i == 2} {return -level 0 -code continue}\n        if {$i == 4} {return -level 0 -code break}\n        incr n\n    }\n    return $n\n}\nputs [p]\n",
        kept: &["incr n"],
        gone: &[],
        printed: "2\n",
        first: "8.5",
    },
    LevelZeroReturn {
        source: "proc e {} {\n    return -level 0 -code error -errorcode {A B} boom\n    set z 1\n    puts never\n}\nputs [catch e m]\nputs $m\nputs $::errorCode\n",
        kept: &[],
        gone: &["puts never"],
        printed: "1\nboom\nA B\n",
        first: "8.5",
    },
    LevelZeroReturn {
        source: "proc p {} {\n    foreach i {1 2 3} {\n        return -level 0 -code break\n        set xb 1\n    }\n    foreach i {1 2} {\n        return -level 0 -code continue\n        set yc 1\n    }\n    return [list [info exists xb] [info exists yc]]\n}\nputs [p]\nproc q {} {\n    catch {return -level 0 -code error e; set xe 1}\n    return [info exists xe]\n}\nputs [q]\n",
        kept: &["return [list [info exists xb] [info exists yc]]"],
        gone: &["set xb 1", "set yc 1"],
        printed: "0 0\n0\n",
        first: "8.5",
    },
];

/// A `return` at level 0 completes where it stands, with its code (#2357):
/// `ok` runs on to the next statement, `error` raises, and `break` or
/// `continue` leaves the loop around it. One decoding of `return`'s options,
/// the registry's, answers the lowering, the CFG and the solver alike, and
/// read a level-0 `return` as the procedure's exit no longer: O107 deleted
/// the statements after `return -level 0 -code ok x` in a procedure and at
/// the top level, and the `try` handlers its code selects. In a procedure, at
/// the top level, in a loop and under `try`, each program prints what tclsh
/// prints before and after `tcl opt`, from the first release with `-level`,
/// and what follows a level-0 jump or error in its block never runs, which
/// O107 removes. Under 8.4, which reads three options, `-level` is rejected,
/// and `catch` reports it.
#[test]
fn a_level_zero_return_completes_where_it_stands() {
    for program in &LEVEL_ZERO_RETURNS {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, rewrites) = optimised(program.source, dialect);
            for statement in program.kept {
                assert!(
                    rewritten.contains(statement),
                    "{dialect}: `{statement}` runs after a level-0 return\n{rewritten}\n{rewrites:#?}"
                );
            }
            for statement in program.gone {
                assert!(
                    !rewritten.contains(statement),
                    "{dialect}: `{statement}` never runs after a level-0 jump or error\n{rewritten}\n{rewrites:#?}"
                );
            }
        }
        prints_under_releases_from(program.source, program.printed, program.first);
    }
    let rejected = "proc p {} {catch {return -level 0 -code ok x} m; return $m}\nputs [p]\n";
    for (series, tclsh) in releases_on_path() {
        let printed = if series == "8.4" {
            "bad option \"-level\": must be -code, -errorcode, or -errorinfo\n"
        } else {
            "x\n"
        };
        let (rewritten, _) = optimised(rejected, &dialect_of(series));
        for program in [rejected, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// The e5 program with its stub block in place: `db_query` and `db_eval`
/// declared under each `-frame` word, called in a procedure that holds `g`.
fn stubbed_e5(frame: &str) -> String {
    format!(
        "# tcl-lsp: stubs-begin\n# tcl-lsp: stub db_query {{sql}}{frame}\n\
         # tcl-lsp: stub db_eval {{sql script:body}}{frame}\n# tcl-lsp: stubs-end\n\
         proc q {{}} {{\n    set g 5\n    db_query {{select 1}}\n    \
         if {{$g == 5}} {{puts five}} else {{puts other}}\n}}\n\
         proc p {{}} {{\n    set g 5\n    db_eval {{select 1}} {{set g 6}}\n    \
         if {{$g == 5}} {{puts five}} else {{puts other}}\n}}\nq\np\n"
    )
}

/// A stub that states its frame effect is a command the module can name
/// (D255). `-frame own` and `-frame none` state that the call crosses no
/// frame, so the locals `q` holds at `db_query {select 1}` keep their values
/// and `$g == 5` is decided; `-frame caller` states `argparse`'s effect, and a
/// stub that states none may reach the frame that calls it as code the module
/// cannot see may, so the condition stays open under both. A stub with a body
/// role answers as the registry's rule answers for one: `db_eval`'s call is
/// the barrier `time {…}` lowers to, and the condition after it stays open
/// whatever the frame effect. Each declaration runs under tclsh with a
/// definition it describes — a procedure that returns, or one that sets its
/// caller's `g` through `upvar 1` — and prints what the claims allow.
#[test]
fn a_stub_that_states_its_frame_effect_is_a_named_head() {
    let own = "proc db_query {sql} {return 1}\nproc db_eval {sql script} {uplevel 1 $script}\n";
    let caller = "proc db_query {sql} {upvar 1 g g; set g 6}\nproc db_eval {sql script} {uplevel 1 $script}\n";
    for (frame, definitions, claims, printed) in [
        (" -frame own", own, &[true][..], "five\nother\n"),
        (" -frame none", own, &[true][..], "five\nother\n"),
        (" -frame caller", caller, &[][..], "other\nother\n"),
        ("", caller, &[][..], "other\nother\n"),
    ] {
        let source = stubbed_e5(frame);
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                condition_claims(&source, dialect),
                claims,
                "{dialect}{frame}:\n{source}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            assert_eq!(
                run_script(&tclsh, &format!("{definitions}{source}")),
                Some((true, printed.to_owned())),
                "tclsh{series}{frame}"
            );
        }
    }
    let registry = static_context_for("tcl8.6").commands();
    let barrier_of = |source: &str, head: &str| {
        let declared =
            tcl_compiler::analyser::utils::document_declared_surface(source, None, "tcl8.6");
        let module = tcl_compiler::lowering::lower_to_ir_with(
            tcl_compiler::lowering::Lowerer::with_config(
                registry,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
            )
            .with_declared_commands(Some(&declared)),
            source,
        );
        let mut reasons = Vec::new();
        tcl_compiler::ir::for_each_statement(&module.procedures["::p"].body, &mut |statement| {
            if let Statement::Barrier {
                command, reason, ..
            } = statement
                && command == head
            {
                reasons.push(reason.clone());
            }
        });
        reasons
    };
    let timed = barrier_of(
        "proc p {} {\n    set g 5\n    time {set g 6}\n    puts $g\n}\n",
        "time",
    );
    assert_eq!(timed, ["unsupported body command"]);
    for frame in ["", " -frame own", " -frame none", " -frame caller"] {
        assert_eq!(barrier_of(&stubbed_e5(frame), "db_eval"), timed, "{frame}");
    }
    // Run as a callback, a command that reaches the frame it runs in writes
    // the global frame: `$done` stays open after `update` under `-frame
    // caller`, and with no `-frame`, where tclsh prints 1.
    let callback = |frame: &str| {
        format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub db_bind {{spec}}{frame}\n\
             # tcl-lsp: stubs-end\nset done 0\nafter 10 {{db_bind {{done}}}}\nafter 50\n\
             update\nif {{$done}} {{puts 1}} else {{puts 0}}\n"
        )
    };
    let reaches = "proc db_bind {spec} {upvar 1 done d; set d 1}\n";
    for (frame, definition, claims, printed) in [
        (" -frame caller", reaches, &[][..], "1\n"),
        ("", reaches, &[][..], "1\n"),
        (
            " -frame own",
            "proc db_bind {spec} {return}\n",
            &[false][..],
            "0\n",
        ),
    ] {
        let source = callback(frame);
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            assert_eq!(
                condition_claims(&source, dialect),
                claims,
                "{dialect}{frame}:\n{source}"
            );
        }
        for (series, tclsh) in releases_on_path() {
            assert_eq!(
                run_script(&tclsh, &format!("{definition}{source}")),
                Some((true, printed.to_owned())),
                "tclsh{series}{frame}"
            );
        }
    }
}

/// A lambda an `after` callback applies runs its body in a frame of its own,
/// so what it writes in the global frame is the callback's write: the scan of
/// callback scripts reads the lambda as a procedure's summary reads a body,
/// and `$done` after `update` decides nothing. Each program prints `changed`
/// and `1` under tclsh 8.5 to 9.1, before and after `tcl opt`, where the
/// rewrite had printed `zero` and `0`.
#[test]
fn a_lambda_a_callback_applies_writes_the_globals_its_body_writes() {
    for (lambda, words) in [
        ("{} {global done; set done 1}", ""),
        ("{} {set ::done 1}", ""),
        ("{} {upvar #0 done d; set d 1}", ""),
        ("{} {incr ::done}", ""),
        ("{} {global done; foreach done {1} {}}", ""),
        ("{} {global done; catch {set done 1}}", ""),
        ("{} {uplevel #0 {set done 1}}", ""),
        ("{} {apply {{} {global done; set done 1}}}", ""),
        ("{} {after 0 {set done 1}; update}", ""),
        ("{x} {global done; set done $x}", " 1"),
    ] {
        for callback in [
            format!("after 10 {{apply {{{lambda}}}{words}}}"),
            format!("after 10 [list apply {{{lambda}}}{words}]"),
        ] {
            let source = format!(
                "set done 0\n{callback}\nafter 50\nupdate\n\
                 if {{$done eq 0}} {{puts zero}} else {{puts changed}}\nputs $done\n"
            );
            prints_under_releases_from(&source, "changed\n1\n", "8.5");
        }
    }
}

/// A `try` body that cannot fall through is thrown to from the point it
/// raises at, and its first command may fail before it stores anything, with
/// the state the body entered with: after `try {set x [expr {1 / $d}]; error
/// boom} on error {} {}` the handler sees `x` unbound where the division
/// raised, so `info exists x` decides nothing, the program prints `no` for a
/// zero divisor and `yes` for any other, and it does so before and after `tcl
/// opt`.
#[test]
fn a_body_that_ends_in_an_error_may_have_failed_at_its_start() {
    let source = "proc p {d} {\n    try {set x [expr {1 / $d}]; error boom} on error {} {}\n    if {[info exists x]} {return yes}\n    return no\n}\nputs [p 0]\nputs [p 1]\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            rewritten.contains("return no"),
            "{dialect}: x may be unbound where the handler runs\n{rewritten}"
        );
    }
    prints_under_releases_from(source, "no\nyes\n", "8.6");
}

/// The ways a `try` body or one of its handlers leaves, each with a `finally`
/// clause that sets the global `g`: what a procedure runs and why the clause
/// is reached.
const FINALLY_PATHS: [(&str, &str); 13] = [
    ("an error", "try {error boom} finally {set g 1}"),
    ("a throw", "try {throw {A B} boom} finally {set g 1}"),
    ("a return", "try {return early} finally {set g 1}"),
    (
        "a break out of a loop",
        "while 1 { try {break} finally {set g 1} }",
    ),
    (
        "an error in a loop",
        "foreach i {1 2} { try {error boom} finally {set g 1} }",
    ),
    (
        "every branch of an if leaving",
        "try {if {[info exists ::c]} {return ok} else {error boom}} finally {set g 1}",
    ),
    (
        "every arm of a switch leaving",
        "try {switch [info exists ::c] {1 {return ok} default {error boom}}} finally {set g 1}",
    ),
    ("a tailcall", "try {tailcall list} finally {set g 1}"),
    (
        "a handler's return",
        "try {error boom} on error {} {return handled} finally {set g 1}",
    ),
    (
        "a handler's error",
        "try {error boom} on error {} {error again} finally {set g 1}",
    ),
    (
        "a handler's break",
        "foreach i {1 2} { try {error boom} on error {} {break} finally {set g 1} }",
    ),
    (
        "an exit that rejects its status",
        "try {exit abc} finally {set g 1}",
    ),
    (
        "an error only a trap might take",
        "try {error boom} trap {NOT MATCHING} {} {exit 0} finally {set g 1}",
    ),
];

/// A `finally` clause runs however the `try` body or a handler leaves, so its
/// body is live on every path (#2142). Each statement of [`FINALLY_PATHS`] sets
/// the global `g` in its clause, and `catch {p}; puts $g` prints `1` under
/// tclsh 8.6 to 9.1 before and after the optimiser, which had emptied the
/// clause and printed `0`; so does a `return $x` whose substitution only a
/// handler that exits may take, and a handler that exits once its variable is
/// bound, where a write trace refuses the binding. In a procedure, `set f 0;
/// try {error boom} finally {set f 1}; return $f` reads no unset `f`: the body
/// always raises, so the `return` never runs and both stores are dead — W220
/// is a true positive on each — and once a `catch` lets the `return` run, the
/// clause's store is the one it reads. The issue's two programs that must keep
/// firing still do: a handler that may not run binds nothing (W210), and a
/// handler's store the clause overwrites is dead (W220).
#[test]
fn try_finally_runs_on_every_path() {
    let mut sources: Vec<(String, &str)> = FINALLY_PATHS
        .iter()
        .map(|(why, statement)| {
            (
                format!(
                    "set g 0\nproc p {{}} {{\n    global g\n    {statement}\n}}\ncatch {{p}}\nputs $g\n"
                ),
                *why,
            )
        })
        .collect();
    sources.push((
        "set g 0\nproc p {x} {\n    global g\n    try {return $x} on error {} {exit 0} finally {set g 1}\n}\np 5\nputs $g\n".to_owned(),
        "a return a handler takes only the substitution's error of",
    ));
    sources.push((
        "set g 0\nproc tr args {error TRACE}\nproc p {} {\n    global g\n    trace add variable msg write tr\n    try {error boom} on error msg {exit 0} finally {set g 1}\n}\ncatch p\nputs $g\n".to_owned(),
        "a handler whose binding a trace refuses before its exit",
    ));
    for (source, why) in &sources {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains("set g 1"),
                "{why}: {dialect}: the clause runs on this path\n{rewritten}"
            );
        }
        prints_under_releases_from(source, "1\n", "8.6");
    }

    let issue =
        "proc p {} {\n    set f 0\n    try {error boom} finally {set f 1}\n    return $f\n}\n";
    let caught = "proc p {} {\n    set f 0\n    catch {try {error boom} finally {set f 1}}\n    return $f\n}\nputs [p]\n";
    let unbound =
        "proc q {c} {\n    try {if {$c} {error boom}} on error {} {set g 1}\n    return $g\n}\n";
    let overwritten = "proc p {} {\n    try {error boom} on error {} {set f 2} finally {set f 1}\n    return $f\n}\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            !reports(issue, dialect, DiagCode::W210),
            "{dialect}: the clause binds `f` before any read\n{issue}"
        );
        assert!(
            reports(issue, dialect, DiagCode::W220),
            "{dialect}: the `return` never runs, so both stores are dead\n{issue}"
        );
        assert!(
            !reports(caught, dialect, DiagCode::W210),
            "{dialect}\n{caught}"
        );
        let (rewritten, _) = optimised(caught, dialect);
        assert!(
            rewritten.contains("set f 1"),
            "{dialect}: the clause's store is read\n{rewritten}"
        );
        assert!(
            reports(unbound, dialect, DiagCode::W210),
            "{dialect}: a handler that may not run binds nothing\n{unbound}"
        );
        assert!(
            reports(overwritten, dialect, DiagCode::W220),
            "{dialect}: the clause overwrites the handler's store\n{overwritten}"
        );
    }
    prints_under_releases_from(caught, "1\n", "8.6");
}

/// A `try`'s handlers are read once, by the registry's handler chain, and the
/// control-flow graph wires the edges it names. The first match runs, so a
/// handler an earlier unconditional one pre-empts runs nothing — a `-` handler
/// pre-empts like any other, a `trap` never does; a `-` handler runs its
/// owner's script, which a match reaches with the group's names bound; and a
/// `break` the first matching handler selects goes to that handler, not to its
/// loop. Each program prints what tclsh 8.6 to 9.1 print, before and after the
/// optimiser, and reads no unset name (no W210).
#[test]
fn try_handlers_run_as_the_registry_chain_reads_them() {
    let programs = [
        (
            "proc p {} {\n    try {error boom} on error {} {set x 1} on error {} {return} finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    set x 1\n    try {error boom} on error {} - on ok {} {} on error {} {unset x; return} finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    set x 0\n    try {error boom} on error {} - on ok {} {set x 1} finally {}\n    return $x\n}\nputs [p]\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try {error boom} on error {} - on ok {} {set x 1} finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc q {} {\n    try {error boom} on error {m} - on ok {} {return \"caught $m\"}\n    return after\n}\nputs [q]\n",
            "caught boom\n",
        ),
        (
            "proc s {} {\n    try {error boom} trap {X} {} {return T} on error {} {return E}\n}\nputs [s]\n",
            "E\n",
        ),
        (
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1} finally {}\n        break\n    }\n    puts $x\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    while 1 {\n        try {break} on break {} {set x 1}\n        break\n    }\n    puts $x\n}\np\n",
            "1\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// A handler's selector is read with the target's numerals: `on 010` takes
/// code 8 up to 8.6 and code 10 from 9.0. A body that completes with code 8
/// reaches the handler under 8.6, so its store stays and `catch p; puts $g`
/// prints `1`; from 9.0 nothing takes the code, the handler never runs, its
/// store is dead and the program prints `0`. Each release prints the same
/// before and after the optimiser under its own dialect.
#[test]
fn a_handler_selector_reads_the_targets_numerals() {
    let source = "set g 0\nproc p {} {\n    global g\n    try {return -level 0 -code 8 boom} on 010 {} {set g 1} finally {}\n}\ncatch p\nputs $g\n";
    let (under_8_6, _) = optimised(source, "tcl8.6");
    assert!(
        under_8_6.contains("set g 1"),
        "8.6 reads `010` as 8: the handler runs\n{under_8_6}"
    );
    let (under_9_0, _) = optimised(source, "tcl9.0");
    assert!(
        !under_9_0.contains("set g 1"),
        "9.0 reads `010` as 10: the handler never runs\n{under_9_0}"
    );
    for (series, tclsh) in releases_on_path() {
        if series < "8.6" {
            continue;
        }
        let printed = if series == "8.6" { "1\n" } else { "0\n" };
        let (rewritten, _) = optimised(source, &dialect_of(series));
        for program in [source, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A handler that never runs — one an earlier handler pre-empts, a `-`
/// handler whose script is its owner's, one the body's completion is known
/// to miss, and one whose only way in is a normal completion the body never
/// makes — leaves the `try` that does run. The binding of a handler's
/// variables carries the span of the whole `try`, and the optimiser had
/// deleted it with the dead handler: the `try` vanished, with the handler
/// that runs and the `break` of its body. Each program prints what tclsh 8.6
/// to 9.1 print before and after the optimiser, and the `try` stays.
#[test]
fn a_handler_that_never_runs_keeps_its_try() {
    let programs = [
        (
            "proc p {} {\n    try {error boom} on error {} {puts first} on 1 {m} {puts second}\n    puts after\n}\np\n",
            "first\nafter\n",
            "try {error boom}",
        ),
        (
            "proc p {} {\n    try {error boom} on error {m} - on ok {} {puts \"caught $m\"}\n    puts after\n}\np\n",
            "caught boom\nafter\n",
            "try {error boom}",
        ),
        (
            "proc p {} {\n    foreach i {1 2} {\n        puts $i\n        try {break} on error {m} {puts caught}\n        puts after\n    }\n    puts done\n}\np\n",
            "1\ndone\n",
            "try {break}",
        ),
        (
            "proc p {} {\n    try {\n        try {error a} finally {}\n        puts unreached\n    } on ok {m} {puts ok} on error {} {puts caught}\n    puts after\n}\np\n",
            "caught\nafter\n",
            "try {error a}",
        ),
    ];
    for (source, printed, kept) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(source, dialect);
            assert!(
                rewritten.contains(kept),
                "{dialect}: the `try` stays\n{rewritten}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// What the `finally` clause resumes once it is done. A `try` that never
/// completes normally does not fall through its clause: after `try {break} on
/// error {} {} finally {}` in a loop, `set x 1` never runs and the read after
/// the loop is of an unset `x` (W210, a true positive: tclsh fails there), and
/// after `try {return early} on error {} {} finally {}` the code that follows
/// is dead (O107). A clause that itself transfers control keeps its transfer:
/// its `break` overrides a pending `return`, so the code after the loop runs.
/// A `return` an inner `try … finally` intercepts reaches the outer clause
/// through the inner one, so the name the inner clause binds is bound; an
/// inner handler that catches an error runs before the outer clause; a `break`
/// leaves through both clauses in order; and a `return` that passes an inner
/// clause resumes past the code after the inner `try`, where the outer clause
/// may read an unset name (W210, a true positive). Each program prints what
/// tclsh 8.6 to 9.1 print before and after the optimiser.
#[test]
fn a_finally_resumes_what_it_interrupted() {
    let never_completes = "proc p {} {\n    while 1 {\n        try {break} on error {} {} finally {}\n        set x 1\n    }\n    puts $x\n}\n";
    let returns = "proc p {} {\n    try {return early} on error {} {} finally {}\n    set x 1\n    return $x\n}\nputs [p]\n";
    let resumes_past = "proc p {c} {\n    try { try {if {$c} {return}} finally {}; set x 1 } finally {puts $x}\n}\np 0\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert!(
            reports(never_completes, dialect, DiagCode::W210),
            "{dialect}: `set x 1` never runs\n{never_completes}"
        );
        assert!(
            rewrites_of(returns, dialect)
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O107),
            "{dialect}: the code after the `try` is dead\n{returns}"
        );
        assert!(
            reports(resumes_past, dialect, DiagCode::W210),
            "{dialect}: the outer clause may read `x` unset\n{resumes_past}"
        );
        assert!(
            !rewrites_of(resumes_past, dialect)
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O102),
            "{dialect}: `x` has no single reaching definition\n{resumes_past}"
        );
    }
    prints_under_releases_from(returns, "early\n", "8.6");
    prints_under_releases_from(resumes_past, "1\n", "8.6");

    let programs = [
        (
            "proc p {} {\n    while 1 { try {return early} finally {break} }\n    set x 1\n    return \"after $x\"\n}\nputs [p]\n",
            "after 1\n",
        ),
        (
            "proc p {} {\n    try { try {return ok} finally {set x 1} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try {try {error boom} on error {} {set x 1; return} finally {set y 1}} finally {puts $x; puts $y}\n}\np\n",
            "1\n1\n",
        ),
        (
            "proc p {} {\n    set x 0\n    while 1 {\n        try { try {unset x; break} finally {set y 1} } finally {set x 5}\n    }\n    return $x\n}\nputs [p]\n",
            "5\n",
        ),
        (
            "proc p {} {\n    set x 0\n    while {$x < 3} {\n        try {unset x; continue} finally {set x 5}\n    }\n    return $x\n}\nputs [p]\n",
            "5\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// A process exit runs no `finally`: in `try {exit 7} finally {set g 1}` the
/// clause is never entered, so its store is dead code (O107), where a
/// `return` or an error would have run it.
#[test]
fn an_exit_runs_no_finally() {
    for body in ["exit", "exit 7"] {
        let source =
            format!("proc p {{}} {{\n    global g\n    try {{{body}}} finally {{set g 1}}\n}}\n");
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                rewrites_of(&source, dialect)
                    .iter()
                    .any(|rewrite| rewrite.code == DiagCode::O107),
                "{dialect}: the clause is never entered\n{source}"
            );
        }
    }
}

/// A body that leaves before its last statement is thrown to from where it
/// leaves, not from the dead code after it: the handler of `try {return r3;
/// set v 8} on return {} {puts t8}` runs, where its only edge had come from
/// the dead `set v 8` and the optimiser deleted the handler's body and the code
/// after the `try`; a dead `error` after the `return` is no throw point that
/// takes the `return`'s place; and a `break` caught by `on break` binds the
/// handler's store on each pass. Each program prints what tclsh 8.6 to 9.1
/// print, before and after the optimiser, and reads no unset name.
#[test]
fn a_handler_is_thrown_to_from_where_the_body_leaves() {
    let programs = [
        (
            "proc p {} {\n    try {return r3; set v 8} on return {} {puts t8}\n    return done\n}\nputs [p]\n",
            "t8\ndone\n",
        ),
        (
            "proc p {} {\n    set x 1\n    try {set x 5; return r3; set y 1} on return {} {puts \"h $x\"}\n    puts $x\n}\np\n",
            "h 5\n5\n",
        ),
        (
            "proc p {} {\n    try {return r3; error dead} on return {} {puts t8}\n    return done\n}\nputs [p]\n",
            "t8\ndone\n",
        ),
        (
            "proc p {} {\n    foreach i {1 2} {\n        try {break; set y 1} on break {} {set x $i}\n    }\n    puts $x\n}\np\n",
            "2\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
}

/// The ways a `try` body, or a script it holds, fails between two stores, each
/// with what tclsh 8.6 to 9.1 print and the store a failure leaves, which is
/// no dead store. `foo` raises with no argument and with `2`, and `p 1` sets
/// `x` to 1 before the statement, so each program prints the value stored
/// before the failure where the store after it would have given another.
const BETWEEN_TWO_STORES: [(&str, &str, &str, &str); 16] = [
    (
        "a handler",
        "try {set x 2; foo; set x 3} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a finally clause no handler stands in for",
        "try {set x 2; foo; set x 3} finally {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a finally clause, where the first command fails before it stores",
        "try {set x [foo]; set x 3} finally {puts $x}",
        "1\n",
        "set x 1",
    ),
    (
        "a handler and the clause after it",
        "try {set x 2; foo; set x 3} on error {} {puts \"h $x\"} finally {puts \"f $x\"}",
        "h 2\nf 2\n",
        "set x 2",
    ),
    (
        "a body that cannot fall through",
        "try {set x 2; foo; set x 3; error boom} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "an arm of an if",
        "try {if {$c} {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "an arm, with the clause",
        "try {if {$c} {set x 2; foo; set x 3}} finally {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a loop body",
        "try {foreach i {1 2} {set x $i; foo $i; set x 9}} on error {} {puts $x}",
        "2\n",
        "set x $i",
    ),
    (
        "a for loop",
        "try {for {set i 0} {$i < 3} {incr i} {set x $i; foo $i}} on error {} {puts $x}",
        "2\n",
        "set x $i",
    ),
    (
        "an arm of a switch",
        "try {switch $c {1 {set x 2; foo; set x 3} default {set x 4}}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a nested try no handler of which takes the error",
        "try {try {set x 2; foo; set x 3} on break {} {}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a nested try with no handler",
        "try {try {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a handler of a nested try",
        "try {try {error a} on error {} {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "the clause of a nested try",
        "try {try {error a} finally {set x 2; foo; set x 3}} on error {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a trap",
        "try {set x 2; foo; set x 3} trap {} {} {puts $x}",
        "2\n",
        "set x 2",
    ),
    (
        "a handler a `-` handler hands its match to",
        "try {set x 2; foo; set x 3} on error {m} - on ok {} {puts $x}",
        "2\n",
        "set x 2",
    ),
];

/// A command of a `try` body may fail wherever it stands, at any depth, and a
/// handler or the `finally` clause is thrown to with what the body has stored
/// by then. Each statement of [`BETWEEN_TWO_STORES`] stores 2, calls `foo`,
/// which raises, and stores another value: the handler or clause prints 2
/// under tclsh 8.6 to 9.1, before and after the optimiser, and `set x 2` is
/// no dead store. The optimiser had deleted it and forwarded the store after
/// the failure: `try {set x 2; foo; set x 3} finally {puts $x}` became `puts
/// 3`, and `set x 1; try {set x 2; foo; set x 1} on error {} {}; puts $x`
/// printed `1`, since the handler and the clause were thrown to from the
/// state before the body and at its end alone.
#[test]
fn a_try_body_is_thrown_to_from_between_its_stores() {
    for (why, statement, printed, kept) in BETWEEN_TWO_STORES {
        let source = format!(
            "proc foo {{args}} {{if {{$args eq {{}} || [lindex $args 0] == 2}} {{error x}}}}\nproc p {{c}} {{\n    set x 1\n    {statement}\n}}\ncatch {{p 1}}\n"
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                rewritten.contains(kept),
                "{why}: {dialect}: `{kept}`, the store a failure leaves, stays\n{rewritten}"
            );
        }
        prints_under_releases_from(&source, printed, "8.6");
    }
    let issue = "proc foo {} {error x}\nproc p {} {\n    set x 1\n    try {set x 2; foo; set x 1} on error {} {}\n    puts $x\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let held = last_value(issue, dialect, "::p", "x");
        assert!(
            !matches!(held, LatticeValue::Const(ConstValue::Int(1))),
            "{dialect}: the handler may see 2, not a constant 1: {held:?}"
        );
    }
    prints_under_releases_from(issue, "2\n", "8.6");
}

/// The blocks a `try` body is split into keep what the handlers and the clause
/// were proved of it as one block. A statement that completes with its code
/// from the state before it stays with the statement before it, and the first
/// statements of the body read as one run: `set z 0; error boom` and `set a 1;
/// set b 2; error boom` are an error an `on error` handler takes whole, so the
/// clause around a nested `try` sees the name that handler binds (no W210).
/// The clause is thrown to the state before a statement only where that
/// failure escapes the handlers: a literal assignment raises an error alone,
/// which an `on error` handler takes, and which an `on break` handler does not
/// see at all, so a `break` it takes is still forwarded the value stored
/// before it. And a statement that leaves after its own
/// stores is thrown from the state it leaves, where the next command raises
/// only after a store: `upfoo` writes `x` and raises before `lassign` runs, so
/// the handler sees `a` still `old`, and where a `catch` follows it, which
/// runs its own script first, so that the `on break` handler sees what a
/// `return -code break` left. Each program prints what tclsh 8.6 to 9.1
/// print, before and after the optimiser.
#[test]
fn a_split_try_body_keeps_what_its_handlers_were_proved() {
    let programs = [
        (
            "proc p {} {\n    try { try {set z 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try { try {set a 1; set b 2; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try { try {set {[} 0; error boom} on error {} {set x 1; return} finally {} } finally {puts $x}\n}\np\n",
            "1\n",
        ),
        (
            "proc p {} {\n    try {set z 0; set y 1} on error {} {set y 2} finally {puts $y}\n}\np\n",
            "1\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: every path into the read binds it\n{source}"
            );
        }
        prints_under_releases_from(source, printed, "8.6");
    }
    // A literal assignment raises an error if it fails at all, which an `on
    // break` handler does not take: the handler sees `y` at 2 alone, and the
    // optimiser forwards it, as it did before the body was split.
    let breaks = "proc p {} {\n    foreach i {1} {\n        try {set y 1; set y 2; break} on break {} {puts $y}\n    }\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(breaks, dialect);
        assert!(
            rewritten.contains("puts 2"),
            "{dialect}: the handler sees `y` at 2\n{rewritten}"
        );
    }
    prints_under_releases_from(breaks, "2\n", "8.6");
    let after_stores = "proc upfoo {} {upvar x x; set x 5; error boom}\nproc p {} {\n    array set b {k v}\n    set a old\n    try {upfoo; lassign {new second} a b} on error {} {puts $a}\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(after_stores, dialect);
        assert!(
            rewritten.contains("puts $a"),
            "{dialect}: `a` is old or new where the handler runs\n{rewritten}"
        );
    }
    prints_under_releases_from(after_stores, "old\n", "8.6");

    // `upfoo` writes `x` and breaks; the `catch` after it runs its own script
    // first, so only the statement's own stores make its block a point, and
    // the `on break` handler sees `x` at 5, not the 1 before and after it.
    let breaks_after_stores = "proc upfoo {} {upvar x x; set x 5; return -code break}\nproc p {} {\n    set x 1\n    try {upfoo; catch {set y 1}; set x 1} on break {} {puts $x}\n}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let (rewritten, _) = optimised(breaks_after_stores, dialect);
        assert!(
            rewritten.contains("puts $x"),
            "{dialect}: `x` is 1 or 5 where the handler runs\n{rewritten}"
        );
        let held = last_value(breaks_after_stores, dialect, "::p", "x");
        assert!(
            !matches!(held, LatticeValue::Const(ConstValue::Int(1))),
            "{dialect}: the handler may see 5, not a constant 1: {held:?}"
        );
    }
    prints_under_releases_from(breaks_after_stores, "5\n", "8.6");
}

/// The blocks of `proc`'s `puts` statements, in source order.
fn puts_blocks(unit: &CompilationUnit, proc: &str) -> Vec<tcl_compiler::cfg::BlockId> {
    let function = unit.procedures.get(proc).expect("the procedure");
    let mut found: Vec<(u32, tcl_compiler::cfg::BlockId)> = function
        .cfg
        .blocks
        .iter()
        .flat_map(|(&id, block)| {
            block
                .statements
                .iter()
                .filter(|statement| {
                    matches!(statement, Statement::Call { command, .. } if command == "puts")
                })
                .map(move |statement| (statement.span().start(), id))
        })
        .collect();
    found.sort_unstable();
    found.into_iter().map(|(_, id)| id).collect()
}

/// The value `var`'s version `version` holds at `block` of `proc`: the
/// block-qualified lookup, which a refinement in force there narrows.
fn value_in(
    unit: &CompilationUnit,
    proc: &str,
    block: tcl_compiler::cfg::BlockId,
    var: &str,
    version: u32,
) -> Option<LatticeValue> {
    let function = unit.procedures.get(proc).expect("the procedure");
    let symbol = function.ssa.var_symbol(var).expect("the variable");
    function.sccp.value_at(block, (symbol, version)).cloned()
}

/// A refinement never makes a version, and a merge keeps only what every
/// path into it carries: the arms of `if {$x eq "a"} … elseif {$x eq "b"}`
/// hold `x` at `a` and at `b`, and where they meet — the `else` arm returns,
/// so only the two refined arms reach the `puts` past them — `x` reads as
/// its own value, never as either refinement or a set of the two. tclsh 8.4
/// to 9.1 print `a 1`, `a`, `b 2` and `b` for `p a; p b; p c`, before and
/// after `tcl opt`.
#[test]
fn a_merge_drops_the_refinement() {
    let source = "proc p {x} {\n    if {$x eq \"a\"} {\n        set r 1\n    } elseif {$x eq \"b\"} {\n        set r 2\n    } else {\n        return\n    }\n    puts \"$x $r\"\n    puts $x\n}\np a\np b\np c\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        let refined: Vec<String> = function
            .sccp
            .refinements
            .iter()
            .filter_map(|refinement| match &refinement.fact {
                tcl_registry::value_transfer::FactView::Exact(value, _) => {
                    Some(String::from_utf8_lossy(&value.bytes).into_owned())
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            refined,
            ["a", "b"],
            "{dialect}: each arm's edge refines `x`"
        );
        let blocks: Vec<_> = function
            .cfg
            .blocks
            .iter()
            .filter_map(|(&id, block)| {
                block
                    .statements
                    .iter()
                    .find_map(|statement| match statement {
                        Statement::AssignConst { name, value, .. } if name == "r" => {
                            Some((value.clone(), id))
                        }
                        _ => None,
                    })
            })
            .collect();
        for (value, block) in blocks {
            let expected = if value == "1" { "a" } else { "b" };
            assert_eq!(
                value_in(&unit, "::p", block, "x", 0),
                Some(text(expected)),
                "{dialect}: the arm that sets `r` to {value} holds `x` at {expected}"
            );
        }
        for block in puts_blocks(&unit, "::p") {
            assert_eq!(
                value_in(&unit, "::p", block, "x", 0),
                value_at(&unit, "::p", "x", 0),
                "{dialect}: past the merge `x` is its own value"
            );
            assert_eq!(function.sccp.refinements_in(block).count(), 0, "{dialect}");
        }
    }
    prints_under_every_release(source, "a 1\na\nb 2\nb\n");
}

/// A traced variable is never refined: a read trace may write the variable
/// as the read runs, so the `$x` the arm prints need not be the `a` the
/// condition read. `tr` leaves the first read alone and rewrites `x` on the
/// second, and tclsh 8.4 to 9.1 print `changed`, before and after `tcl opt`.
/// A trace another procedure installs on a name makes it externally mutable
/// in every function, so `q`'s `y` is never refined either.
#[test]
fn a_traced_variable_is_never_refined() {
    let source = "set n 0\nproc tr {name1 name2 op} {upvar 1 $name1 v; incr ::n; if {$::n > 1} {set v changed}}\nproc p {x} {\n    trace add variable x read tr\n    if {$x eq \"a\"} {puts $x}\n}\nproc t {} {trace add variable y write tr}\nproc q {y} {\n    if {$y eq \"a\"} {puts $y}\n}\np a\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        for proc in ["::p", "::q"] {
            let function = unit.procedures.get(proc).expect("the procedure");
            assert!(
                function.sccp.refinements.is_empty(),
                "{dialect} {proc}: {:?}",
                function.sccp.refinements
            );
            assert!(function.sccp.value_entries.is_empty(), "{dialect} {proc}");
        }
    }
    prints_under_every_release(source, "changed\n");
}

/// The nested equality decides: inside `if {$x eq "a"}`'s arm `x` is `a`,
/// so `if {$x eq "b"}` there is false — a constant branch the solver applies,
/// its true block outside the executable blocks — under every dialect, and
/// the optimiser drops `puts never`. The program prints `inner` for `a` and
/// nothing for `b` under tclsh 8.4 to 9.1, before and after `tcl opt`.
#[test]
fn the_nested_equality_decides() {
    let source = "proc p {x} {\n    if {$x eq \"a\"} {\n        if {$x eq \"b\"} {puts never} else {puts inner}\n    }\n}\nset ::v a\np $::v\nset ::v b\np $::v\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        let decided = function
            .sccp
            .constant_branches
            .iter()
            .find(|branch| branch.condition == "$x eq \"b\"")
            .unwrap_or_else(|| panic!("{dialect}: {:?}", function.sccp.constant_branches));
        assert!(!decided.value, "{dialect}");
        let dead = function
            .cfg
            .block_id(&decided.not_taken_target)
            .expect("the arm");
        assert!(
            !function.sccp.executable_blocks.contains(&dead),
            "{dialect}: the inner true block never runs"
        );
        let (rewritten, _) = optimised(source, dialect);
        assert!(
            !rewritten.contains("puts never"),
            "{dialect}: the optimiser drops the dead arm\n{rewritten}"
        );
    }
    prints_under_every_release(source, "inner\n");
}

/// The page's twelve programs (`docs/design/compiler/value-transfers.md`
/// § *Predicate refinement*), each through a procedure whose argument no
/// call site pins — a global the analysis holds no value for — so the
/// branch itself is the only evidence: a numeric `==` keeps the string
/// (`1.0`, ` 1`, `01`, and `08` from 9.0), `eq` compares spellings, a
/// `string is` test proves a type and never a value (` 12 `, `{}` with and
/// without `-strict`, `0x10`), truth is no value (`yes`), and from 8.5 an
/// opaque `-nocase` `switch` and `in` keep what they were given. Each prints
/// the same under every release before and after `tcl opt`.
///
/// A read of the refined variable itself is never rewritten, so the `==`
/// arms, an `in` arm and a flattened `switch`'s arms also read the arm's
/// value through a definition (`set y $x; puts $y`), which O100 inlines
/// where the arm proves an exact value: an exact value where the table
/// says "never the string" prints `1` for `1.0`, and one member of an `in`
/// set prints that member for another.
#[test]
fn the_twelve_refinement_witnesses() {
    let every = "proc p {x} {if {$x == 1} {set y $x; puts $y; return [string length $x]}; return none}\n\
                 proc q {x} {if {$x == 1} {set y $x; puts $y; return $x}; return none}\n\
                 proc r {x} {if {$x eq \"1\"} {return yes}; return no}\n\
                 proc s {x} {if {$x eq \"a\"} {return $x}; return none}\n\
                 proc t {x} {if {[string is integer -strict $x]} {return [string length $x]}; return none}\n\
                 proc u {x} {if {[string is integer $x]} {return \"yes [string length $x]\"}; return no}\n\
                 proc u2 {x} {if {[string is integer -strict $x]} {return yes}; return no}\n\
                 proc v {x} {if {[string is integer -strict $x]} {return $x}; return none}\n\
                 proc w {x} {if {$x} {return $x}; return none}\n\
                 proc k {x} {switch -- $x {a {set y $x; puts $y; return A} b {set y $x; puts $y; return B}}; return none}\n\
                 set ::v 1.0; puts [p $::v]\n\
                 set ::v \" 1\"; puts [p $::v]\n\
                 set ::v 01; puts [q $::v]\n\
                 set ::v 1.0; puts [r $::v]\n\
                 set ::v a; puts [s $::v]\n\
                 set ::v \" 12 \"; puts [t $::v]\n\
                 set ::v {}; puts [u $::v]; puts [u2 $::v]\n\
                 set ::v 0x10; puts [v $::v]\n\
                 set ::v yes; puts [w $::v]\n\
                 set ::v b; puts [k $::v]\n";
    prints_under_every_release(
        every,
        "1.0\n3\n 1\n2\n01\n01\nno\na\n4\nyes 0\nno\n0x10\nyes\nb\nB\n",
    );
    let leading_zero = "proc y {x} {if {$x == 8} {set y $x; puts $y; return \"yes [string length $x]\"}; return no}\n\
                        set ::v 08; puts [y $::v]\n";
    for (series, tclsh) in releases_on_path() {
        let expected = if series < "9.0" {
            "no\n"
        } else {
            "08\nyes 2\n"
        };
        let (rewritten, _) = optimised(leading_zero, &dialect_of(series));
        for program in [leading_zero, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((true, expected.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
    let from_85 = "proc z {x} {switch -nocase -- $x {a {return $x}}; return none}\n\
                   proc m {x} {if {$x in {a b c}} {set y $x; puts $y; return $x}; return none}\n\
                   set ::v A; puts [z $::v]\n\
                   set ::v b; puts [m $::v]\n";
    prints_under_releases_from(from_85, "A\nb\nb\n", "8.5");
}

/// The lines, from 1, of the S100 shimmer warnings in `source` under
/// `dialect`, in order.
fn shimmer_lines(source: &str, dialect: &str) -> Vec<usize> {
    let registry = static_context_for(dialect).commands();
    let unit = CompilationUnit::build_for_dialect(source, registry, false, dialect);
    let mut lines: Vec<usize> =
        tcl_compiler::shimmer::find_shimmer_warnings_for_cu(&unit, registry)
            .iter()
            .filter(|warning| warning.code == DiagCode::S100)
            .map(|warning| {
                let start = usize::try_from(warning.span.start())
                    .map_or(source.len(), |start| start.min(source.len()));
                source[..start].matches('\n').count() + 1
            })
            .collect();
    lines.sort_unstable();
    lines
}

/// A `string is` test types its arm: once `string is integer -strict $i`
/// holds, `i` has an integer representation (`tcl::unsupported::representation`
/// reads `int`, tclsh 8.6 to 9.1), so reading it as an index in the arm
/// converts nothing, while past the arm, where the test may have failed,
/// the read still converts a string — S100 on line 6 and not on line 4. The
/// program prints `b` twice under every release, before and after `tcl opt`.
#[test]
fn a_string_is_test_types_its_arm() {
    let source = "proc p {s} {\n    set i [string trim $s]\n    if {[string is integer -strict $i]} {\n        puts [lindex {a b c} $i]\n    }\n    puts [lindex {a b c} $i]\n}\nset ::v 1\np $::v\n";
    for dialect in DIALECTS {
        assert_eq!(shimmer_lines(source, dialect), vec![6], "{dialect}");
    }
    prints_under_every_release(source, "b\nb\n");
}

/// A definition in a refined arm takes the arm's value: `set y $x` inside
/// `if {$x eq "a"}` makes `y` the constant `a`, so O100 inlines it where it
/// is read (`puts $y` becomes `puts a`), while a read of `x` itself keeps the
/// version's own value and stays as written. The program prints `a` twice
/// for `a` and nothing for `b` under tclsh 8.4 to 9.1, before and after
/// `tcl opt`.
#[test]
fn a_definition_in_a_refined_arm_takes_the_arms_value() {
    let source = "proc p {x} {\n    if {$x eq \"a\"} {\n        set y $x\n        puts $y\n        puts $x\n    }\n}\nset ::v a\np $::v\nset ::v b\np $::v\n";
    for dialect in DIALECTS {
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        let y = function.ssa.var_symbol("y").expect("y");
        assert_eq!(
            function.sccp.values.get(&(y, 1)),
            Some(&text("a")),
            "{dialect}"
        );
        let (rewritten, rewrites) = optimised(source, dialect);
        assert!(rewritten.contains("puts a\n"), "{dialect}:\n{rewritten}");
        assert!(rewritten.contains("puts $x"), "{dialect}:\n{rewritten}");
        assert!(
            rewrites
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O100),
            "{dialect}: {rewrites:?}"
        );
    }
    prints_under_every_release(source, "a\na\n");
}

/// A range from a comparison holds of an integer operand, and the bounds
/// checks take one only for an index proved an integer: `end` passes `$i > 5`
/// as a string and indexes `c` (#2368's program, its `gets stdin` a
/// parameter here), and `7.0` fails `$i != 7` and is no index, `lindex`
/// raising `bad index "7.0"` (the review's program), so W230 reports
/// neither, under every dialect. Proved an integer by `string is integer
/// -strict`, `9` past `$i > 5` is past the end: W230 reports it, and tclsh
/// prints `<>`.
#[test]
fn a_range_from_a_comparison_narrows_only_a_proved_integer() {
    let end = "proc q {s} {\n    set i [string trim $s]\n    if {$i > 5} {\n        set x [lindex {a b c} $i]\n        puts $x\n    }\n}\nq end\n";
    let point = "proc g {s} {\n    set i [string trim $s]\n    if {$i != 7} { puts no } else { set x [lindex {a b c} $i]; puts $x }\n}\ng 7.0\n";
    let proved = "proc h {s} {\n    set i [string trim $s]\n    if {[string is integer -strict $i] && $i > 5} {\n        set x [lindex {a b c} $i]\n        puts <$x>\n    }\n}\nh 9\n";
    for dialect in DIALECTS {
        assert!(!reports(end, dialect, DiagCode::W230), "{dialect}: end");
        assert!(!reports(point, dialect, DiagCode::W230), "{dialect}: 7.0");
        assert!(reports(proved, dialect, DiagCode::W230), "{dialect}: 9");
    }
    prints_under_every_release(end, "c\n");
    prints_under_every_release(proved, "<>\n");
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(point, &dialect_of(series));
        for program in [point, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, String::new())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// The script an opaque `catch` keeps inside itself reads a name it writes
/// as its own, by the rule an opaque `switch`'s arms are read by: its reads
/// less its writes are what it reads of the frame. A body that sets a name
/// and then reads it — a `set`, a `foreach` binder, a `for` counter in a
/// procedure — draws no W210 under every dialect, where the marker ahead of
/// the call had made each read one of a name that may be unset; tclsh 8.4
/// to 9.1 print what the bodies print, before and after `tcl opt`. A name the
/// body only may set is still undefined after the call on the path where it
/// did not: `puts $x` past `catch {if {$c} {set x 1}}` draws W210, and tclsh
/// raises on `p 0`.
#[test]
fn an_opaque_catch_reads_its_own_writes_as_its_own() {
    for (source, printed) in [
        ("catch {set i 0; puts $i}\n", "0\n"),
        ("catch {foreach x {1 2} {puts $x}}\n", "1\n2\n"),
        (
            "proc p {} {catch {for {set i 0} {$i < 3} {incr i} {puts $i}}}\np\n",
            "0\n1\n2\n",
        ),
    ] {
        for dialect in DIALECTS {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}: {source}"
            );
        }
        prints_under_every_release(source, printed);
    }
    let may_set = "proc p {c} {catch {if {$c} {set x 1}}; puts $x}\np 1\np 0\n";
    for dialect in DIALECTS {
        assert!(reports(may_set, dialect, DiagCode::W210), "{dialect}");
    }
    for (series, tclsh) in releases_on_path() {
        let (rewritten, _) = optimised(may_set, &dialect_of(series));
        for program in [may_set, rewritten.as_str()] {
            assert_eq!(
                run_script(&tclsh, program),
                Some((false, "1\n".to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// A loop that runs past the enumeration cap
/// (`DEFAULT_MAX_STATIC_LOOP_ITERS` passes) declines: the solver publishes
/// nothing on its exit, so the branch after it decides nothing and the
/// widened lattice stands. A loop that runs to the cap is enumerated, and
/// its branch decides. tclsh 8.4 to 9.1 print `yes` for both, before and
/// after `tcl opt`.
#[test]
fn the_iteration_cap_publishes_nothing() {
    let cap = tcl_compiler::static_loops::DEFAULT_MAX_STATIC_LOOP_ITERS;
    for (bound, enumerated) in [(cap, true), (cap + 1, false)] {
        let source = format!(
            "proc p {{}} {{\n    for {{set i 0}} {{$i < {bound}}} {{incr i}} {{}}\n    \
             if {{$i == {bound}}} {{puts yes}} else {{puts no}}\n}}\np\n"
        );
        let condition = format!("$i == {bound}");
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            let function = unit.procedures.get("::p").expect("the procedure");
            let decided = function
                .sccp
                .constant_branches
                .iter()
                .find(|branch| branch.condition == condition)
                .map(|branch| branch.value);
            assert_eq!(decided, enumerated.then_some(true), "{dialect}: {bound}");
            assert_eq!(
                function.sccp.loop_enumerations.len(),
                usize::from(enumerated),
                "{dialect}: {bound}"
            );
        }
        prints_under_every_release(&source, "yes\n");
    }
}

/// The enumeration stores only to a place whose kind it proves — one it
/// holds a scalar of, or holds unbound — and names an element as the word
/// gives it: `foreach b` over an array `b` raises on its first binding, and
/// `${a(k)}` and `set a(k) …` raise on a scalar `a`, so a `catch` body that
/// does any of them leaves its counter where the error left it. Had the run
/// bound the array as a scalar, or read and written `a(k)` as `a`, `$n == 0`
/// would decide false and `i` would leave the `catch` at 3. A binder takes
/// the same rule as every other store, so `foreach a(k) {1 2} {}` is not
/// run, as `set a(k) …` is not. tclsh 8.4 to 9.1 print `zero`, `0` twice and
/// `two`, before and after `tcl opt`.
#[test]
fn an_enumeration_runs_only_over_places_it_proves() {
    let array_binder = "proc p {} {\n    array set b {k keep}\n    set n 0\n    \
                        catch {foreach b {1 2} {incr n}}\n    \
                        if {$n == 0} {puts zero} else {puts other}\n}\np\n";
    let element = |command: &str| {
        format!(
            "proc p {{}} {{\n    set a 5\n    set i 9\n    \
             catch {{for {{set i 0}} {{$i < 3}} {{incr i}} {{set y $a; {command}}}}}\n    \
             puts $i\n}}\np\n"
        )
    };
    let elements = [element("set x ${a(k)}"), element("set a(k) $i")];
    let element_binder = "proc p {} {\n    set a(k) 0\n    foreach a(k) {1 2} {}\n    \
                          if {$a(k) == 2} {puts two} else {puts other}\n}\np\n";
    for dialect in DIALECTS {
        let binder_unit = unit_of(element_binder, dialect);
        assert!(
            binder_unit
                .procedures
                .get("::p")
                .expect("the procedure")
                .sccp
                .loop_enumerations
                .is_empty(),
            "{dialect}"
        );
        let unit = unit_of(array_binder, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        assert!(
            function
                .sccp
                .constant_branches
                .iter()
                .all(|branch| branch.condition != "$n == 0"),
            "{dialect}: {:?}",
            function.sccp.constant_branches
        );
        for source in &elements {
            assert!(
                !matches!(
                    last_value(source, dialect, "::p", "i"),
                    LatticeValue::Const(_)
                ),
                "{dialect}\n{source}"
            );
        }
    }
    prints_under_every_release(array_binder, "zero\n");
    for source in &elements {
        prints_under_every_release(source, "0\n");
    }
    prints_under_every_release(element_binder, "two\n");
}

/// A name a loop rebinds that is dead after it — a temporary set before the
/// loop and in its body, read nowhere after — has no φ where the loop leaves,
/// so the version live there is the one from before the loop, which the
/// loop's state does not describe, and the solver states nothing of it. It
/// had stated the loop's last value of the name for that version, which the
/// settled run contradicted, and the contradiction dropped every loop's state
/// in the unit: the loop's own counter, and a later loop's that has nothing
/// to do with the name. A contradiction now drops only the loop whose state
/// it contradicts. The branch after each loop decides, for I230 as for `tcl
/// opt`. tclsh 8.4 to 9.1 print `yes`, `yes` and `three`, `yes`, and `yes`,
/// before and after `tcl opt`.
#[test]
fn a_name_dead_after_its_loop_keeps_every_loops_state() {
    let top = "set tmp 1\nset n 0\nforeach x {1 2} { set tmp $x; incr n }\n\
               if {$n == 2} {puts yes} else {puts no}\n";
    let later = format!(
        "{top}for {{set i 0}} {{$i < 3}} {{incr i}} {{}}\n\
         if {{$i == 3}} {{puts three}} else {{puts other}}\n"
    );
    let dead_while = "set tmp 1\nset n 0\nwhile {$n < 2} { set tmp $n; incr n }\n\
                      if {$n == 2} {puts yes} else {puts no}\n";
    let idiom = "proc p {} {\n    set found 0\n    set last \"\"\n    \
                 foreach x {a b c} { set last $x; if {$x eq \"b\"} { set found 1 } }\n    \
                 if {$found} { return yes }\n    return no\n}\nputs [p]\n";
    for dialect in DIALECTS {
        for (source, claims) in [
            (top, vec![true]),
            (later.as_str(), vec![true, true]),
            (dead_while, vec![true]),
            (idiom, vec![true]),
        ] {
            assert_eq!(
                condition_claims(source, dialect),
                claims,
                "{dialect}\n{source}"
            );
        }
        assert_eq!(
            unit_of(&later, dialect)
                .top_level
                .sccp
                .loop_enumerations
                .len(),
            2,
            "{dialect}"
        );
    }
    prints_under_every_release(top, "yes\n");
    prints_under_every_release(&later, "yes\nthree\n");
    prints_under_every_release(dead_while, "yes\n");
    prints_under_every_release(idiom, "yes\n");
}

/// The state a `foreach` or a `while` leaves decides the branch after it,
/// for I230 as for `tcl opt`: the counter the solver keeps widened inside
/// each loop is 3, then -2, on the loop's exit edges. The `foreach` leaves by
/// its latch, whose test the analysis cannot read, so the solver reaches the
/// block after it while the counter's own value is still a transient
/// constant the loop's state rules out; that value decides nothing, and the
/// arm the state rules out is never reached. tclsh 8.4 to 9.1 print `three`
/// and `minus`, before and after `tcl opt`.
#[test]
fn the_state_a_loop_leaves_decides_the_branch_after_it() {
    let source = "proc p {} {\n    set n 0\n    foreach x {a b c} { incr n }\n    \
                  if {$n == 3} { puts three } else { puts other }\n    set i 10\n    \
                  while {$i > 0} { incr i -3 }\n    \
                  if {$i == -2} { puts minus } else { puts other }\n}\np\n";
    for dialect in DIALECTS {
        assert_eq!(condition_claims(source, dialect), [true, true], "{dialect}");
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        assert_eq!(
            function
                .sccp
                .loop_enumerations
                .iter()
                .map(|record| record.iterations)
                .collect::<Vec<_>>(),
            [3, 4],
            "{dialect}"
        );
    }
    prints_under_every_release(source, "three\nminus\n");
}

/// A loop's own test is the branch that leaves the loop when false, wherever
/// its block sits: `while 1` decided true draws no I230, and an `if` in a
/// loop's body is an `if` like any other, so one decided true there draws
/// one. tclsh 8.4 to 9.1 print `3` and `done`, before and after `tcl opt`.
#[test]
fn a_loops_own_test_is_the_branch_that_leaves_it() {
    let source = "proc p {} {\n    set n 0\n    for {set i 0} {$i < 3} {incr i} {\n        \
                  set k 1\n        if {$k == 1} { incr n }\n    }\n    puts $n\n}\n\
                  proc q {} {\n    while 1 { return done }\n}\np\nputs [q]\n";
    for dialect in DIALECTS {
        assert_eq!(condition_claims(source, dialect), [true], "{dialect}");
    }
    prints_under_every_release(source, "3\ndone\n");
}

/// A store before an opaque `switch` arm or an opaque `catch` body that may
/// write an element of the array it names is read by the statement: where
/// no arm runs, or the body raises before the element write lands, the base
/// holds what it held, so `set a 5` stays live, W220 does not report it, and
/// O109 does not delete it — which made `puts $a` raise `can't read "a": no
/// such variable` where tclsh prints `5`. The read is quoted, and
/// read-before-set reads the base as the definition it was, so a base never
/// set before draws no W210. tclsh 8.4 to 9.1 print `5` twice, `5`, `1`, and
/// `1` and `0`, before and after `tcl opt`.
#[test]
fn a_may_written_element_reads_the_store_to_its_array() {
    let switch_arm = "proc p {x} {\n    set a 5\n    switch -glob -- $x { x* {set a(k) 1} }\n    \
                      puts $a\n}\np y\np z\n";
    let catch_body = "set a 5\ncatch {set a(k) 1}\nputs $a\n";
    let never_set_catch = "catch {set a(k) 1}\nputs [array size a]\n";
    let never_set_switch = "proc p {x} {\n    switch -glob -- $x { x* {set a(k) 1} }\n    \
                            puts [array size a]\n}\np x\np y\n";
    for dialect in DIALECTS {
        for source in [switch_arm, catch_body] {
            assert!(
                !reports(source, dialect, DiagCode::W220),
                "{dialect}\n{source}"
            );
        }
        for source in [never_set_catch, never_set_switch] {
            assert!(
                !reports(source, dialect, DiagCode::W210),
                "{dialect}\n{source}"
            );
        }
    }
    prints_under_every_release(switch_arm, "5\n5\n");
    prints_under_every_release(catch_body, "5\n");
    prints_under_every_release(never_set_catch, "1\n");
    prints_under_every_release(never_set_switch, "1\n0\n");
}

/// A loop's body leaves the loop, or writes its counter, through a script
/// word however it is written: `if` runs a bare or a quoted body as it runs a
/// braced one, so `if {$i < 0} break` is an exit and `if {$i < -5} "set i
/// 20"` a write of the counter, and none of these loops is provably infinite
/// (#2381) — the first was W241, "counter $i starts at 5, moves by -1 per
/// step, and compares < 10 (never reached)". tclsh 8.4 to 9.1 print `-1`,
/// `-1`, `-6` and `19`, before and after `tcl opt`.
#[test]
fn a_bare_or_quoted_body_word_leaves_the_loop() {
    let programs = [
        (
            "for {set i 5} {$i < 10} {incr i -1} {if {$i < 0} break}\nputs $i\n",
            "-1\n",
        ),
        (
            "set i 5\nwhile {$i < 10} {if {$i < 0} break; incr i -1}\nputs $i\n",
            "-1\n",
        ),
        (
            "for {set i 0} {$i < 10} {incr i -1} {if {$i < -5} \"break\"}\nputs $i\n",
            "-6\n",
        ),
        (
            "for {set i 0} {$i < 10} {incr i -1} {if {$i < -5} \"set i 20\"}\nputs $i\n",
            "19\n",
        ),
    ];
    for (source, printed) in programs {
        for dialect in DIALECTS {
            assert!(
                !reports(source, dialect, DiagCode::W241),
                "{dialect}\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
}

/// W241's counter path reads what may write the counter from the unit that
/// holds the loop: every statement of the loop's blocks that defines it, and
/// every call there to code the module cannot see. A write a `[…]` word
/// makes, a binder, a procedure's `uplevel` and the `upvar` helper idiom each
/// define the counter, and a call to a procedure another file defines — here
/// one the program sources — is one the module cannot see, at the top level
/// and in a procedure; each `while` was W241, "while loop is provably
/// infinite: counter $i starts at 5, moves by -1 per step, and compares < 10
/// (never reached)", and so was the `for` with a literal start, before the
/// slice. tclsh 8.4 to 9.1 print each program's number, before and after
/// `tcl opt`.
#[test]
fn whatever_writes_the_counter_keeps_w241_silent() {
    let helper = std::env::temp_dir().join(format!(
        "value-transfer-witness-helper-{}.tcl",
        std::process::id()
    ));
    std::fs::write(&helper, "proc foo {name} {upvar 1 $name v; set v 100}\n")
        .expect("the helper file");
    let source_helper = format!("source {{{}}}\n", helper.display());
    let sourced = [
        format!("{source_helper}set i 5\nwhile {{$i < 10}} {{incr i -1; foo i}}\nputs $i\n"),
        format!(
            "{source_helper}proc s {{}} {{\n    set i 5\n    \
             while {{$i < 10}} {{incr i -1; foo i}}\n    return $i\n}}\nputs [s]\n"
        ),
    ];
    let mut programs: Vec<(&str, &str)> = vec![
        (
            "set i 5\nwhile {$i < 10} {incr i -1; set j [incr i 20]}\nputs $i\n",
            "24\n",
        ),
        (
            "set i 5\nwhile {$i < 10} {incr i -1; foreach i {100} {}}\nputs $i\n",
            "100\n",
        ),
        (
            "proc q {} {uplevel 1 {set i 100}}\nset i 5\n\
             while {$i < 10} {incr i -1; q}\nputs $i\n",
            "100\n",
        ),
        (
            "proc foo {name} {upvar 1 $name v; set v 100}\nset i 5\n\
             while {$i < 10} {incr i -1; foo i}\nputs $i\n",
            "100\n",
        ),
        (
            "for {set i 5} {$i < 10} {incr i -1} {if {$i < 0} {set j [incr i 20]}}\nputs $i\n",
            "18\n",
        ),
    ];
    programs.extend(sourced.iter().map(|source| (source.as_str(), "100\n")));
    for (source, printed) in programs {
        for dialect in DIALECTS {
            assert!(
                !reports(source, dialect, DiagCode::W241),
                "{dialect}\n{source}"
            );
        }
        prints_under_every_release(source, printed);
    }
    let _ = std::fs::remove_file(&helper);
}

/// A loop statement's literal word is its value as Tcl substitutes it: a bare
/// or quoted word's escapes are decoded, so `lappend r a\x41` appends `aA`,
/// `append r \x41` appends `A`, and an `incr` by `\x31` adds 1. The
/// enumeration had handed each word on as written, so it left `r` holding
/// `a\x41 a\x41` and `tcl opt` folded the test after the loop to its false
/// arm. Each loop now leaves what Tcl leaves and the test decides true, for
/// I230 as for `tcl opt`. tclsh 8.4 to 9.1 print `yes` for each, before and
/// after `tcl opt`.
#[test]
fn a_loop_word_is_its_value_with_its_escapes_decoded() {
    for source in [
        "set r {}\nforeach x {1 2} { lappend r a\\x41 }\n\
         if {$r eq \"aA aA\"} {puts yes} else {puts no}\n",
        "set r {}\nfor {set i 0} {$i < 2} {incr i} { append r \\x41 }\n\
         if {$r eq \"AA\"} {puts yes} else {puts no}\n",
        "set r {}\nforeach x {1 2} { lappend r \"a\\x41\" }\n\
         if {$r eq \"aA aA\"} {puts yes} else {puts no}\n",
        "for {set i 0} {$i < 2} {incr i \\x31} {}\n\
         if {$i == 2} {puts yes} else {puts no}\n",
    ] {
        for dialect in DIALECTS {
            assert_eq!(
                condition_claims(source, dialect),
                [true],
                "{dialect}\n{source}"
            );
        }
        prints_under_every_release(source, "yes\n");
    }
}

/// The eleven loop programs of the interface page (§ *Bounded-loop
/// enumeration*), one each at the top level: the solver runs each loop to
/// its exit over exact state — a `for` to its false condition, after a
/// `break`, past a `continue`, against a fractional bound, from an empty
/// start script and with its counter written in its body; a `for` the error
/// path of an opaque `catch` leaves; a `foreach` over no elements, over a
/// list too short for its last pass, and to a `break` — and the branch after
/// it decides on what the loop leaves, for the lattice, for I230 and for
/// `tcl opt`, which keeps only the arm that runs: `i` 5; `t` 6; `i` 3 after
/// `break`; `t` 3 with `continue`; `i` 3 for `$i < 2.5`; `i` 4 for the
/// empty start; `i` 4 and `n` 2; `i` 2 on the error path; `x` unbound after
/// `foreach x {} {}`; `a` 3 and `b` empty; `n` 1 and `x` 2. tclsh 8.4 to
/// 9.1 print each program's answer before and after `tcl opt`.
#[test]
fn the_eleven_loop_witnesses() {
    let witnesses = [
        ("for {set i 0} {$i < 5} {incr i} {}\n", "$i == 5", "five"),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {incr t $i}\n",
            "$t == 6",
            "six",
        ),
        (
            "for {set i 0} {$i < 10} {incr i} {if {$i == 3} break}\n",
            "$i == 3",
            "three",
        ),
        (
            "set t 0; for {set i 0} {$i < 4} {incr i} {if {$i == 1} continue; incr t}\n",
            "$t == 3",
            "three",
        ),
        ("for {set i 0} {$i < 2.5} {incr i} {}\n", "$i == 3", "three"),
        (
            "set i 0; for {} {$i < 3} {} {incr i 2}\n",
            "$i == 4",
            "four",
        ),
        (
            "set n 0; for {set i 0} {$i < 3} {incr i} {set i [expr {$i + 1}]; incr n}\n",
            "$i == 4 && $n == 2",
            "yes",
        ),
        (
            "catch {for {set i 0} {$i < 5} {incr i} {if {$i == 2} {error x}}}\n",
            "$i == 2",
            "two",
        ),
        ("foreach x {} {}\n", "[info exists x]", ""),
        ("foreach {a b} {1 2 3} {}\n", "$a == 3 && $b eq \"\"", "yes"),
        (
            "set n 0; foreach x {1 2 3} {if {$x == 2} break; incr n}\n",
            "$n == 1 && $x == 2",
            "yes",
        ),
    ];
    for (program, condition, taken) in witnesses {
        // The ninth program's test is the one that holds false.
        let holds = !taken.is_empty();
        let (then, otherwise) = if holds {
            (taken, "other")
        } else {
            ("bound", "unbound")
        };
        let source =
            format!("{program}if {{{condition}}} {{puts {then}}} else {{puts {otherwise}}}\n");
        let printed = if holds { then } else { otherwise };
        let dropped = if holds { otherwise } else { then };
        for dialect in DIALECTS {
            let unit = unit_of(&source, dialect);
            assert!(
                unit.top_level
                    .sccp
                    .constant_branches
                    .iter()
                    .any(|branch| { branch.condition == condition && branch.value == holds }),
                "{dialect}: {source}{:?}",
                unit.top_level.sccp.constant_branches
            );
            assert_eq!(
                condition_claims(&source, dialect),
                [holds],
                "{dialect}: {source}"
            );
            let (rewritten, _) = optimised(&source, dialect);
            assert!(
                !rewritten.contains(&format!("puts {dropped}")),
                "{dialect}: {source}{rewritten}"
            );
        }
        prints_under_every_release(&source, &format!("{printed}\n"));
    }
}

/// The interface page's correlated pairs (§ *The correlated finite-set
/// limit*) as the page writes them, `incr x [expr {$b / $a}]`: the
/// enumeration runs the amount's `[expr …]` over its state, so each loop
/// leaves what ordered execution leaves — `x` 20 and `y` 25 — where pairing
/// the binders by position would answer 20 for both and their cartesian
/// product neither, and each branch after a loop decides true. Inside each
/// loop the lattice still proves no amount — the quotient pairs two distinct
/// finite inputs, which it declines as correlated — so each `incr` declines.
/// tclsh 8.4 to 9.1 print `twenty` and `twentyfive`, before and after `tcl
/// opt`.
#[test]
fn the_correlated_pairs_decide_by_enumeration() {
    let source = "proc p {} {\n\
                   set x 0\n\
                   foreach {a b} {1 10 2 20} { incr x [expr {$b / $a}] }\n\
                   if {$x == 20} { puts twenty } else { puts other }\n\
                   set y 0\n\
                   foreach {a b} {1 20 2 10} { incr y [expr {$b / $a}] }\n\
                   if {$y == 25} { puts twentyfive } else { puts other }\n\
                  }\n";
    for dialect in DIALECTS {
        assert_eq!(condition_claims(source, dialect), [true, true], "{dialect}");
        let unit = unit_of(source, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        let published: Vec<(String, LatticeValue)> = function
            .sccp
            .loop_enumerations
            .iter()
            .flat_map(|record| record.published.iter().cloned())
            .filter(|(name, _)| name == "x" || name == "y")
            .collect();
        assert_eq!(
            published,
            [
                ("x".to_owned(), LatticeValue::Const(ConstValue::Int(20))),
                ("y".to_owned(), LatticeValue::Const(ConstValue::Int(25))),
            ],
            "{dialect}"
        );
        assert_eq!(
            answers_for(&unit, "::p", "incr"),
            ["declined: not-exact", "declined: not-exact"],
            "{dialect}"
        );
    }
    prints_under_every_release(&format!("{source}p\n"), "twenty\ntwentyfive\n");
}

/// A loop condition's math function is the one the module binds: with `abs`
/// rebound by `proc ::tcl::mathfunc::abs`, `$i < abs(-3)` runs the loop to
/// 99 under 8.5 to 9.1, so the enumeration declines — the shared lattice
/// under the module's observed bindings, the rewrite under its whole-module
/// trust — rather than stop at 3, and the branch after the loop decides
/// nothing. tclsh 8.5 to 9.1 print `other` at the top level and in a
/// procedure, before and after `tcl opt`.
#[test]
fn a_loop_condition_reads_the_math_binding() {
    let top_level = "proc ::tcl::mathfunc::abs {x} {return 99}\n\
                     for {set i 0} {$i < abs(-3)} {incr i} {}\n\
                     if {$i == 3} {puts three} else {puts other}\n";
    let in_a_procedure = "proc p {} {\n    for {set i 0} {$i < abs(-3)} {incr i} {}\n    \
                          if {$i == 3} {puts three} else {puts other}\n}\n\
                          proc ::tcl::mathfunc::abs {x} {return 99}\np\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        for source in [top_level, in_a_procedure] {
            assert!(
                !reports(source, dialect, DiagCode::I230),
                "{dialect}\n{source}"
            );
        }
        let unit = unit_of(top_level, dialect);
        assert!(
            unit.top_level.sccp.loop_enumerations.is_empty(),
            "{dialect}"
        );
        let unit = unit_of(in_a_procedure, dialect);
        let function = unit.procedures.get("::p").expect("the procedure");
        assert!(function.sccp.loop_enumerations.is_empty(), "{dialect}");
    }
    prints_under_releases_from(top_level, "other\n", "8.5");
    prints_under_releases_from(in_a_procedure, "other\n", "8.5");
}

/// The argument-sensitive re-run that O103 folds a call from runs the
/// callee's loop under the call's seeds: `[f 3]` runs `f`'s `for` three times
/// and returns `t` at 6, which the loop's exit state holds where the return
/// reads it, so the call folds to 6. tclsh 8.4 to 9.1 print `6`, before and
/// after `tcl opt`.
#[test]
fn the_argument_sensitive_rerun_runs_the_callees_loop() {
    let source = "proc f {n} {\n    set t 0\n    for {set i 0} {$i < $n} {incr i} {incr t 2}\n    \
                  return $t\n}\nset r [f 3]\nputs $r\n";
    for dialect in DIALECTS {
        let (rewritten, applied) = optimised(source, dialect);
        assert!(
            applied
                .iter()
                .any(|rewrite| rewrite.code == DiagCode::O103 && rewrite.replacement == "6"),
            "{dialect}: {applied:?}"
        );
        assert!(!rewritten.contains("[f 3]"), "{dialect}:\n{rewritten}");
    }
    prints_under_every_release(source, "6\n");
}

/// The state an enumerated loop leaves bounds its counter past the loop
/// header's widening: after `for {set i 0} {$i < 5} {incr i} {}`, `i` is
/// exactly 5, so `lindex` of a three-element list at `$i` is past the end and
/// W230 says so, at the top level and in a procedure. tclsh 8.4 to 9.1 print
/// an empty line, before and after `tcl opt`.
#[test]
fn an_enumerated_loop_bounds_its_counter_after_it() {
    let top_level = "set l {a b c}\nfor {set i 0} {$i < 5} {incr i} {}\nputs [lindex $l $i]\n";
    let in_a_procedure = "proc p {} {\n    set l {a b c}\n    for {set i 0} {$i < 5} {incr i} {}\n    \
                          puts [lindex $l $i]\n}\np\n";
    for dialect in DIALECTS {
        for source in [top_level, in_a_procedure] {
            assert!(
                reports(source, dialect, DiagCode::W230),
                "{dialect}\n{source}"
            );
        }
    }
    prints_under_every_release(top_level, "\n");
    prints_under_every_release(in_a_procedure, "\n");
}
