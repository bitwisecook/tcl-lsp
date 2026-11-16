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

//! The stock command identities a native compilation entry retains: renames, ensembles, bootstrap maps, observers and procedure cache keys.

use super::*;

#[test]
fn native_entry_retains_stock_identity_after_qualified_rename() {
    use tcl_runtime_api::native_compilation::NativeCommandImplementation;
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert!(
        vm.eval_source("namespace eval N {}; rename set N::moved")
            .unwrap()
            .code
            .is_ok()
    );
    let entry = vm
        .native_compilation_entry_for_namespace("N", false)
        .unwrap();
    assert!(entry.closed);
    let moved = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "moved" && row.slot.namespace == ["N".to_owned()])
        .expect("qualified moved stock command");
    assert!(matches!(&moved.implementation,
            NativeCommandImplementation::Registry { identity, .. } if identity == "set"));
    assert!(
        vm.command_binding_matches(&tcl_runtime_api::CommandBindingIdentity::in_namespace(
            "N", "moved", "set"
        ))
    );
}

#[test]
fn native_entry_preserves_unmaterialised_unknown_handler_prefix() {
    let mut vm = Vm::new();
    vm.declare_namespace_key("N");
    let namespace = vm.intern_ns("N");
    let number = Value::double(1.0 / 3.0);
    let prefix = vec![Value::string("handler"), number.clone()];
    let protocol = vm
        .native_invocation_dialect()
        .native_string_materialization(None)
        .unwrap()
        .protocol();
    vm.ns_unknowns.insert(
        namespace,
        NativeUnknownHandler {
            root: Value::native_list_constructor(prefix, protocol),
            length: 2,
        },
    );

    let entry = vm
        .native_compilation_entry_for_namespace("N", false)
        .unwrap();
    let captured = entry
        .namespaces
        .iter()
        .find(|row| row.token == u64::from(namespace.0))
        .expect("namespace remains present at compilation entry");
    assert_eq!(
        captured.unknown_handler,
        Some(vec![Some(NameBytes::from("handler")), None])
    );
    assert!(number.existing_string_representation().is_none());
    assert_eq!(number.double_representation(), Some(1.0 / 3.0));
    assert!(
        entry
            .namespaces
            .iter()
            .any(|row| row.token != captured.token && row.unknown_handler.is_none())
    );
}

#[test]
fn fixed_math_admission_retains_actual_registration_and_owner_identity() {
    use tcl_runtime_api::native_compilation::{
        NativeMathFunctionPrerequisite, NativeMathFunctionResolution,
    };
    let mut vm = crate::native_fixture::interpreter(release_profile("tcl8.4"));
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    let table = entry
        .math_functions
        .expect("C8.4 uses a fixed native math table");
    let NativeMathFunctionResolution::Present(abs) = table.lookup("abs") else {
        panic!("registered abs")
    };
    assert_eq!(abs.arity, Some(1));
    let asm = FunctionAsm {
        native_math_table_prerequisite: Some(NativeMathFunctionPrerequisite {
            interpreter: entry.interpreter,
            table,
        }),
        ..Default::default()
    };
    assert!(vm.native_math_table_prerequisite_matches(&asm));
    // A same-spelled Tcl handler replacement is a different lookup table.
    vm.register_guarded_builtin("::tcl::mathfunc::abs", guarded_builtin, GUARDED_IDENTITY);
    assert!(vm.native_math_table_prerequisite_matches(&asm));
    vm.register_stock_builtin("::tcl::mathfunc::abs", guarded_builtin);
    assert!(!vm.native_math_table_prerequisite_matches(&asm));
    assert!(matches!(
        vm.try_run_function(&asm),
        Err(tcl_runtime_api::NativeExecutionError::CompilationAdmission(
            tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired
        ))
    ));
    let other = crate::native_fixture::interpreter(release_profile("tcl8.4"));
    assert!(!other.native_math_table_prerequisite_matches(&asm));
}

