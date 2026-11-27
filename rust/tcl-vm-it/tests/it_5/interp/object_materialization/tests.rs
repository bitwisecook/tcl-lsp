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

//! Lists read and written by an authored 8.4 realm and the 9.0 host realm
//! of one iRules simulation, against tclsh 8.4.20 and 9.0.4.

use crate::prelude::*;
use tcl_dialect::TclVersion;

fn answer(completion: &tcl_runtime_api::Completion<Value>, script: &str) -> String {
    assert_eq!(completion.code, tcl_runtime_api::Code::Ok, "{script}");
    completion.result.try_to_str().unwrap().to_string()
}

/// The iRules simulation over a Tcl 9 engine, every authored provider set.
fn simulation() -> Vm {
    use tcl_native_model::{
        LogicalExpressionParseProvider, LogicalExpressionQuoteProvider, LogicalSourceWordProvider,
    };
    let host = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = Vm::with_native_core(
        Box::new(std::io::sink()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        host,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let profile = tcl_dialect::DialectProfile::irules();
    vm.set_dialect_profile(profile);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    assert!(vm.set_logical_eval_object_provider(
        tcl_native_model::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_source_word_provider(LogicalSourceWordProvider::Tcl84CoreSimulation));
    assert!(vm.set_logical_expression_parse_provider(
        LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_quote_provider(LogicalExpressionQuoteProvider::Tcl84CoreSimulation));
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_compiled_variable_provider(
        tcl_native_model::native_compiled_variables::LogicalCompiledVariableProvider::Tcl84CoreSimulation
    ));
    vm
}

#[test]
fn one_proc_reads_and_writes_its_lists_under_one_recipe_in_each_realm() {
    // Expected forms are tclsh 8.4.20 and 9.0.4 output for the same script.
    const DEFINE: &str = "proc p {args} {lindex $args 0; linsert $args 0 x}
        proc q {args} {list [lindex $args 0] [linsert $args 0 x]}";
    const CASES: &[(&str, &str, &str)] = &[
        ("p #c d", "x #c d", "x #c d"),
        ("q #c d", "#c {x #c d}", "{#c} {x #c d}"),
        ("list #a b", "#a b", "{#a} b"),
        ("lindex {a\\x4142 b} 0", "aB", "aA42"),
        ("linsert {a\\x4142 b} 0 x", "x aB b", "x aA42 b"),
    ];
    let mut vm = simulation();
    answer(&vm.try_eval_source(DEFINE).unwrap(), DEFINE);
    let mut framework = simulation();
    answer(
        &framework.try_eval_native_host_source(DEFINE).unwrap(),
        DEFINE,
    );
    let mut differences = Vec::new();
    for (script, tcl84, tcl90) in CASES {
        let authored = answer(&vm.try_eval_source(script).unwrap(), script);
        let host = answer(
            &framework.try_eval_native_host_source(script).unwrap(),
            script,
        );
        if (authored.as_str(), host.as_str()) != (*tcl84, *tcl90) {
            differences.push(format!("{script}: {authored:?} / {host:?}"));
        }
    }
    assert_eq!(differences, Vec::<String>::new());
}

#[test]
fn a_list_sealed_by_one_realm_is_refused_by_the_other() {
    let mut vm = simulation();
    answer(
        &vm.try_eval_native_host_source("set ::crossing [list a b]; llength $::crossing")
            .unwrap(),
        "host list",
    );
    let crossed = vm.try_eval_source("lindex $::crossing 0");
    assert!(
        !crossed
            .as_ref()
            .is_ok_and(|completion| completion.code == tcl_runtime_api::Code::Ok),
        "an authored reader cannot reparse a host-sealed list"
    );
}

#[test]
fn a_framework_read_of_an_authored_frame_uses_the_frame_names() {
    let host = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = simulation();
    assert!(vm.register_framework_builtin("::host_uplevel", "uplevel", host));
    let script = "proc p {} {set x 1; set y [::host_uplevel 0 {set x}]; incr x; list $x $y}
        p";
    assert_eq!(answer(&vm.try_eval_source(script).unwrap(), script), "2 1");
}
