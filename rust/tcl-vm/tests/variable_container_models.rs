// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent native vectors for variable roots, elements, aliases and scope writes.

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_vm::Vm;

#[test]
fn selected_container_models_match_native_root_and_alias_vectors() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).unit_profile();
        let expected = tcl_test_support::variable_containers::variable_container_expectations(
            profile.variable_container_model().unwrap(),
            Some(profile.vm_runtime_version),
        )
        .unwrap();
        for (index, (source, expected)) in
            tcl_test_support::variable_containers::variable_container_scripts()
                .iter()
                .zip(expected.lines())
                .enumerate()
        {
            let mut vm = Vm::default();
            vm.set_dialect_profile(profile);
            vm.set_compiler(Box::new(BytecodeCompileService::default()));
            checked += 1;
            match vm.try_eval_source(source) {
                Ok(result) if result.code.is_ok() && result.result.to_str().as_ref() == expected => {}
                Ok(result) => failures.push(format!("{dialect} vector {index}: code {:?}, got {:?}, expected {expected:?}: {source}", result.code, result.result.to_str())),
                Err(error) => failures.push(format!("{dialect} vector {index}: {error:?}: {source}")),
            }
        }
    }
    assert_eq!(checked, 60);
    assert!(
        failures.is_empty(),
        "{} of {checked} vectors failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn dictionary_element_aliases_follow_replacements_and_values_copy_on_write() {
    let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
    for (source, expected) in [
        (
            "array set a(k) {inner VALUE};list $a [set a(k)]",
            "{k {inner VALUE}} {inner VALUE}",
        ),
        (
            "set a {k {inner OLD}};upvar 0 a(k) link;array set link {inner NEW};list $a $link",
            "{k {inner NEW}} {inner NEW}",
        ),
        ("list [array exists ::env] [catch {set ::env} r]", "1 0"),
        (
            "set a ODD;list [catch {set a(k) NEW} r] $r",
            "1 {can't set \"a(k)\": variable isn't array}",
        ),
        (
            "set a {k OLD};set copy $a;set a(k) NEW;list $a $copy",
            "{k NEW} {k OLD}",
        ),
        (
            "set a {k OLD};upvar 0 a(k) link;set a {k NEW};set link CHANGED;list $link $a",
            "CHANGED {k CHANGED}",
        ),
        (
            "set a {k OLD};upvar 0 a(k) link;unset a;set link NEW;list $link $a",
            "NEW {k NEW}",
        ),
        (
            "set a {k OLD other KEEP};unset a(k);list $a [info exists a(k)]",
            "{other KEEP} 0",
        ),
    ] {
        let mut vm = Vm::default();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        let result = vm.try_eval_source(source).unwrap();
        assert!(result.code.is_ok(), "{source}: {}", result.result.to_str());
        assert_eq!(result.result.to_str().as_ref(), expected, "{source}");
    }
}