#[test]
fn retained_stock_ensemble_keeps_its_actual_worker_generations() {
    let mut vm = Vm::new();
    let root = vm.registered_command_token_bytes(b"string").unwrap();
    let length = vm
        .registered_command_token_bytes(b"::tcl::string::length")
        .unwrap();
    let repeat = vm
        .registered_command_token_bytes(b"::tcl::string::repeat")
        .unwrap();
    let tokens = vm.registered_stock_implementation_tokens(std::slice::from_ref(&root));
    assert!(tokens.contains(&root));
    assert!(tokens.contains(&length));
    assert!(tokens.contains(&repeat));
    vm.retain_registered_command_tokens(&tokens);
    assert_eq!(
        vm.registered_command_token_bytes(b"::tcl::string::length"),
        Some(length)
    );
    assert_eq!(
        vm.try_invoke_command(
            "string",
            &[Value::string("repeat"), Value::string("ab"), Value::int(3)]
        )
        .unwrap()
        .result
        .string_bytes()
        .as_ref(),
        b"ababab"
    );
    assert!(vm.registered_command_token_bytes(b"open").is_none());

    let mut other = Vm::new();
    assert_eq!(
        other.registered_stock_implementation_tokens(&tokens),
        tokens
    );
    other.register("string", guarded_builtin);
    let replacement = other.registered_command_token_bytes(b"string").unwrap();
    assert_eq!(
        other.registered_stock_implementation_tokens(std::slice::from_ref(&replacement)),
        vec![replacement]
    );
}

#[test]
fn retained_expression_keeps_stock_workers_without_replacement_or_random_grants() {
    let mut vm = Vm::new();
    let root = vm.registered_command_token_bytes(b"expr").unwrap();
    let sin = vm
        .registered_command_token_bytes(b"::tcl::mathfunc::sin")
        .unwrap();
    let rand = vm
        .registered_command_token_bytes(b"::tcl::mathfunc::rand")
        .unwrap();
    vm.register_guarded_builtin("::tcl::mathfunc::abs", guarded_builtin, GUARDED_IDENTITY);
    let replacement = vm
        .registered_command_token_bytes(b"::tcl::mathfunc::abs")
        .unwrap();
    let tokens = vm.registered_stock_implementation_tokens(std::slice::from_ref(&root));
    assert!(tokens.contains(&sin));
    assert!(!tokens.contains(&rand));
    assert!(!tokens.contains(&replacement));
    vm.retain_registered_command_tokens(&tokens);
    assert_eq!(
        vm.registered_command_token_bytes(b"::tcl::mathfunc::sin"),
        Some(sin)
    );
    assert!(
        vm.registered_command_token_bytes(b"::tcl::mathfunc::rand")
            .is_none()
    );
    assert!(
        vm.registered_command_token_bytes(b"::tcl::mathfunc::abs")
            .is_none()
    );
}

#[test]
fn native_array_bootstrap_retains_actual_ensemble_and_private_hook_identity() {
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCompilerHookPresence,
    };
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let vm = crate::native_fixture::interpreter(release_profile(name));
        let ensemble = vm
            .native_invocation_dialect()
            .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Array)
            .is_some();
        assert_eq!(
            matches!(vm.commands.get("array"), Some(Command::Ensemble(_))),
            ensemble,
            "{name}"
        );
        let entry = vm
            .native_compilation_entry_for_namespace("", false)
            .unwrap();
        let worker = entry
            .commands
            .iter()
            .find(|row| row.slot.namespace == ["tcl", "array"] && row.slot.simple == "get");
        assert_eq!(worker.is_some(), ensemble, "{name}");
        assert_eq!(vm.namespace_exists("::tcl::array"), ensemble, "{name}");
        if let Some(worker) = worker {
            assert!(
                matches!(&worker.implementation,
                    NativeCommandImplementation::Registry { identity, .. }
                    if identity == "tcl::array::get"),
                "{name}"
            );
            assert_eq!(
                worker.compiler_hook,
                NativeCompilerHookPresence::Present,
                "{name}"
            );
            assert!(worker.compiler.is_some(), "{name}");
        }
    }
    let mut vm = Vm::new();
    vm.register("::tcl::array::get", guarded_builtin);
    select_release(&mut vm, "tcl8.4");
    assert!(vm.commands.contains_key("tcl::array::get"));
    assert_eq!(vm.stock_native_identity("::tcl::array::get"), None);
    assert!(vm.namespace_exists("::tcl::array"));
    let mut vm = Vm::new();
    vm.declare_namespace_key("tcl::array::user");
    select_release(&mut vm, "tcl8.4");
    assert!(vm.namespace_exists("::tcl::array::user"));
}

