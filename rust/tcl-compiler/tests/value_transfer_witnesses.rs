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
use tcl_compiler::optimiser::Optimisation;
use tcl_compiler::optimiser::manager::optimise_source_multipass;
use tcl_registry::model::ingress::{resolve_environment, static_context_for};

/// The releases the oracle runs, oldest first.
const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// The multipass optimiser's iteration ceiling for a witness program.
const PASSES: usize = 8;

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
