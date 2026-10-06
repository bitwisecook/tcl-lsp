// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The actual native engines define the storage contract, independently of Rust.

use tcl_dialect::VariableContainerModel;
use tcl_test_support::variable_containers::{
    variable_container_expectations, variable_container_scripts,
};
use tcl_test_support::{available_tclshs, locate_jimsh, require_jimsh, run_script_fixture};

fn check(path: &std::path::Path, wanted: &str, label: &str) -> usize {
    let scripts = variable_container_scripts();
    let expectations: Vec<_> = wanted.lines().collect();
    assert_eq!(scripts.len(), expectations.len(), "complete {label} column");
    for (index, (script, expected)) in scripts.iter().zip(expectations).enumerate() {
        let source = format!("puts [eval {{{script}}}]\n");
        let output = run_script_fixture(path, &source, &[])
            .unwrap()
            .strict_text()
            .unwrap();
        assert_eq!(output.trim(), expected, "{label} observation {index}");
    }
    scripts.len()
}

#[test]
fn actual_native_variable_container_observations() {
    let references = available_tclshs();
    let _ = references
        .first()
        .expect("configure shared C Tcl oracle overrides");
    let mut count = 0;
    for reference in references {
        let wanted = variable_container_expectations(
            VariableContainerModel::DistinctArray,
            Some(reference.version),
        )
        .unwrap();
        count += check(&reference.path, wanted, &reference.patchlevel);
    }
    let jim = if std::env::var_os("TCL_LSP_REQUIRE_JIM_ORACLE").is_some() {
        Some(require_jimsh().unwrap())
    } else {
        locate_jimsh().unwrap()
    };
    if let Some(reference) = jim {
        let wanted =
            variable_container_expectations(VariableContainerModel::DictionaryValue, None).unwrap();
        count += check(&reference.path, wanted, &reference.patchlevel);
    }
    eprintln!("{count} actual native variable-container observations passed");
}