#[test]
fn native_string_and_dict_bootstrap_export_the_actual_private_maps() {
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let vm = crate::native_fixture::interpreter(release_profile(name));
        let entry = vm
            .native_compilation_entry_for_namespace("", false)
            .unwrap();
        for (family, root, member) in [
            (
                tcl_registry::EnsembleImplementationFamily::String,
                "string",
                "length",
            ),
            (
                tcl_registry::EnsembleImplementationFamily::Dict,
                "dict",
                "get",
            ),
        ] {
            let namespace = vm
                .native_invocation_dialect()
                .ensemble_implementation_namespace(family);
            let worker = entry
                .commands
                .iter()
                .find(|row| row.slot.namespace == ["tcl", root] && row.slot.simple == member);
            assert_eq!(worker.is_some(), namespace.is_some(), "{name}/{root}");
            let compiler = entry
                .commands
                .iter()
                .find(|row| row.slot.namespace.is_empty() && row.slot.simple == root)
                .and_then(|row| row.compiler.as_ref())
                .and_then(|compiler| compiler.ensemble.as_ref());
            assert_eq!(compiler.is_some(), namespace.is_some(), "{name}/{root}");
            if let Some(compiler) = compiler {
                assert!(compiler.prefixes);
                assert!(compiler.map.iter().any(|(key, value)| key == member
                    && value == &[Some(NameBytes::from(format!("::tcl::{root}::{member}")))]));
                let worker = worker.unwrap();
                assert_eq!(
                    worker.compiler_hook,
                    NativeCompilerHookPresence::Present,
                    "{name}/{root}"
                );
                assert!(worker.compiler.is_some());
            }
        }
    }
    let mut vm = Vm::new();
    vm.register("::tcl::string::length", guarded_builtin);
    select_release(&mut vm, "tcl8.5");
    assert_eq!(vm.stock_native_identity("::tcl::string::length"), None);
    vm.register("append", guarded_builtin);
    select_release(&mut vm, "tcl8.4");
    assert_eq!(vm.stock_native_identity("append"), None);
    assert!(vm.commands.contains_key("tcl::string::length"));
}

#[test]
fn native_namespace_bootstrap_retains_actual_worker_and_profile_ownership() {
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCompilerHookPresence,
    };
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let vm = crate::native_fixture::interpreter(release_profile(name));
        let ensemble = vm
            .native_invocation_dialect()
            .ensemble_implementation_namespace(
                tcl_registry::EnsembleImplementationFamily::Namespace,
            )
            .is_some();
        assert_eq!(
            matches!(vm.commands.get("namespace"), Some(Command::Ensemble(_))),
            ensemble,
            "{name}"
        );
        let entry = vm
            .native_compilation_entry_for_namespace("", false)
            .unwrap();
        let worker = entry
            .commands
            .iter()
            .find(|row| row.slot.namespace == ["tcl", "namespace"] && row.slot.simple == "current");
        assert_eq!(worker.is_some(), ensemble, "{name}");
        assert_eq!(vm.namespace_exists("::tcl::namespace"), ensemble, "{name}");
        if let Some(worker) = worker {
            assert!(
                matches!(&worker.implementation,
                    NativeCommandImplementation::Registry { identity, .. }
                    if identity == "tcl::namespace::current"),
                "{name}"
            );
            assert_eq!(
                worker.compiler_hook,
                NativeCompilerHookPresence::Present,
                "{name}"
            );
            assert!(worker.compiler.is_some(), "{name}");
        }
    }
    let mut vm = Vm::new();
    vm.register("::tcl::namespace::current", guarded_builtin);
    select_release(&mut vm, "tcl8.4");
    assert!(vm.commands.contains_key("tcl::namespace::current"));
    assert_eq!(vm.stock_native_identity("::tcl::namespace::current"), None);
    assert!(vm.namespace_exists("::tcl::namespace"));
    let mut vm = Vm::new();
    vm.declare_namespace_key("tcl::namespace::user");
    select_release(&mut vm, "tcl8.4");
    assert!(vm.namespace_exists("::tcl::namespace::user"));
}

