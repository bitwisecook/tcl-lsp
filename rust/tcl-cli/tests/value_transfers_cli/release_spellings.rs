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

//! What the folder may write for each release: double spellings, operators the release lacks, `tcl_precision` and the script encoding.

use super::*;

/// `tcl opt --profile full` over `source` under the Jim dialect.
fn opt_under_jim(source: &str) -> String {
    run_tcl(&[
        "opt",
        "--source",
        source,
        "--profile",
        "full",
        "--dialect",
        "jim",
    ])
}

#[test]
fn opt_preserves_constant_fixture_outcomes() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/work/name-handling/handover/oo-dispatch-probe/constants/folds");
    let oracles = tclshs_from("8.4");
    let script = std::env::temp_dir().join(format!(
        "value-transfers-cli-{}-constant-fixtures.tcl",
        std::process::id()
    ));
    for (name, must_rewrite) in [
        ("c13-number-forms.tcl", false),
        ("c14-float-format.tcl", false),
        ("c18-octal-release.tcl", true),
        ("e06-float.tcl", true),
        ("e07-float-text.tcl", true),
    ] {
        let source = std::fs::read_to_string(fixtures.join(name)).expect("original fold fixture");
        for series in RELEASES {
            // File output has no optimiser report, so even whitespace and
            // comments participate in the source comparison.
            run_tcl(&[
                "opt",
                "--source",
                &source,
                "--profile",
                "full",
                "--dialect",
                tcl_dialect::TclVersion::profile_name_for(series).expect("modelled release"),
                "--output",
                script.to_str().expect("UTF-8 oracle script path"),
            ]);
            let optimised = std::fs::read(&script).expect("optimised fixture bytes");
            assert_eq!(
                source.as_bytes() != optimised,
                must_rewrite,
                "tcl{series} {name}: unexpected source bytes {optimised:?}"
            );
            if let Some((_, tclsh)) = oracles.iter().find(|(release, _)| *release == series) {
                let tclsh = Path::new(tclsh);
                // Tcl includes this path in an uncaught error's traceback.
                std::fs::write(&script, &source).expect("write original fixture");
                let before = tcl_test_support::run_script_file(tclsh, &script)
                    .expect("run original fixture");
                std::fs::write(&script, &optimised).expect("write optimised fixture");
                let after = tcl_test_support::run_script_file(tclsh, &script)
                    .expect("run optimised fixture");
                let expected_status = i32::from(series == "8.4" && name == "c14-float-format.tcl");
                assert_eq!(
                    before.exit_code,
                    Some(expected_status),
                    "tclsh{series} {name}: {before:?}"
                );
                assert_eq!(
                    before, after,
                    "tclsh{series} {name}: native outcome changed"
                );
            }
            std::fs::remove_file(&script).expect("remove oracle script");
        }
    }
}

