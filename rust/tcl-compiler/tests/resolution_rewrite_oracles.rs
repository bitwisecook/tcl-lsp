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

//! Independent references must execute source rewrites equivalently, using
//! the same fixtures and dialect-specific observations as other consumers.

use std::fs;
use std::path::Path;

use tcl_compiler::optimiser::manager::optimise_source_multipass;
use tcl_registry::model::ingress::{resolve_environment, static_context_for};
use tcl_syntax::execution_conformance::{
    ExecutionDomain, RewriteCase, filesystem_cases, rewrite_cases, vectors,
};
use tcl_test_support::{
    ScriptOutcome, available_tclshs, locate_jimsh, require_jimsh, run_script_file,
    run_script_fixture,
};

fn optimise(source: &str, dialect: &str) -> String {
    let registry = static_context_for(dialect).commands();
    let profile = resolve_environment(dialect).analyser_profile();
    optimise_source_multipass(source, registry, Some(profile), 5).0
}

fn run_file(interpreter: &Path, source: &str, label: &str) -> ScriptOutcome {
    let root = std::env::temp_dir().join(format!(
        "resolution-rewrite-{}-{}-{label}",
        std::process::id(),
        interpreter
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    fs::create_dir_all(&root).expect("fixture directory");
    let path = root.join("case.tcl");
    fs::write(&path, source).expect("fixture source");
    let result = run_script_file(interpreter, &path).expect("execute reference source");
    fs::remove_dir_all(root).expect("remove fixture directory");
    result
}

fn first_error(outcome: &ScriptOutcome) -> String {
    let text = std::str::from_utf8(&outcome.stderr).expect("UTF-8 fixture error");
    let line = text.lines().next().unwrap_or_default();
    line.split_once(": Error: ")
        .map_or(line, |(_, message)| message)
        .to_owned()
}

fn assert_fixture(case: &RewriteCase, interpreter: &Path, dialect: &str, jim: bool) {
    let original = run_file(interpreter, case.source, case.name);
    let expected_output = if jim { case.jim_stdout } else { case.c_stdout };
    let expected_error = if jim { case.jim_error } else { case.c_error };
    assert_eq!(
        original.stdout,
        expected_output.as_bytes(),
        "{} {dialect}: original output",
        case.name
    );
    assert_eq!(
        first_error(&original),
        expected_error.unwrap_or_default(),
        "{} {dialect}: original error",
        case.name
    );
    assert_eq!(
        original.success(),
        expected_error.is_none(),
        "{} {dialect}: original status",
        case.name
    );
    let rewritten = optimise(case.source, dialect);
    let after = run_file(interpreter, &rewritten, case.name);
    assert_eq!(
        after.exit_code, original.exit_code,
        "{} {dialect}: changed status\n{rewritten}",
        case.name
    );
    assert_eq!(
        after.stdout, original.stdout,
        "{} {dialect}: changed output\n{rewritten}",
        case.name
    );
    assert_eq!(
        first_error(&after),
        first_error(&original),
        "{} {dialect}: changed error\n{rewritten}",
        case.name
    );
}

fn assert_shared_vectors(interpreter: &Path, dialect: &str) {
    for case in filesystem_cases() {
        let source = case.script();
        let original =
            run_script_fixture(interpreter, &source, case.files).expect("original loader fixture");
        let expected = if dialect == "jim" {
            case.jim_want
        } else {
            case.c_want
        };
        assert_eq!(
            original
                .strict_text()
                .expect("clean original loader result"),
            expected,
            "{} {dialect}: original loader",
            case.name
        );
        let rewritten = optimise(&source, dialect);
        let after = run_script_fixture(interpreter, &rewritten, case.files)
            .expect("rewritten loader fixture");
        assert_eq!(
            after.exit_code, original.exit_code,
            "{} {dialect}: loader status",
            case.name
        );
        assert_eq!(
            after.stdout, original.stdout,
            "{} {dialect}: loader output\n{rewritten}",
            case.name
        );
        assert_eq!(
            first_error(&after),
            first_error(&original),
            "{} {dialect}: loader error",
            case.name
        );
    }
    for domain in ExecutionDomain::ALL {
        for case in vectors(domain) {
            let source = case.script();
            let original = run_file(interpreter, &source, &case.id);
            let rewritten = optimise(&source, dialect);
            let after = run_file(interpreter, &rewritten, &case.id);
            assert_eq!(
                after.exit_code, original.exit_code,
                "{} {dialect}: changed status",
                case.id
            );
            assert_eq!(
                after.stdout, original.stdout,
                "{} {dialect}: changed output\n{rewritten}",
                case.id
            );
            assert_eq!(
                first_error(&after),
                first_error(&original),
                "{} {dialect}: changed error\n{rewritten}",
                case.id
            );
        }
    }
}

#[test]
fn source_rewrites_preserve_all_available_c_tcl_observations() {
    let references = available_tclshs();
    assert!(!references.is_empty(), "configure the C Tcl oracle matrix");
    for reference in references {
        let dialect = format!("tcl{}", reference.version.version_string());
        for case in rewrite_cases() {
            assert_fixture(case, &reference.path, &dialect, false);
        }
        assert_shared_vectors(&reference.path, &dialect);
    }
}

#[test]
fn source_rewrites_preserve_current_jim_observations() {
    let reference = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("valid Jim override")
    };
    let Some(reference) = reference else {
        eprintln!("Jim oracle unavailable and not requested");
        return;
    };
    for case in rewrite_cases() {
        assert_fixture(case, &reference.path, "jim", true);
    }
    assert_shared_vectors(&reference.path, "jim");
}

/// Equivalence alone could pass if a proof regression withdrew every rewrite.
/// This control requires a concrete optimisation and then executes it in each
/// native interpreter under that interpreter's exact dialect.
#[test]
fn proved_constant_rewrite_changes_source_and_preserves_native_result() {
    let source = "proc p {} {expr {2 + 3}}\nputs [p]\n";
    let mut references: Vec<_> = available_tclshs()
        .into_iter()
        .map(|reference| {
            let dialect = format!("tcl{}", reference.version.version_string());
            (reference.path, dialect)
        })
        .collect();
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().expect("required current Jim"))
    } else {
        locate_jimsh().expect("valid Jim override")
    };
    if let Some(reference) = jim {
        references.push((reference.path, "jim".to_owned()));
    }
    assert!(!references.is_empty(), "configure the native oracle matrix");
    for (interpreter, dialect) in references {
        let rewritten = optimise(source, &dialect);
        assert_ne!(rewritten, source, "{dialect}: proved rewrite withdrawn");
        let outcome = run_file(&interpreter, &rewritten, "positive-constant");
        assert!(outcome.success(), "{dialect}: rewritten error {outcome:?}");
        assert_eq!(outcome.stdout, b"5\n", "{dialect}: {rewritten}");
    }
}
