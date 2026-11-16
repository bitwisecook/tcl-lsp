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

//! What `tcl opt` folds: over builtins, detected dialects, several inputs, the committed samples and each release's arithmetic edges.

use super::*;

/// A builtin reading a variable an earlier fold made constant folds in the
/// same pass (tclsh 8.6/9.0: `set u [string toupper abc]; string length $u`
/// is 3). A value reassigned on some path, or read through a trace, does not.
#[test]
fn opt_folds_a_builtin_over_a_folded_builtin_result() {
    let fold = |source: &str| {
        String::from_utf8(run_tcl(&["opt", "--dialect", "tcl8.6", "--source", source]))
            .expect("utf-8 output")
    };
    let chained = fold("set u [string toupper abc]\nputs [string length $u]\n");
    assert!(chained.contains("puts 3"), "{chained}");
    let expr = fold("set t [expr {3+4}]\nputs [string length $t]\n");
    assert!(expr.contains("puts 1"), "{expr}");
    let reassigned = fold(
        "set u [string toupper abc]\nif {[gets stdin x] > 0} {set u $x}\nputs [string length $u]\n",
    );
    assert!(
        reassigned.contains("puts [string length $u]"),
        "{reassigned}"
    );
    let traced = fold(
        "set u [string toupper abc]\ntrace add variable u read {apply {args {set ::u zz}}}\nputs [string length $u]\n",
    );
    assert!(traced.contains("puts [string length $u]"), "{traced}");
}

/// The transform verbs auto-detect a document's dialect, so an iRule folds
/// the same with and without an explicit `--dialect`.
///
/// If `dialect_or_default()` fell back to `tcl8.6` whenever the flag was
/// absent, the optimiser would run the file as plain Tcl: `contains` would
/// not be an operator, the condition would never fold, and no `O101` would
/// be reported.
#[test]
fn opt_detects_the_irules_dialect_without_the_flag() {
    let input = fixtures_dir().join("wordOperator.irule");
    let detected =
        String::from_utf8(run_tcl(&["opt", input.to_str().unwrap()])).expect("utf-8 output");
    let explicit = String::from_utf8(run_tcl(&[
        "opt",
        "--dialect",
        "f5-irules",
        input.to_str().unwrap(),
    ]))
    .expect("utf-8 output");
    assert!(
        detected.contains("O112") && detected.contains("HTTP::respond 200"),
        "the detected dialect must fold the word-operator condition: {detected}"
    );
    assert_eq!(
        detected, explicit,
        "detection must produce exactly what --dialect f5-irules produces"
    );
}

/// A rewrite made on the `tcl8.6` reading of a document that states no
/// dialect is announced, since `1_000` folds the other way under Tcl 9
/// (#2373); a stated dialect needs no note.
#[test]
fn opt_names_the_release_it_assumed_for_an_unstated_document() {
    let source = "proc p {} {set s 1_000; if {$s == 1000} {puts yes} else {puts no}}";
    let assumed = tcl()
        .args(["opt", "--profile", "full", "--source", source])
        .reused_output()
        .expect("failed to spawn tcl binary");
    let stderr = String::from_utf8_lossy(&assumed.stderr);
    assert!(
        stderr.contains("no dialect stated or detected; optimised as tcl8.6"),
        "{stderr}"
    );
    let stated = tcl()
        .args([
            "opt",
            "--profile",
            "full",
            "--dialect",
            "tcl8.6",
            "--source",
            source,
        ])
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(
        !String::from_utf8_lossy(&stated.stderr).contains("no dialect stated"),
        "{}",
        String::from_utf8_lossy(&stated.stderr)
    );
}

/// The control for [`opt_detects_the_irules_dialect_without_the_flag`]: the
/// same condition in plain Tcl source stays plain Tcl, where `contains` is not
/// an operator and nothing folds.
#[test]
fn opt_leaves_a_word_operator_alone_in_plain_tcl() {
    let out = String::from_utf8(run_tcl(&[
        "opt",
        "--source",
        "set x \"abcdef\"\nif {$x contains \"cd\"} { puts hit }",
    ]))
    .expect("utf-8 output");
    assert!(
        !out.contains("if {1}"),
        "plain Tcl has no `contains` operator, so the condition must not fold: {out}"
    );
}

