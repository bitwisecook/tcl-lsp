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

//! The value-transfer lane's CLI witnesses
//! (`docs/design/lanes/value-transfers.md` § *Plan for slices 2–13*, D9):
//! the `tcl explore` and `tcl opt` evidence the lane's exit criteria name,
//! driven through the built `tcl` binary rather than the library directly.
//! Separate from `rust/tcl-cli/tests/cli.rs`, which the diagnostic-policy
//! lane edits; every later slice extends this file instead.

use std::path::PathBuf;
use std::process::Command;

/// An `XDG_CONFIG_HOME` for one spawn that names no directory — unique to
/// the spawn and never created — so no `tcl-lsp/config.ini` can be found
/// under it: the global policy layer every verb resolves is empty, and a
/// developer's own `config.ini` cannot change what these witnesses see.
fn absent_config_home() -> PathBuf {
    static SPAWNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let spawn = SPAWNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "value-transfers-cli-tests-{}-{spawn}",
        std::process::id()
    ))
}

/// The built `tcl` binary, isolated from the machine's global configuration.
fn tcl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
    command.env("XDG_CONFIG_HOME", absent_config_home());
    command
}

/// Run the built `tcl` binary with `args`, returning captured stdout text.
fn run_tcl(args: &[&str]) -> String {
    let output = tcl()
        .args(args)
        .output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// VT3.9's route tally, read from `tcl explore --show sccp`: two fused
/// `expr` statements are two expression entries, one nesting a
/// direct-routed `string length`, so `direct 1 · expression 2 ·
/// implementation 0` — the exit evidence's own line
/// (`docs/design/lanes/value-transfers.md` § *Slice 3* › *Goal and exit*).
#[test]
fn explore_sccp_prints_the_route_tally() {
    let text = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set r [expr {\"x\"}]; set n [expr {[string length abcdef] * 2}]}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(text.contains("r#1 = const('x')"), "{text}");
    assert!(text.contains("n#1 = const(12)"), "{text}");
    assert!(
        text.contains("routes entered: direct 1 · expression 2 · implementation 0"),
        "{text}"
    );
}

/// VT2.10's deferred CLI witness binary (D9, D35): the `tcl explore` exit
/// lines slice 2's § *Goal and exit* names — program (3)'s `incr` route
/// twice, an `f5-irules` release-ambiguous decline, and `llength`'s
/// direct route — each hand-verified at the time and now pinned here.
#[test]
fn explore_sccp_prints_the_route_of_each_statement() {
    let program_three = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set n 1; incr n; incr n 2; return $n}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(program_three.contains("n#3 = const(4)"), "{program_three}");
    assert_eq!(
        program_three
            .matches("route incr: direct cell-increment (registry)")
            .count(),
        2,
        "{program_three}"
    );
    assert_eq!(
        program_three.matches("· answer: evaluated").count(),
        2,
        "{program_three}"
    );

    let leading_zero = run_tcl(&[
        "explore",
        "--source",
        "proc p {} {set z 010; incr z}",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
        "--dialect",
        "f5-irules",
    ]);
    assert!(
        leading_zero.contains("· answer: declined: release-ambiguous: numeral-grammar"),
        "{leading_zero}"
    );

    let llength = run_tcl(&[
        "explore",
        "--source",
        "set r [llength {a b}]",
        "--show",
        "sccp",
        "--text",
        "--no-colour",
    ]);
    assert!(
        llength.contains("route llength: direct list-length (registry)"),
        "{llength}"
    );
}

/// VT2.10: `tcl opt --profile full --dialect tcl8.6` forwards program
/// (3)'s constant return into its call site (O100 / O103).
#[test]
fn opt_forwards_program_three() {
    let out = run_tcl(&[
        "opt",
        "--source",
        "proc p {} {set n 1; incr n; incr n 2; return $n}\nputs [p]",
        "--profile",
        "full",
        "--dialect",
        "tcl8.6",
    ]);
    assert!(out.contains("puts 4"), "{out}");
}

/// VT2.10: a value-position cell update whose read the host statement does
/// not hold keeps its store — `set n 1` stays ahead of `set result [incr
/// n]` rather than being read by a join the optimiser cannot see through.
#[test]
fn opt_keeps_the_store_behind_a_nested_increment() {
    let out = run_tcl(&[
        "opt",
        "--source",
        "proc p {} {set n 1; set result [incr n]; puts $result; puts $n}\np",
        "--profile",
        "full",
        "--dialect",
        "tcl8.6",
    ]);
    assert!(out.contains("set n 1"), "{out}");
}

/// VT2.9 (#2214): a `::`-qualified global a nested `incr` writes is a
/// global write in its procedure's summary, so `set hits 0` and `puts
/// $hits` around `bump`'s `[incr ::hits]` are both kept — the program
/// prints `1` under `tclsh8.4` to `tclsh9.1`, as the original does.
#[test]
fn opt_keeps_a_global_a_nested_increment_writes() {
    let source = "set hits 0\nproc bump {} {set y [incr ::hits]; return $y}\nbump\nputs $hits\n";
    let out = run_tcl(&["opt", "--source", source, "--profile", "full"]);
    assert!(out.contains("set hits 0"), "{out}");
    assert!(out.contains("puts $hits"), "{out}");
}