#[test]
fn native_entry_observer_closure_is_independent_of_values_and_command_closure() {
    use tcl_runtime_api::native_compilation::NativeVariableObserverPresence;
    let mut vm = crate::native_fixture::interpreter(release_profile("tcl8.4"));
    let initial = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(
        initial.variable_observers,
        NativeVariableObserverPresence::BoundedNativeOnly
    );
    assert!(initial.variable_observers.permits_no_callbacks());
    vm.add_var_trace(
        "::count",
        vec!["write".to_owned()],
        "callback".to_owned(),
        false,
    );
    let observed = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(
        observed.variable_observers,
        NativeVariableObserverPresence::Present
    );
    assert!(!observed.variable_observers.permits_no_callbacks());
    assert_ne!(
        initial, observed,
        "actual entry cache key retains observer registration"
    );
    vm.remove_var_trace("::count", &["write".to_owned()], "callback");
    assert_eq!(
        vm.native_compilation_entry_for_namespace("", false)
            .unwrap()
            .variable_observers,
        NativeVariableObserverPresence::BoundedNativeOnly
    );
    assert_eq!(
        crate::native_fixture::interpreter(release_profile("jim"))
            .native_compilation_entry_for_namespace("", false)
            .unwrap()
            .variable_observers,
        NativeVariableObserverPresence::Absent
    );
    assert!(!NativeVariableObserverPresence::Unknown.permits_no_callbacks());
}

#[test]
fn native_entry_trusts_registered_stock_identity_and_keeps_host_builtins_opaque() {
    use tcl_runtime_api::native_compilation::NativeCommandImplementation;
    let mut vm = Vm::new();
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    let stock = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "set" && row.slot.namespace.is_empty())
        .expect("stock set entry");
    assert!(matches!(&stock.implementation,
            NativeCommandImplementation::Registry { identity, .. } if identity == "set"));
    vm.register_guarded_builtin("set", guarded_builtin, GUARDED_IDENTITY);
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    let custom = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "set" && row.slot.namespace.is_empty())
        .expect("custom set entry");
    assert_eq!(custom.implementation, NativeCommandImplementation::Opaque);
    assert!(
        !vm.command_binding_matches(&tcl_runtime_api::CommandBindingIdentity::in_namespace(
            "", "set", "set"
        ))
    );
}

#[test]
fn ensemble_compiler_export_does_not_donate_modified_handler_identity() {
    use tcl_runtime_api::native_compilation::NativeCommandImplementation;
    let mut vm = Vm::new();
    let Command::Ensemble(token) = vm.commands.get("info").unwrap().clone() else {
        panic!("native info ensemble")
    };
    let number = Value::double(1.0 / 3.0);
    let mut config = token.config();
    let original_compiler = config.native.as_ref().unwrap().identity;
    config.map = vec![(
        NameBytes::from("exists"),
        vec![
            Some(NameBytes::from("::replacement")),
            number
                .resident_string_bytes()
                .map(|bytes| NameBytes::from(bytes.as_ref())),
        ],
    )];
    token.configure(config);
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    let binding = entry
        .commands
        .iter()
        .find(|binding| binding.slot.simple == "info" && binding.slot.namespace.is_empty())
        .unwrap();
    assert_eq!(binding.implementation, NativeCommandImplementation::Opaque);
    let compiler = binding.compiler.as_ref().unwrap();
    assert_eq!(compiler.registry_identity, original_compiler);
    assert_eq!(
        compiler.ensemble.as_ref().unwrap().map,
        vec![(
            NameBytes::from("exists"),
            vec![Some(NameBytes::from("::replacement")), None]
        )]
    );
    assert!(number.existing_string_representation().is_none());
    let required = tcl_runtime_api::native_compilation::NativeEnsembleCompilerPrerequisite {
        interpreter: entry.interpreter,
        lookup_namespace_token: entry.current_namespace,
        invocation_word: "info".into(),
        slot: binding.slot.clone(),
        namespace_token: binding.namespace_token,
        token: binding.token,
        implementation_generation: binding.implementation_generation,
        compiler: compiler.clone(),
        selected_worker: None,
        nested_compilers: Vec::new(),
        guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
    };
    assert!(vm.native_ensemble_compiler_prerequisite_matches(&required));
    let mut config = token.config();
    config.parameters.push(NameBytes::from("argument"));
    token.configure(config);
    assert!(!vm.native_ensemble_compiler_prerequisite_matches(&required));
    vm.register("info", guarded_builtin);
    assert!(!vm.native_ensemble_compiler_prerequisite_matches(&required));
}