/// Companion to #2120: fixing the cross-file fold must not change what the
/// output *is*. README and `kcs-feature-tcl-verb-cli` document
/// `tcl opt src/ -o build/optimised.tcl` as optimising a tree "into one
/// output script", so the rendered text stays a program: no `# file:` banner
/// may precede it, or a leading `#!` is pushed off byte zero and the result
/// is no longer executable. Per-file attribution belongs in the trailing
/// comment block, which cannot corrupt the script.
#[test]
fn opt_over_several_inputs_keeps_the_first_shebang_at_byte_zero() {
    let out = String::from_utf8(run_tcl(&[
        "opt",
        "--source",
        "#!/usr/bin/env tclsh\nset a [expr {1 + 1}]\nputs $a",
        "--source",
        "set b [expr {2 + 2}]\nputs $b",
    ]))
    .expect("utf-8 output");
    assert!(
        out.starts_with("#!/usr/bin/env tclsh"),
        "the first input's shebang must stay at byte 0 so the bundled script \
         is still executable: {out}"
    );
    // The attribution still has to be somewhere — in the comment block.
    assert!(
        out.contains("# optimised:"),
        "the rewrite summary must still be reported: {out}"
    );
}

/// Each input is optimised independently; a store cannot flow across files.
#[test]
fn opt_does_not_fold_a_store_across_a_file_boundary() {
    let out = String::from_utf8(run_tcl(&[
        "opt",
        "--source",
        "set shared_value 42",
        "--source",
        "puts $shared_value",
    ]))
    .expect("utf-8 output");
    assert!(
        out.contains("puts $shared_value"),
        "the second input never sees the first input's assignment at run \
         time, so the read must stay a variable read, not fold to a literal: \
         {out}"
    );
    assert!(
        !out.contains("puts 42"),
        "the old concatenating path forwarded the literal across the file \
         boundary: {out}"
    );
    assert!(
        out.contains("set shared_value 42"),
        "the first input's store must survive — it is not dead just because \
         a *different* file never reads it: {out}"
    );
    assert!(
        !out.contains("O109"),
        "the old path eliminated the store as a dead store once the fold \
         made it look unused: {out}"
    );
}

/// The committed `samples/optimiser/` outputs are what the current optimiser
/// produces, byte for byte.
///
/// Without a test comparing them to a real run, committed samples can drift
/// from actual behaviour undetected — e.g. showing an `incr` rewrite the
/// current optimiser declines, or a footer format it no longer emits. The
/// regeneration loop in `samples/optimiser/README.md` is exactly this test,
/// so a pass that changes what any profile emits fails here until the
/// samples are refreshed with it.
#[test]
fn samples_optimiser_profiles_are_regenerated() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = root.join("samples/optimiser/input.tcl");
    let input = input.to_str().expect("the sample path is UTF-8");
    for profile in ["readability", "standard", "full", "aggressive"] {
        let produced = String::from_utf8(run_tcl(&["opt", "--profile", profile, input]))
            .expect("tcl opt emits UTF-8");
        let committed_path = root.join(format!("samples/optimiser/profile_{profile}.tcl"));
        let committed = std::fs::read_to_string(&committed_path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", committed_path.display()));
        assert_eq!(
            produced, committed,
            "samples/optimiser/profile_{profile}.tcl no longer matches the optimiser. \
             Regenerate the four outputs with the loop in samples/optimiser/README.md, \
             and check the per-profile prose there still describes what they show.",
        );
    }
}

/// Every dialect the constant folder is asked to answer for: the unnamed
/// default, each core release and Jim.
const FOLD_DIALECTS: [Option<&str>; 7] = [
    None,
    Some("tcl8.4"),
    Some("tcl8.5"),
    Some("tcl8.6"),
    Some("tcl9.0"),
    Some("tcl9.1"),
    Some("jim"),
];

/// Run `tcl VERB --source SOURCE` under `dialect`, tolerating a non-zero exit
/// (findings) but never an abort: a panic exits 101 with `panicked at` on
/// stderr. Returns stdout.
fn run_on_source_without_panic(verb_args: &[&str], dialect: Option<&str>, source: &str) -> String {
    let mut command = tcl();
    command.args(verb_args);
    if let Some(dialect) = dialect {
        command.args(["--dialect", dialect]);
    }
    let output = command
        .args(["--source", source])
        .reused_output()
        .expect("failed to spawn tcl binary");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.code() != Some(101) && !stderr.contains("panicked"),
        "tcl {verb_args:?} --dialect {dialect:?} aborted on {source:?}: {stderr}"
    );
    String::from_utf8(output.stdout).expect("utf-8 output")
}

