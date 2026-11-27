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

//! The authored 8.4 `package` table on an iRules simulation, beside the
//! host's own.

use crate::prelude::*;
use tcl_registry::native_package::AuthoredPackageProvider;

fn host() -> Vm {
    let profile = tcl_vm::test_support::profile("tcl9.0");
    let mut vm = Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("actual host core");
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    let _ = vm.set_command_surface_profile(profile);
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.set_logical_package_provider(Some(AuthoredPackageProvider::Tcl84Core)));
    vm
}

#[track_caller]
fn text(completion: Completion<Value>) -> String {
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    let text = completion
        .result
        .try_to_str()
        .expect("ASCII package control")
        .to_string();
    drop(completion);
    text
}

fn package(vm: &mut Vm, words: &[&str]) -> Completion<Value> {
    let mut argv = vec![Value::string("package")];
    argv.extend(words.iter().map(|word| Value::string(*word)));
    crate::fixture::call(vm, "::tmm::_logical_package", &argv)
}

fn host_version(vm: &mut Vm, name: &str) -> String {
    text(
        vm.try_eval_native_host_source(&format!("package provide {name}"))
            .expect("native host activation"),
    )
}

#[test]
fn original_authored_namespace_selector_keeps_table_members_and_strict_utf8_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = host();
    let original = Value::string("ex");
    let table = Value::list(vec![Value::string("exists"), Value::string("eval")]);
    let reached = crate::fixture::call(
        &mut vm,
        "::tmm::_logical_namespace_member",
        &[original, table],
    );
    assert_eq!(reached.code, Code::Ok);
    assert_eq!(reached.result.string_bytes().as_ref(), b"exists");
    let raw = Value::new_native_string_bytes(b"\xff".as_slice());
    let table = Value::list(vec![raw]);
    assert!(
        vm.try_invoke_command(
            "::tmm::_logical_namespace_member",
            &[Value::string("x"), table]
        )
        .is_err()
    );
}

#[test]
fn authored_package_members_mutate_only_the_isolated_table() {
    let mut vm = host();
    assert_eq!(text(package(&mut vm, &["prov", "Tcl"])), "8.4");
    assert_eq!(
        text(package(&mut vm, &["req", "-exact", "Tcl", "8.4"])),
        "8.4"
    );
    assert!(host_version(&mut vm, "Tcl").starts_with("9.0"));
    assert_eq!(text(package(&mut vm, &["provi", "Demo", "1.2"])), "");
    assert_eq!(text(package(&mut vm, &["pres", "Demo"])), "1.2");
    assert_eq!(host_version(&mut vm, "Demo"), "");
    assert_eq!(text(package(&mut vm, &["forget", "Tcl"])), "");
    assert_eq!(text(package(&mut vm, &["provide", "Tcl"])), "");
    assert!(host_version(&mut vm, "Tcl").starts_with("9.0"));
    let ambiguous = package(&mut vm, &["pr"]);
    assert_eq!(ambiguous.code, Code::Error);
    assert!(
        ambiguous
            .result
            .try_to_str()
            .unwrap()
            .contains("ambiguous option")
    );
    let arity = package(&mut vm, &["vsat", "1.2", "1.0", "2.0"]);
    assert_eq!(arity.code, Code::Error);
    assert_eq!(
        arity.result.try_to_str().unwrap().as_ref(),
        "wrong # args: should be \"package vsatisfies version1 version2\""
    );
}

#[test]
fn authored_package_loader_requires_independent_script_dispatch_provider() {
    let mut vm = host();
    assert_eq!(
        text(package(
            &mut vm,
            &[
                "ifneeded",
                "Demo",
                "1.2",
                "::tmm::_logical_package package provide Demo 1.2"
            ]
        )),
        ""
    );
    let mut argv = vec![Value::string("package")];
    argv.extend(["require", "Demo"].map(Value::string));
    assert!(
        vm.try_invoke_command("::tmm::_logical_package", &argv)
            .is_err()
    );
    assert_eq!(host_version(&mut vm, "Demo"), "");
    assert_eq!(text(package(&mut vm, &["provide", "Demo"])), "");
}

#[test]
fn authored_package_loaders_children_and_host_activation_are_separate() {
    let mut vm = host();
    assert!(vm.set_logical_eval_object_provider(
        tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
    ));
    assert_eq!(
        text(package(
            &mut vm,
            &[
                "ifn",
                "Demo",
                "1.2",
                "::tmm::_logical_package package provide Demo 1.2"
            ]
        )),
        ""
    );
    assert_eq!(text(package(&mut vm, &["versions", "Demo"])), "1.2");
    assert_eq!(text(package(&mut vm, &["req", "Demo"])), "1.2");
    assert_eq!(host_version(&mut vm, "Demo"), "");
    assert_eq!(
        text(package(
            &mut vm,
            &["unknown", "::tmm::_logical_package package provide"]
        )),
        ""
    );
    assert_eq!(
        text(package(&mut vm, &["unknown"])),
        "::tmm::_logical_package package provide"
    );
    let native = vm
        .try_eval_native_host_source("::tmm::_logical_package package provide Tcl")
        .expect("native host activation");
    assert!(text(native).starts_with("9.0"));
    assert_eq!(text(package(&mut vm, &["provide", "Tcl"])), "8.4");
    let child = vm
        .try_eval_native_host_source(
            "interp create worker; list \
            [worker eval {::tmm::_logical_package package provide Tcl}] \
            [worker eval {::tmm::_logical_package package provide Demo}] \
            [worker eval {::tmm::_logical_package package unknown}]",
        )
        .expect("native host activation");
    assert_eq!(child.code, Code::Ok, "{child:?}");
    assert_eq!(child.result.to_str().as_ref(), "8.4 {} {}");
    assert!(vm.set_logical_package_provider(None));
    assert_eq!(
        vm.try_invoke_command("::tmm::_logical_package", &[Value::string("package")])
            .unwrap_err()
            .to_string(),
        "authored package provider is unavailable"
    );
}