/// Run `source` and its `opt` rewrite under the reference tclsh of each of
/// `releases`: both print `printed`.
fn both_print(source: &str, releases: &[&str], printed: &str) {
    let tclshs = tclshs_from("8.4");
    for (series, tclsh) in tclshs
        .iter()
        .filter(|(series, _)| releases.contains(series))
    {
        let optimised = statements_of(&opt_under(source, series));
        for program in [source, optimised.as_str()] {
            assert_eq!(
                run_tclsh(tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// tclsh 8.5.19 to 9.1.0 print 2^64 as `1.844674407370955e+19`, which reads
/// back as 2^64 - 2048, so the folder writes no literal for it: `entier` of
/// the stored value is still 18446744073709551616, and the printed double is
/// still tclsh's own spelling.
#[test]
fn opt_writes_no_literal_for_a_double_whose_spelling_does_not_read_back() {
    let source = "set x [expr {2.0**64}]\nputs [expr {entier($x)}]\nputs [expr {2.0**64}]\n";
    for series in ["8.5", "8.6", "9.0", "9.1"] {
        let optimised = statements_of(&opt_under(source, series));
        assert!(
            !optimised.contains("e+19"),
            "tcl{series}: a lossy literal was written\n{optimised}"
        );
    }
    both_print(
        source,
        &["8.5", "8.6", "9.0", "9.1"],
        "18446744073709551616\n1.844674407370955e+19\n",
    );
}

/// Tcl 8.4 and Jim print a computed double with `%.12g`: the folder spells
/// one that reads back the way they do (`1e-05`) and writes none for one
/// that does not (`1.0/3` is `0.333333333333`, whose `string length` is 14
/// on 8.4, not the 18 of 8.5's spelling) — #2395.
#[test]
fn opt_spells_doubles_as_tcl_8_4_and_jim_do() {
    let source = "puts [expr {1e-5 * 1}]\nset y [expr {1.0/3}]\nputs $y\nputs [string length $y]\n";
    let tcl84 = statements_of(&opt_under(source, "8.4"));
    let jim = statements_of(&opt_under_jim(source));
    for (dialect, optimised) in [("tcl8.4", &tcl84), ("jim", &jim)] {
        assert!(
            !optimised.contains("0.3333333333333333") && !optimised.contains("puts 18"),
            "{dialect}: folded under 8.5's spelling\n{optimised}"
        );
        assert!(!optimised.contains("1e-5\n"), "{dialect}:\n{optimised}");
    }
    assert!(tcl84.contains("puts 1e-05"), "{tcl84}");
    both_print(source, &["8.4"], "1e-05\n0.333333333333\n14\n");
    let later = statements_of(&opt_under(source, "8.6"));
    assert!(later.contains("puts 1e-5"), "{later}");
}

/// A script that sets `tcl_precision` changes how 8.4 to 8.6 print a double
/// (9.0 removed the variable): the folder spells no computed double there,
/// and folds as before under 9.0 and 9.1.
#[test]
fn opt_spells_no_double_where_the_script_sets_tcl_precision() {
    let source = "set tcl_precision 6\nset a [expr {1.5 * 2}]\nputs $a\nputs [expr {1.0/3}]\n";
    for series in ["8.4", "8.5", "8.6"] {
        let optimised = statements_of(&opt_under(source, series));
        assert!(
            !optimised.contains("puts 3.0") && !optimised.contains("0.3333"),
            "tcl{series}: a double folded under a script-set precision\n{optimised}"
        );
    }
    for series in ["9.0", "9.1"] {
        let optimised = statements_of(&opt_under(source, series));
        assert!(optimised.contains("puts 3.0"), "tcl{series}:\n{optimised}");
    }
    for (series, tclsh) in tclshs_from("8.4") {
        let optimised = statements_of(&opt_under(source, series));
        assert_eq!(
            run_tclsh(&tclsh, source),
            run_tclsh(&tclsh, &optimised),
            "tclsh{series}:\n{optimised}"
        );
    }
    // A name the program computes may be `tcl_precision` too.
    let computed = "set v tcl_\nappend v precision\nset $v 6\nputs [expr {1.5 * 2}]\n";
    let optimised = statements_of(&opt_under(computed, "8.6"));
    assert!(!optimised.contains("puts 3.0"), "{optimised}");
}

/// tclsh 8.4.20 and 8.5.19 print a computed `INT64_MIN` as `-8`
/// (`TclFormatInt`), so the folder leaves the computation for the program's
/// own runtime there; 8.6 onward print the digits, which the folder writes.
#[test]
fn opt_writes_no_literal_for_int64_min_where_the_release_prints_minus_eight() {
    let source = "puts [expr {-9223372036854775807 - 1}]\n\
                  puts [string length [expr {~9223372036854775807}]]\n\
                  proc h {} {return [expr {int(-9223372036854775808.0)}]}\nputs [h]\n";
    for series in ["8.4", "8.5"] {
        let optimised = statements_of(&opt_under(source, series));
        assert!(
            !optimised.contains("9223372036854775808\n") && !optimised.contains("puts 20"),
            "tcl{series}: folded the digits\n{optimised}"
        );
    }
    for series in ["8.6", "9.0", "9.1"] {
        let optimised = statements_of(&opt_under(source, series));
        assert!(
            optimised.contains("puts -9223372036854775808"),
            "tcl{series}:\n{optimised}"
        );
    }
    for (series, tclsh) in tclshs_from("8.4") {
        let printed = if series < "8.6" {
            "-8\n2\n-8\n"
        } else {
            "-9223372036854775808\n20\n-9223372036854775808\n"
        };
        let optimised = statements_of(&opt_under(source, series));
        for program in [source, optimised.as_str()] {
            assert_eq!(
                run_tclsh(&tclsh, program),
                Some((true, printed.to_owned())),
                "tclsh{series}:\n{program}"
            );
        }
    }
}

/// `source` and its `opt` rewrite under `tclsh`: both exit alike and print
/// alike — a fold never turns the program's error into a value.
fn rewrite_behaves_alike(source: &str, series: &str, tclsh: &str) -> String {
    let optimised = statements_of(&opt_under(source, series));
    assert_eq!(
        run_tclsh(tclsh, source),
        run_tclsh(tclsh, &optimised),
        "tclsh{series}:\n{source}\n---\n{optimised}"
    );
    optimised
}

/// An operator or math function the target release lacks folds nothing:
/// tclsh 8.4.20 rejects `2**3` (`unexpected operator *`), `in`/`ni`
/// (`invalid bareword`) and `max`/`entier`/`isqrt`/`bool` (`unknown math
/// function`); 8.4 to 8.6 reject `lt`/`le`/`gt`/`ge`; 8.5 and 8.6 lack the
/// 9.0 `is*` classifiers; 9.0 lacks 9.1's TIP 745 functions. `eq`/`ne` and
/// `wide` exist from 8.4. Each program prints under every tclsh what its
/// rewrite prints, and a release that has the construct still folds it.
#[test]
fn opt_folds_no_operator_or_function_the_release_lacks() {
    // (expression, value, first release that has it)
    let cases = [
        ("2**3", "8", "8.5"),
        ("1 in {1 2}", "1", "8.5"),
        ("1 ni {1 2}", "0", "8.5"),
        ("\"a\" lt \"b\"", "1", "9.0"),
        ("\"a\" ge \"b\"", "0", "9.0"),
        ("\"a\" eq \"a\"", "1", "8.4"),
        ("\"a\" ne \"b\"", "1", "8.4"),
        ("max(1,2)", "2", "8.5"),
        ("entier(2.5)", "2", "8.5"),
        ("isqrt(17)", "4", "8.5"),
        ("bool(1)", "1", "8.5"),
        ("wide(3)", "3", "8.4"),
        ("isinf(1.0)", "0", "9.0"),
        ("log2(8)", "3.0", "9.1"),
        ("cbrt(8)", "2.0", "9.1"),
    ];
    for (expression, value, since) in cases {
        let source = format!("puts [expr {{{expression}}}]\n");
        for series in RELEASES {
            let optimised = statements_of(&opt_under(&source, series));
            let folded = optimised.contains(&format!("puts {value}"));
            assert_eq!(
                folded,
                series >= since,
                "tcl{series}: `{expression}` folded={folded}\n{optimised}"
            );
        }
        for (series, tclsh) in tclshs_from("8.4") {
            rewrite_behaves_alike(&source, series, &tclsh);
        }
    }
}

/// Jim's `pow` is its `**` operator: jimsh 0.84 prints `8` for `pow(2,3)`
/// and `0` for `pow(2,-1)`, where every tclsh prints `8.0` and `0.5`, so
/// the folder writes neither under Jim and still folds a double operand.
#[test]
fn opt_folds_no_integer_pow_under_jim() {
    let optimised = statements_of(&opt_under_jim(
        "puts [expr {pow(2,3)}]\nputs [expr {pow(2,-1)}]\nputs [expr {pow(2.0,3)}]\n",
    ));
    assert!(
        !optimised.contains("puts 8.0\nputs") && !optimised.contains("puts 0.5"),
        "{optimised}"
    );
    assert!(optimised.trim_end().ends_with("puts 8.0"), "{optimised}");
    let tcl = statements_of(&opt_under("puts [expr {pow(2,3)}]\n", "8.6"));
    assert!(tcl.contains("puts 8.0"), "{tcl}");
}

/// `tcl diag` already reports an operator the release lacks (W003), and an
/// unreachable-branch claim (I230) never rests on a condition the release
/// cannot evaluate: `$x ni {1 2}` raises under tclsh 8.4.
#[test]
fn diag_reports_an_operator_the_release_lacks_and_decides_nothing_with_it() {
    let source = "set x 3\nputs [expr {2**3}]\nif {$x ni {1 2}} { puts no }\n";
    let tcl84 = diagnostics_at(source, "tcl8.4", &[]);
    assert!(tcl84.contains(&(2, 14, "W003".to_owned())), "{tcl84:?}");
    assert!(
        tcl84
            .iter()
            .any(|(line, _, code)| *line == 3 && code == "W003"),
        "{tcl84:?}"
    );
    assert!(
        !tcl84.iter().any(|(_, _, code)| code == "I230"),
        "{tcl84:?}"
    );
    let tcl85 = diagnostics_at(source, "tcl8.5", &[]);
    assert!(
        !tcl85.iter().any(|(_, _, code)| code == "W003"),
        "{tcl85:?}"
    );
    assert!(
        tcl85
            .iter()
            .any(|(line, _, code)| *line == 3 && code == "I230"),
        "{tcl85:?}"
    );
}

/// tclsh 8.4.20 raises on a double literal outside the double range
/// (`4.9e-324`: `floating-point value too large to represent`) and on
/// reading a subnormal back from text (`set x 1e-310; expr {$x * 1}`), but
/// computes and prints a subnormal (`1e-300 * 1e-10` is `1e-310`). Each
/// program's rewrite under 8.4 behaves as the program does, and a computed
/// subnormal is not written as a literal 8.4 cannot read back.
#[test]
fn opt_under_8_4_keeps_what_its_double_range_decides() {
    let programs = [
        "puts [expr {4.9e-324}]\n",
        "puts [expr {1e-310 * 1}]\n",
        "set x 1e-310\nputs [expr {$x * 1}]\n",
        "set y [expr {1e-300 * 1e-10}]\nputs [expr {$y * 1}]\n",
        "puts [expr {1e-400}]\n",
    ];
    for source in programs {
        let optimised = statements_of(&opt_under(source, "8.4"));
        assert!(!optimised.contains("set y 1e-310"), "{optimised}");
        for (series, tclsh) in tclshs_from("8.4").iter().filter(|(s, _)| *s == "8.4") {
            rewrite_behaves_alike(source, series, tclsh);
        }
    }
}

/// `format %g` is C's `%g` at its own default precision, which
/// `tcl_precision` never reaches: after `set tcl_precision 3`, tclsh 8.4.20
/// to 9.1.0 all print `format %g 1.23456` as `1.23456`, while a computed
/// double's own string rep (`format %s [expr {...}]`) is `1.23` through 8.6.
/// So the folder still folds the first and leaves the second under 8.4 to
/// 8.6.
#[test]
fn opt_folds_format_g_whatever_tcl_precision_holds() {
    let source = "set tcl_precision 3\nputs [format %g 1.23456]\n\
                  puts [format %s [expr {1.23456 * 1}]]\n";
    for series in RELEASES {
        let optimised = statements_of(&opt_under(source, series));
        assert!(
            optimised.contains("puts 1.23456\n"),
            "tcl{series}:\n{optimised}"
        );
        if series < "9.0" {
            assert!(
                optimised.contains("[expr {1.23456 * 1}]"),
                "tcl{series}:\n{optimised}"
            );
        }
    }
    for (series, tclsh) in tclshs_from("8.4") {
        rewrite_behaves_alike(source, series, &tclsh);
    }
}

/// #2129: `--source-encoding` declares the encoding the target reads the
/// script in. The file holds the UTF-8 bytes of `É` (`C3 89`) in
/// `string range "Éabc" 1 2`. Measured: tclsh 8.6.18 under `LC_ALL=C`
/// (`encoding system` iso8859-1) prints the bytes `89 61`; tclsh 8.6.18 and
/// 9.0.4 with `source -encoding iso8859-1` give `"\u{89}a"` and with
/// `source -encoding utf-8` give `ab`. `auto` folds only where the release
/// reads UTF-8 (9.0, 9.1); a declared encoding folds exactly under every
/// release, and the rewrite is written back in that encoding.
#[test]
fn opt_folds_a_non_ascii_literal_under_the_declared_script_encoding() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("source-encoding-2129");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let file = dir.join("range.tcl");
    std::fs::write(&file, b"puts [string range \"\xC3\x89abc\" 1 2]\n").expect("fixture");
    let first_line = |dialect: &str, encoding: &str| -> Vec<u8> {
        let output = tcl()
            .args([
                "opt",
                "--dialect",
                dialect,
                "--source-encoding",
                encoding,
                "--no-colour",
            ])
            .arg(&file)
            .reused_output()
            .expect("tcl opt runs");
        assert!(output.status.success(), "{dialect} {encoding}: {output:?}");
        output
            .stdout
            .split(|&byte| byte == b'\n')
            .next()
            .unwrap_or_default()
            .to_vec()
    };
    let unchanged = b"puts [string range \"\xC3\x89abc\" 1 2]".to_vec();
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let auto = if dialect.starts_with("tcl9") {
            b"puts ab".to_vec()
        } else {
            unchanged.clone()
        };
        assert_eq!(first_line(dialect, "auto"), auto, "{dialect} auto");
        assert_eq!(
            first_line(dialect, "iso8859-1"),
            b"puts \x89a".to_vec(),
            "{dialect} iso8859-1"
        );
        assert_eq!(
            first_line(dialect, "utf-8"),
            b"puts ab".to_vec(),
            "{dialect} utf-8"
        );
    }
}
