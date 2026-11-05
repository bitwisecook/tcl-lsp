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

//! Oracle test for regex-source tracking, using a real `tclsh9.0` as the
//! ground truth.
//!
//! The feature highlights the def-site *literal* that a variable carries into a
//! `regexp` pattern.  Correctness means: the literal we highlight, with its
//! surrounding delimiters stripped, is *exactly* the string the interpreter
//! binds to that variable (the text `regexp` actually compiles).  We verify
//! that by asking a real `tclsh9.0` for the variable's value and comparing.
//!
//! Skips cleanly when no `tclsh9.0` is on `PATH` (CI without a Tcl 9 build).

use std::io::Write;
use std::process::{Command, Stdio};

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::regex_source::regex_source_literal_spans;
use tcl_registry::CommandRegistry;

/// Run `script` on `tclsh` via stdin, returning `(exit_ok, stdout)`.
fn run_tcl(tclsh: &str, script: &str) -> Option<(bool, String)> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    {
        let mut stdin = child.stdin.take()?;
        stdin.write_all(script.as_bytes()).ok()?;
    }
    let out = child.wait_with_output().ok()?;
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    ))
}

/// Locate a `tclsh9.0` (or a `tclsh` reporting a `9.x` patchlevel) on `PATH`.
fn find_tclsh9() -> Option<String> {
    for cand in ["tclsh9.0", "tclsh"] {
        if let Some((true, out)) = run_tcl(cand, "puts -nonewline [info patchlevel]")
            && out.starts_with("9.")
        {
            return Some(cand.to_string());
        }
    }
    None
}

/// Strip a quoted/braced word's surrounding delimiters.
fn inner(word: &str) -> &str {
    let b = word.as_bytes();
    match (b.first(), b.last()) {
        (Some(b'"'), Some(b'"')) | (Some(b'{'), Some(b'}')) if word.len() >= 2 => {
            &word[1..word.len() - 1]
        }
        _ => word,
    }
}

fn highlighted_literals(source: &str) -> Vec<String> {
    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(source, &registry, false);
    regex_source_literal_spans(
        source,
        &cu,
        &registry,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
    .into_iter()
    .map(|s| source[s.start() as usize..s.end() as usize].to_owned())
    .collect()
}

#[test]
fn highlighted_literal_matches_tcl9_variable_value() {
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping regex_source oracle: no tclsh9.0 on PATH");
        return;
    };

    // Each literal is assigned to a variable that flows into a `regexp`
    // pattern.  The literal (delimiters stripped) must equal the runtime value
    // tclsh binds to the variable.
    let literals = [
        "{a+b}",
        "\".*abc\"",
        "{[0-9]+}",
        "\"foo|bar\"",
        "{^\\d{3}$}",
    ];

    for lit in literals {
        let feature_src = format!("set r {lit}\nregexp $r $s\n");
        let got = highlighted_literals(&feature_src);
        assert_eq!(got.len(), 1, "expected exactly one regex source for {lit}");
        let our_inner = inner(&got[0]);

        // Ground truth: what does tclsh bind to `$r`?
        let (ok, val) =
            run_tcl(&tclsh, &format!("set r {lit}\nputs -nonewline $r")).expect("tclsh runs");
        assert!(ok, "tclsh failed for {lit}");
        assert_eq!(
            our_inner, val,
            "highlighted literal {our_inner:?} != tclsh value {val:?} for {lit}"
        );
    }
}

#[test]
fn highlighted_literal_inside_namespace_eval_matches_tcl9() {
    // A regex source assigned *inside* a `namespace eval` body flows through the
    // synthetic body unit. The highlighted literal must still equal the value
    // the interpreter binds to the (namespace-scoped) variable — read back as
    // `${::ns::r}` after the eval completes.
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping regex_source oracle: no tclsh9.0 on PATH");
        return;
    };
    for lit in ["{a+b}", "\".*abc\"", "{[0-9]+}"] {
        let feature_src = format!("namespace eval ::ns {{\n  set r {lit}\n  regexp $r $s\n}}\n");
        let got = highlighted_literals(&feature_src);
        assert_eq!(
            got.len(),
            1,
            "expected one regex source inside ns-eval for {lit}"
        );
        let our_inner = inner(&got[0]);
        let (ok, val) = run_tcl(
            &tclsh,
            &format!("namespace eval ::ns {{ set r {lit} }}\nputs -nonewline ${{::ns::r}}"),
        )
        .expect("tclsh runs");
        assert!(ok, "tclsh failed for {lit}");
        assert_eq!(
            our_inner, val,
            "ns-eval highlighted literal {our_inner:?} != tclsh value {val:?} for {lit}"
        );
    }
}

#[test]
fn highlighted_literal_inside_apply_lambda_matches_tcl9() {
    // A regex source assigned inside an `apply` lambda body flows through the
    // synthetic body unit. The lambda local does not outlive the call, so the
    // oracle prints the value from *inside* the lambda; the highlighted literal
    // must equal it.
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping regex_source oracle: no tclsh9.0 on PATH");
        return;
    };
    for lit in ["{a+b}", "\"foo|bar\"", "{^\\d{3}$}"] {
        let feature_src = format!("apply {{{{s}} {{\n  set r {lit}\n  regexp $r $s\n}}}} foo\n");
        let got = highlighted_literals(&feature_src);
        assert_eq!(
            got.len(),
            1,
            "expected one regex source inside apply for {lit}"
        );
        let our_inner = inner(&got[0]);
        let (ok, val) = run_tcl(
            &tclsh,
            &format!("apply {{{{}} {{ set r {lit}; puts -nonewline $r }}}}"),
        )
        .expect("tclsh runs");
        assert!(ok, "tclsh failed for {lit}");
        assert_eq!(
            our_inner, val,
            "apply highlighted literal {our_inner:?} != tclsh value {val:?} for {lit}"
        );
    }
}

#[test]
fn interpolated_source_is_conservatively_not_tracked() {
    // A value built by *string interpolation* (`set r "$p.*"`) is marked
    // `Overdefined` by the SCCP lattice (interpolation folding is a separate
    // minify pass, not a lattice fact), so the string-constant gate declines
    // it.  Highlighting nothing here is the conservative, false-positive-free
    // choice — verified against tclsh, which *does* resolve a concrete value,
    // documenting the deliberate gap between "lattice-constant" and
    // "runtime-constant".
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping regex_source oracle: no tclsh9.0 on PATH");
        return;
    };
    let src = "set p abc\nset r \"$p.*\"\nregexp $r $s\n";
    assert!(
        highlighted_literals(src).is_empty(),
        "interpolated value is not a lattice constant → not tracked"
    );
    // tclsh nonetheless resolves the pattern — the gap is intentional.
    let (ok, val) =
        run_tcl(&tclsh, "set p abc\nset r \"$p.*\"\nputs -nonewline $r").expect("tclsh runs");
    assert!(ok);
    assert_eq!(val, "abc.*");
}