/// #2401: `abs` of the most negative wide is `2^63`, past what the folder's
/// math-call shape can carry. `tcl opt` and `tcl diag` aborted on it in every
/// dialect. The call is left to run — tclsh 8.5 to 9.1 print
/// `9223372036854775808`, 8.4 raises `ARITH IOVERFLOW`, and jimsh 0.84 wraps
/// to `-9223372036854775808`, so no single folded constant is right — and the
/// same holds wherever the operand reaches `abs` by another spelling.
#[test]
fn opt_and_diag_decline_abs_of_the_most_negative_wide_in_every_dialect() {
    for expr in [
        "abs(-9223372036854775808)",
        "abs(-9223372036854775807-1)",
        "int(abs(-9223372036854775808)) - 1",
    ] {
        let program = format!("set x [expr {{{expr}}}]\nputs $x\n");
        for dialect in FOLD_DIALECTS {
            let optimised =
                run_on_source_without_panic(&["opt", "--profile", "full"], dialect, &program);
            assert!(
                optimised.contains(&format!("expr {{{expr}}}")),
                "`{expr}` must be left to run under {dialect:?}: {optimised}"
            );
        }
    }
    let program = "set x [expr {abs(-9223372036854775808)}]\nputs $x\n";
    for dialect in FOLD_DIALECTS {
        run_on_source_without_panic(&["diag"], dialect, program);
    }
}

/// Beside #2401, the remaining wide-edge folds: `-(-9223372036854775808)`
/// and the quotient of it by `-1` promote to a bignum, which only a target
/// that widens (8.5 onward) folds; and `i64::MIN % -1` is `0` from 8.5 but a
/// hardware divide trap that kills tclsh 8.4 and jimsh (`SIGFPE`, measured),
/// so those never fold it to `0`.
#[test]
fn opt_folds_the_wide_edges_only_where_the_release_computes_them() {
    for (expr, widened) in [
        ("-(-9223372036854775808)", "9223372036854775808"),
        ("(-9223372036854775807 - 1) / -1", "9223372036854775808"),
        ("(-9223372036854775807 - 1) % -1", "0"),
    ] {
        let program = format!("puts [expr {{{expr}}}]\n");
        for dialect in FOLD_DIALECTS {
            let optimised =
                run_on_source_without_panic(&["opt", "--profile", "full"], dialect, &program);
            let folds = matches!(
                dialect,
                None | Some("tcl8.5" | "tcl8.6" | "tcl9.0" | "tcl9.1")
            );
            assert_eq!(
                optimised.contains(&format!("puts {widened}\n")),
                folds,
                "`{expr}` under {dialect:?}: {optimised}"
            );
            if !folds {
                assert!(optimised.contains(expr), "{dialect:?}: {optimised}");
            }
        }
    }
}

/// #2435: a decimal spelling outside the double range is a double from 8.5 and
/// not on 8.4, whose `strtod` reports `ERANGE`. Jim refuses it too, but names
/// no core release, so the fold declines there. Every row measured on tclsh
/// 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0 and jimsh 0.84.
#[test]
fn opt_folds_string_is_double_range_errors_per_release() {
    for spelling in ["1e400", "-1e-400", "-strict 1e400", "4.9e-324"] {
        let program = format!("puts [string is double {spelling}]\n");
        for dialect in FOLD_DIALECTS {
            let optimised =
                run_on_source_without_panic(&["opt", "--profile", "full"], dialect, &program);
            match dialect {
                Some("tcl8.4") => assert!(
                    optimised.contains("puts 0\n"),
                    "8.4 reads `{spelling}` as no double: {optimised}"
                ),
                Some("jim") => assert!(
                    optimised.contains(&format!("string is double {spelling}")),
                    "Jim names no release to fold under: {optimised}"
                ),
                _ => assert!(
                    optimised.contains("puts 1\n"),
                    "{dialect:?} takes `{spelling}` as the rounded double: {optimised}"
                ),
            }
        }
    }
}

/// The boundary doubles and a zero mantissa are doubles on every release, and
/// `expr` on a range-error literal is left to run under 8.4 (it raises
/// `ARITH OVERFLOW` / `UNDERFLOW` there) while 8.5 onward read the value.
#[test]
fn opt_keeps_in_range_doubles_and_declines_an_8_4_range_error_literal() {
    for spelling in ["1.7976931348623157e308", "0e999", "0.0e-400", "Inf"] {
        let program = format!("puts [string is double {spelling}]\n");
        for dialect in [Some("tcl8.4"), Some("tcl9.0")] {
            let optimised =
                run_on_source_without_panic(&["opt", "--profile", "full"], dialect, &program);
            assert!(
                optimised.contains("puts 1\n"),
                "`{spelling}` is a double under {dialect:?}: {optimised}"
            );
        }
    }
    let program = "puts [expr {4.9e-324}]\n";
    let on_84 = run_on_source_without_panic(&["opt", "--profile", "full"], Some("tcl8.4"), program);
    assert!(on_84.contains("expr {4.9e-324}"), "{on_84}");
    let on_85 = run_on_source_without_panic(&["opt", "--profile", "full"], Some("tcl8.5"), program);
    assert!(on_85.contains("puts 5e-324\n"), "{on_85}");
}