#[test]
fn public_handler_registration_cannot_acquire_stock_compiler_provenance() {
    use tcl_runtime_api::native_compilation::{
        NativeCommandImplementation, NativeCompilerHookPresence,
    };
    for registration in 0..3 {
        let mut vm = Vm::new();
        match registration {
            0 => vm.register("set", guarded_builtin),
            1 => vm.register_guarded_builtin("set", guarded_builtin, GUARDED_IDENTITY),
            _ => {
                let spec = tcl_registry::CommandRegistry::build_default();
                vm.register_spec_builtin(
                    spec.get("set").expect("stock descriptor"),
                    guarded_builtin,
                );
            }
        }
        let entry = vm
            .native_compilation_entry_for_namespace("", false)
            .unwrap();
        let custom = entry
            .commands
            .iter()
            .find(|row| row.slot.simple == "set" && row.slot.namespace.is_empty())
            .expect("registered host handler");
        assert_eq!(custom.implementation, NativeCommandImplementation::Opaque);
        assert_eq!(custom.compiler_hook, NativeCompilerHookPresence::Absent);
        assert!(custom.compiler.is_none());
        assert!(!vm.command_binding_matches(
            &tcl_runtime_api::CommandBindingIdentity::in_namespace("", "set", "set")
        ));
    }
    let mut vm = Vm::new();
    vm.set_runtime_version(tcl_dialect::TclVersion::V8_4);
    vm.register("const", guarded_builtin);
    assert!(
        vm.lookup_command("const").is_some(),
        "a host callable has its own availability"
    );
    vm.register_native_command("host", Rc::new(TestNative));
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    let host = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "host")
        .unwrap();
    assert_eq!(host.implementation, NativeCommandImplementation::Opaque);
    assert_eq!(host.compiler_hook, NativeCompilerHookPresence::Absent);
}

#[test]
fn procedure_cache_keys_preserve_constructed_literal_colon_namespaces() {
    let global_provenance = ProcedureProvenance {
        name: "::p".to_owned(),
        namespace_context: Some(tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(
            NamespacePath::root(),
        )),
        parameters: String::new(),
        body: "return SAME".to_owned(),
    };
    let colon_provenance = ProcedureProvenance {
        // Root `::` + literal-colon namespace `:` + separator `::` + p.
        name: ":::::p".to_owned(),
        namespace_context: Some(tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(
            NamespacePath::from_segments([":"]),
        )),
        parameters: String::new(),
        body: "return SAME".to_owned(),
    };
    let global = ProcedureCacheKey::from_module_entry("::p", &global_provenance)
        .expect("global compiler key is valid");
    let colon = ProcedureCacheKey::from_module_entry(":::::p", &colon_provenance)
        .expect("literal-colon compiler key is valid");

    assert_eq!(global.name, "p");
    assert_eq!(colon.name, ":::p");
    assert_ne!(global, colon);
    assert_eq!(
        colon,
        ProcedureCacheKey::from_runtime_key(
            ":::p",
            "",
            "return SAME",
            &NamespacePath::from_segments([":"])
        )
    );

    let malformed = ProcedureProvenance {
        name: "::other".to_owned(),
        ..global_provenance
    };
    assert!(ProcedureCacheKey::from_module_entry("::p", &malformed).is_none());
}

#[test]
fn procedure_cache_keys_distinguish_raw_formal_lists_independently() {
    let body = "return SAME";
    assert_ne!(
        ProcedureCacheKey::from_runtime_key("p", "x", body, &NamespacePath::root()),
        ProcedureCacheKey::from_runtime_key("p", "{x default}", body, &NamespacePath::root()),
    );
}
