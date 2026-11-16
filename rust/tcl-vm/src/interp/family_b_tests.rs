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

use super::*;
use crate::command::NativeCommand;
use std::cell::Cell;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_runtime_api::GLOBAL_FRAME;

const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 41);

fn release_profile(name: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment(name).unit_profile()
}

/// The release's own engine and source compiler.
fn release_vm(version: tcl_dialect::TclVersion) -> Vm {
    crate::native_fixture::interpreter(release_profile(version.dialect_name()))
}

/// Re-select both the actual engine and the logical profile of `name`.
fn select_release(vm: &mut Vm, name: &str) {
    let profile = release_profile(name);
    assert!(vm.set_native_engine_profile(profile), "{name}");
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
}

/// Every capture of an empty literal world renews its observation, so
/// entries are compared without it.
fn without_literal_observation(
    mut entry: tcl_runtime_api::NativeCompilationEntry,
) -> tcl_runtime_api::NativeCompilationEntry {
    entry.empty_literal_world = None;
    entry
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn construction_without_a_native_engine_refuses() {
    let source = tcl_dialect::DialectProfile::irules();
    assert!(
        tcl_registry::InvocationDialect::of_profile(source)
            .native_bootstrap_protocol()
            .is_none()
    );
    assert!(
        Vm::with_native_core(
            Box::new(std::io::sink()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            source,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .is_none()
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn default_c_error_code_stays_private_until_return_options_are_requested() {
    for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let result = crate::return_options::apply(
            &mut vm,
            &[
                Value::string("-code"),
                Value::string("error"),
                Value::string("-level"),
                Value::int(0),
                Value::string("BODY"),
            ],
            tcl_cmd_core::return_options::ReturnOptionsPurpose::User,
        );
        assert_eq!(result.code, Code::Error, "{engine}");
        assert!(crate::command::opt_get(&result.options, "-errorcode").is_none());
        assert_eq!(
            vm.native_errors
                .primitive_error_code
                .as_ref()
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"NONE",
            "{engine}"
        );
        let options = vm.completion_options_snapshot(&result).unwrap();
        assert_eq!(
            crate::command::opt_get(&options, "-errorcode")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"NONE",
            "{engine}"
        );
    }
}

#[test]
fn procedure_publication_keeps_retained_holder_and_release_reparse() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(version);
        let original = vm.activate_namespace_operand(b"a:").unwrap();
        vm.push_ns_eval_token_frame(original, Vec::new());
        assert!(vm.native_procedure_holder_exists(b"p").unwrap());
        let (_, slot, activation) = vm.native_procedure_publication(b"p").unwrap();
        if version <= tcl_dialect::TclVersion::V8_5 {
            assert_ne!(activation, original);
            assert_eq!(vm.namespace_object_bytes(activation).as_bytes(), b"a");
        } else {
            assert_eq!(activation, original);
        }
        assert_eq!(slot.namespace, activation);
        assert_eq!(slot.simple.as_bytes(), b"p");
        vm.pop_ns();
        vm.pop_call_frame();
    }
}

#[test]
fn jim_actual_namespace_objects_are_in_native_snapshots() {
    let mut vm = crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"));
    let context = vm.activate_namespace_operand(b"n\0z").unwrap();
    vm.push_ns_eval_token_frame(context, Vec::new());
    let entry = vm.native_compilation_entry_for_namespace_token(context, true);
    assert_eq!(entry.current_namespace, u64::from(context.0));
    let row = entry
        .namespaces
        .iter()
        .find(|row| row.token == u64::from(context.0))
        .unwrap();
    assert_eq!(
        row.jim_namespace_object.as_ref().unwrap().as_bytes(),
        b"n\0z"
    );
    let namespace = vm
        .with_jim_current_namespace(|original| Ok(original.clone()))
        .unwrap();
    let scope = native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let canonical = vm
        .original_jim_canonical_namespace_object(
            &namespace,
            &Value::from_native_string_bytes(b"p".as_slice()),
            &|vm| scope.check(vm),
        )
        .unwrap();
    assert_eq!(
        canonical.resident_string_bytes().unwrap().as_ref(),
        b"n\0z::p"
    );
    scope.finish(&mut vm).unwrap();
    let (_, slot, body) = vm.native_procedure_publication(b"p").unwrap();
    assert_eq!(slot.namespace, ROOT_NS);
    assert_eq!(slot.simple.as_bytes(), b"n\0z::p");
    assert_eq!(body, context);
    vm.pop_ns();
    vm.pop_call_frame();
}

#[test]
fn native_command_byte_lookup_and_snapshot_keep_distinct_original_keys() {
    use tcl_core_types::NameBytes;
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(version);
        let names = [
            b"k".as_slice(),
            b"k\xff",
            b"k\xc3\xbf",
            b"k\xc0\x80z",
            b"k\xed\xa0\x80",
            b"k\xef\xbf\xbd",
        ];
        let keys: Vec<_> = names
            .iter()
            .map(|name| {
                vm.register_command_in_slot(
                    CommandSlot {
                        namespace: ROOT_NS,
                        simple: NameBytes::from(*name),
                    },
                    Command::Native(Rc::new(TestNative)),
                )
            })
            .collect();
        let mut tokens = HashSet::new();
        for (name, key) in names.iter().zip(&keys) {
            let token = Namespaces::find_command_bytes(&vm, ROOT_NS, name)
                .expect("exact native command key remains addressable");
            assert!(tokens.insert(token));
            assert_eq!(vm.command_display_key_bytes(key).as_bytes(), *name);
            let expected = tcl_syntax::naming::root_unrooted_key_bytes(name);
            assert_eq!(Namespaces::command_name_bytes(&vm, token), Some(expected));
        }
        assert_eq!(
            Namespaces::find_command_bytes(&vm, ROOT_NS, b"k\0ignored"),
            Namespaces::find_command_bytes(&vm, ROOT_NS, b"k")
        );
        let pure = Value::byte_array(b"k\xff".to_vec());
        let materialised = vm
            .native_name_operand_bytes(&pure)
            .expect("C byte-array materialisation");
        assert_eq!(materialised.as_ref(), b"k\xc3\xbf");
        assert_eq!(
            Namespaces::find_command_bytes(&vm, ROOT_NS, &materialised),
            Namespaces::find_command_bytes(&vm, ROOT_NS, b"k\xc3\xbf")
        );
        let entry = vm
            .native_compilation_entry_for_namespace("", false)
            .unwrap();
        for name in names {
            assert!(
                entry
                    .commands
                    .iter()
                    .any(|row| row.slot.simple.as_bytes() == name)
            );
        }
    }
}

#[test]
fn owned_command_queries_release_the_name_world_before_native_reentry() {
    struct RepublishCommand {
        slot: CommandSlot,
        entered: Rc<Cell<bool>>,
    }
    impl NativeCommand for RepublishCommand {
        fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
            // Dispatch must not retain a metadata guard while an embedder runs.
            assert!(vm.name_world.try_borrow_mut().is_ok());
            vm.register_command_in_slot(self.slot.clone(), Command::Native(Rc::new(TestNative)));
            self.entered.set(true);
            ok(Value::empty())
        }
    }
    let mut vm = Vm::new();
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"opaque\xff"),
    };
    let key = vm.register_command_in_slot(slot.clone(), Command::Native(Rc::new(TestNative)));
    let old = Namespaces::find_command_bytes(&vm, ROOT_NS, slot.simple.as_bytes()).unwrap();
    let retained_slot = vm.command_slot(&key).unwrap();
    let retained_display = vm.command_display_key_bytes(&key);
    let retained_generation = vm.visible_command_generation(&key).unwrap();
    let entered = Rc::new(Cell::new(false));
    vm.register_native_command(
        "republish",
        Rc::new(RepublishCommand {
            slot: slot.clone(),
            entered: Rc::clone(&entered),
        }),
    );
    assert_eq!(
        vm.try_invoke_command("republish", &[]).unwrap().code,
        Code::Ok
    );
    assert!(entered.get());
    assert_eq!(retained_slot, slot);
    assert_eq!(retained_display.as_bytes(), b"opaque\xff");
    assert_ne!(
        vm.visible_command_generation(&key),
        Some(retained_generation)
    );
    assert!(Namespaces::command_name_bytes(&vm, old).is_none());
    let replacement = Namespaces::find_command_bytes(&vm, ROOT_NS, b"opaque\xff").unwrap();
    assert_ne!(replacement, old);
    assert_eq!(
        Namespaces::command_name_bytes(&vm, replacement).unwrap(),
        b"::opaque\xff"
    );
}

#[test]
fn unrelated_command_traces_do_not_decode_opaque_command_names() {
    let mut vm = Vm::new();
    vm.register_native_command("watched", Rc::new(TestNative));
    assert_eq!(
        vm.add_cmd_trace(false, "watched", vec!["delete".into()], "unused".into())
            .code,
        Code::Ok
    );
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"opaque\xff"),
    };
    let key = vm.register_command_in_slot(slot.clone(), Command::Native(Rc::new(TestNative)));
    let generation = vm.visible_command_generation(&key).unwrap();
    let replacement = vm.register_command_in_slot(slot, Command::Native(Rc::new(TestNative)));
    assert_eq!(
        vm.command_display_key_bytes(&replacement).as_bytes(),
        b"opaque\xff"
    );
    assert_ne!(
        vm.visible_command_generation(&replacement),
        Some(generation)
    );
    assert!(vm.refused_completion().is_none());
    assert!(vm.name_world.try_borrow_mut().is_ok());
}

#[test]
fn embedder_command_enumeration_and_whitelists_preserve_exact_bytes() {
    let mut vm = Vm::new();
    vm.register_native_command("watched", Rc::new(TestNative));
    vm.register_command_in_slot(
        CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"opaque\xff"),
        },
        Command::Native(Rc::new(TestNative)),
    );
    assert!(vm.command_names().is_err());
    let world = Rc::clone(&vm.name_world);
    vm.retain_commands_bytes(&|name| {
        assert!(world.try_borrow_mut().is_ok());
        name == b"opaque\xff" || name == b"watched"
    });
    assert_eq!(
        vm.command_names_bytes(),
        vec![NameBytes::from(b"opaque\xff"), NameBytes::from(b"watched")]
    );
    assert_eq!(vm.retain_commands(&|name| name == "watched"), 1);
    assert_eq!(vm.command_names().unwrap(), vec!["watched"]);
}

#[test]
fn backing_catalogue_uses_exact_public_names_and_never_private_storage_keys() {
    // Software contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = Vm::new();
    vm.register_native_command("café", Rc::new(TestNative));
    let opaque_slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"opaque_\xff"),
    };
    let key = vm.register_command_in_slot(opaque_slot, Command::Native(Rc::new(TestNative)));
    assert!(key.starts_with("\0tcl-command:"));
    assert_eq!(
        vm.command_display_key_bytes(&key).as_bytes(),
        b"opaque_\xff"
    );
    let generation = vm.visible_command_generation(&key).unwrap();
    let report = vm.backing_report();
    assert!(
        report
            .iter()
            .any(|(name, backing)| name == "café" && *backing == RegisteredBacking::Builtin)
    );
    assert!(
        !report
            .iter()
            .any(|(name, _)| name.starts_with("\0tcl-command:") || name.starts_with("opaque_"))
    );
    assert!(Namespaces::find_command_bytes(&vm, ROOT_NS, b"opaque_\xff").is_some());
    assert_eq!(vm.visible_command_generation(&key), Some(generation));
    assert!(vm.name_world.try_borrow_mut().is_ok());
}

#[test]
fn reserved_command_slots_do_not_manufacture_live_generations() {
    let mut vm = Vm::new();
    let slot = CommandSlot {
        namespace: ROOT_NS,
        simple: NameBytes::from(b"reserved\xff"),
    };
    let key = vm.storage_key_for_command_slot(slot.clone(), slot.simple.as_bytes());
    assert_eq!(vm.command_slot(&key), Some(slot));
    assert_eq!(
        vm.command_display_key_bytes(&key).as_bytes(),
        b"reserved\xff"
    );
    assert_eq!(vm.live_command_generation(&key), None);
    assert_eq!(vm.visible_command_generation(&key), None);
    assert!(Namespaces::find_command_bytes(&vm, ROOT_NS, b"reserved\xff").is_none());
    vm.forget_command_storage_key(&key);
    assert!(vm.command_slot(&key).is_none());
    assert!(vm.name_world.try_borrow_mut().is_ok());
}

#[test]
fn byte_namespace_paths_keep_opaque_and_terminal_colon_token_contexts() {
    for version in tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(version);
        for segment in [b"N\xff".as_slice(), b"N\xc0\x80", b"a:"] {
            let path = NamespacePath::from_segments([segment]);
            vm.declare_namespace_path_with_origin(path.clone(), true);
            let namespace = vm.name_world.borrow().ns_intern[&path];
            let slot = CommandSlot {
                namespace,
                simple: tcl_core_types::NameBytes::from(b"p"),
            };
            vm.register_command_in_slot(slot, Command::Native(Rc::new(TestNative)));
            let command = Namespaces::find_command_bytes(&vm, namespace, b"p")
                .expect("local lookup keeps exact namespace token");
            let rooted = tcl_syntax::naming::root_unrooted_key_bytes(segment);
            assert_eq!(
                Namespaces::find_namespace_bytes(&vm, ROOT_NS, &rooted),
                Some(namespace)
            );
            assert_eq!(Namespaces::name_bytes(&vm, namespace), rooted);
            let mut reported = rooted;
            reported.extend_from_slice(b"::p");
            assert_eq!(Namespaces::command_name_bytes(&vm, command), Some(reported));
            let entry = vm.native_compilation_entry_for_namespace_token(namespace, false);
            assert!(
                entry
                    .namespaces
                    .iter()
                    .any(|row| row.token == u64::from(namespace.0) && row.path == path)
            );
        }
    }
}

#[test]
fn native_compilation_capture_refuses_missing_namespace_coordinates() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let root = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(root.current_namespace, u64::from(ROOT_NS.0));
    assert_eq!(
        root.retained_namespace_context(root.current_namespace)
            .unwrap()
            .path,
        NamespacePath::root()
    );
    let literal_world = root.empty_literal_world.as_ref().unwrap();
    assert!(literal_world.is_current());
    let absent = NamespacePath::from_segments([b"unallocated_capture_namespace".as_slice()]);
    assert!(vm.namespace_token_at_path(&absent, true).is_none());
    for error in [
        vm.native_compilation_entry_for_namespace("unallocated_capture_namespace", false)
            .unwrap_err(),
        vm.compile_module_bytes(
            &tcl_lexer::SourceImage::document("set value 1"),
            &absent,
            false,
        )
        .unwrap_err(),
    ] {
        assert_eq!(
            error.native_access_refusal(),
            Some(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "native compilation namespace coordinates"
                )
            )
        );
        assert!(error.guest_completion().is_none());
    }
    assert!(vm.namespace_token_at_path(&absent, true).is_none());
    assert!(literal_world.is_current());
    let current_root = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(current_root.current_namespace, root.current_namespace);
    assert_eq!(
        current_root
            .retained_namespace_context(current_root.current_namespace)
            .unwrap(),
        root.retained_namespace_context(root.current_namespace)
            .unwrap()
    );
}

#[test]
fn default_native_entry_retains_actual_execution_point_and_assistance_profile() {
    use tcl_dialect::model::DialectPoint;
    let vm = Vm::new();
    assert!(vm.dialect_profile().is_fallback());
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(entry.profile, vm.dialect_profile().cache_key());
    assert_eq!(
        entry.execution_point,
        Some(DialectPoint::for_tcl_version(vm.runtime_version()))
    );
    for version in tcl_dialect::TclVersion::ALL {
        assert_eq!(
            release_vm(version)
                .native_compilation_entry_for_namespace("", false)
                .unwrap()
                .execution_point,
            Some(DialectPoint::for_tcl_version(version))
        );
    }
}

#[test]
fn explicit_host_engine_keeps_source_logical_and_availability_axes_separate() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    vm.set_dialect_profile(source);
    let logical = vm.native_invocation_dialect();
    let lexical = vm.lexer_config();
    assert!(vm.set_command_surface_profile(host));
    assert!(!vm.set_native_engine_profile(source));
    let old_generation = vm.profile_generation();
    assert!(vm.set_native_engine_profile(host));
    assert_ne!(vm.profile_generation(), old_generation);
    assert_eq!(vm.dialect_profile().cache_key(), source.cache_key());
    assert_eq!(vm.native_invocation_dialect().numbers, logical.numbers);
    assert_eq!(
        vm.native_invocation_dialect().characters,
        logical.characters
    );
    assert_eq!(
        vm.native_invocation_dialect().arithmetic(),
        logical.arithmetic()
    );
    assert_eq!(
        vm.native_invocation_dialect().completion_options_policy(),
        logical.completion_options_policy()
    );
    assert_eq!(
        vm.lexer_config().grammar_over(source.grammar),
        lexical.grammar_over(source.grammar)
    );
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(entry.profile, source.cache_key());
    assert_eq!(
        entry.execution_point,
        tcl_registry::InvocationDialect::of_profile(host).execution_point()
    );
    assert_eq!(
        vm.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    let child = vm.fork_child_state();
    assert_eq!(child.source_profile().cache_key(), source.cache_key());
    assert_eq!(child.native_invocation_dialect().numbers, logical.numbers);
    assert_eq!(
        child.actual_native_execution_profile().cache_key(),
        host.cache_key()
    );
    // Explicit capability activation selects C9 policies only for that
    // host call, then restores the original F5 logical invocation.
    vm.active_native_profile = Some(host);
    assert_eq!(
        vm.native_invocation_dialect().numbers,
        tcl_registry::InvocationDialect::of_profile(host).numbers
    );
    vm.active_native_profile = None;
    assert_eq!(vm.native_invocation_dialect().numbers, logical.numbers);
}

#[test]
fn authored_numeric_provider_is_explicit_and_scoped_to_logical_activations() {
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    use tcl_syntax::value::ValueOps;
    let provider = AuthoredLogicalNumericSimulation::Tcl84Core;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(
        vm.native_invocation_dialect()
            .scalar_numeric_input_policy()
            .is_none()
    );
    assert_eq!(
        ValueOps::as_int(&mut vm, &Value::int(1)),
        Err(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)
    );
    let old_policy = vm.native_compiler_policy();
    assert!(vm.set_logical_numeric_provider(provider));
    assert_ne!(vm.native_compiler_policy(), old_policy);
    assert!(
        vm.native_invocation_dialect()
            .scalar_numeric_input_policy()
            .is_none()
    );
    let original = Value::string("077");
    assert_eq!(ValueOps::as_int(&mut vm, &original), Ok(63));
    assert!(original.integer_representation().is_none());
    assert_eq!(ValueOps::as_bool(&mut vm, &Value::string("2")), Ok(true));
    assert!(ValueOps::as_bool(&mut vm, &Value::string("NaN")).is_err());
    assert_eq!(
        ValueOps::as_bool(&mut vm, &Value::double(f64::NAN)),
        Ok(true)
    );
    assert!(
        crate::expr::authored_boolean(vm.numeric_context(), &Value::string("NaN"))
            .expect("authored logical numeric truth")
    );
    assert!(ValueOps::as_int(&mut vm, &Value::from_string_bytes(b"1\0tail".to_vec())).is_err());
    assert!(ValueOps::as_int(&mut vm, &Value::string("18446744073709551616")).is_err());
    let child = vm.fork_child_state();
    let parent_policy = vm.native_compiler_policy();
    let child_id = vm.new_interp_slot(child, vm.cur);
    vm.in_interp(child_id, |child| {
        assert_eq!(child.logical_providers.numeric, Some(provider));
        assert_eq!(child.numeric_context().simulation, Some(provider));
        assert_eq!(child.native_compiler_policy(), parent_policy);
    });
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_numeric_provider(provider));
    assert!(vm.numeric_context().simulation.is_none());
}

#[test]
fn authored_expression_policy_has_no_fixed_table_and_refuses_before_function_operands() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_native_engine_profile(host));
    let absent = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert!(absent.expression_policy.is_none());
    assert!(vm.set_logical_expression_parse_provider(
        LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    let parser = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(
        parser.expression_policy.unwrap().origin,
        ExpressionEvaluationOrigin::AuthoredTcl84Parser
    );
    assert!(
        parser
            .expression_policy
            .unwrap()
            .numeric_simulation
            .is_none()
    );
    assert!(!absent.same_compilation_world(&parser));
    assert!(vm.set_logical_numeric_provider(AuthoredLogicalNumericSimulation::Tcl84Core));
    let scalar = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert!(!parser.same_compilation_world(&scalar));
    assert!(scalar.math_functions.is_none());
    assert_eq!(
        scalar.execution_point,
        tcl_registry::InvocationDialect::of_profile(host).execution_point()
    );
    assert_eq!(
        vm.try_eval_expr("077+1")
            .unwrap()
            .result
            .string_bytes()
            .as_ref(),
        b"64"
    );
    assert!(
        vm.try_eval_expr("abs([set function_operand_entered 1])")
            .is_err()
    );
    assert!(vm.get_var("function_operand_entered").is_none());
}

#[test]
fn host_activation_preserves_separate_authored_numeric_and_expression_policies() {
    use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
    use tcl_syntax::value::ValueOps;
    let provider = AuthoredLogicalNumericSimulation::Tcl84Core;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        Rc::new(crate::host_native::NativeHost::new()),
        host,
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_numeric_provider(provider));
    assert!(vm.set_logical_expression_parse_provider(
        tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    assert_eq!(
        vm.expression_source_profile().cache_key(),
        source.cache_key()
    );
    let logical_sum = crate::expr::arith_in(
        vm.numeric_context(),
        tcl_syntax::expr::BinOp::Add,
        &Value::string("077"),
        &Value::int(1),
    )
    .expect("authored octal arithmetic");
    assert_eq!(logical_sum.string_bytes().as_ref(), b"64");
    let logical_sum_expression = vm
        .try_eval_expr("077+1")
        .expect("explicit authored scalar evaluator");
    assert_eq!(logical_sum_expression.result.string_bytes().as_ref(), b"64");
    let huge = Value::string("18446744073709551616");
    assert!(
        crate::expr::arith_in(
            vm.numeric_context(),
            tcl_syntax::expr::BinOp::Add,
            &huge,
            &Value::int(1)
        )
        .is_err()
    );
    assert_eq!(ValueOps::try_char_len(&mut vm, &Value::string("😀")), Ok(2));
    let user_policy = vm.native_compiler_policy();
    vm.active_native_profile = Some(host);
    assert!(matches!(
        vm.expression_evaluation_policy().unwrap().origin,
        tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::Native(_)
    ));
    let native_abs = vm
        .try_eval_expr("abs(077)")
        .expect("actual C9 command-table math");
    assert_eq!(native_abs.result.string_bytes().as_ref(), b"77");
    assert!(vm.numeric_context().simulation.is_none());
    assert_eq!(vm.expression_source_profile().cache_key(), host.cache_key());
    assert_eq!(ValueOps::as_int(&mut vm, &Value::string("077")), Ok(77));
    assert!(ValueOps::as_int(&mut vm, &Value::string("18446744073709551616")).is_err());
    let native_sum = crate::expr::arith_in(
        vm.numeric_context(),
        tcl_syntax::expr::BinOp::Add,
        &huge,
        &Value::int(1),
    )
    .expect("actual C9 bignum arithmetic");
    assert_eq!(native_sum.string_bytes().as_ref(), b"18446744073709551617");
    assert_eq!(ValueOps::try_char_len(&mut vm, &Value::string("😀")), Ok(1));
    let membership = vm
        .try_eval_expr("\"a\" in {a b c}")
        .expect("host expression grammar");
    assert_eq!(membership.result.string_bytes().as_ref(), b"1");
    assert_ne!(vm.native_compiler_policy(), user_policy);
    vm.active_native_profile = None;
    assert_eq!(
        vm.expression_evaluation_policy().unwrap().origin,
        tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::AuthoredTcl84Parser
    );
    assert_eq!(
        vm.expression_source_profile().cache_key(),
        source.cache_key()
    );
    assert_eq!(vm.native_compiler_policy(), user_policy);
    let child = vm.fork_child_state();
    let child_id = vm.new_interp_slot(child, vm.cur);
    vm.in_interp(child_id, |child| {
        assert_eq!(child.logical_providers.numeric, Some(provider));
        assert_eq!(child.numeric_context().simulation, Some(provider));
        assert_eq!(child.native_compiler_policy(), user_policy);
    });
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_numeric_provider(provider));
    assert!(vm.numeric_context().simulation.is_none());
}

#[test]
fn source_word_recipe_requires_its_own_provider_and_cache_identity() {
    use tcl_registry::invocation_words::LogicalSourceWordProvider;
    let provider = LogicalSourceWordProvider::Tcl84CoreSimulation;
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_native_engine_profile(host));
    let before = vm.native_compiler_policy();
    assert!(vm.set_logical_source_word_provider(provider));
    assert_ne!(vm.native_compiler_policy(), before);
    assert_eq!(
        vm.source_string_protocol(),
        Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    let child = vm.fork_child_state();
    assert_eq!(child.logical_providers.source_words, Some(provider));
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_source_word_provider(provider));
    assert_eq!(
        vm.source_string_protocol(),
        Some(tcl_syntax::native_string::NativeStringProtocol::C(
            tcl_dialect::TclVersion::V9_0
        ))
    );
}

#[test]
fn expression_source_cache_follows_selected_success_and_failure_preparation() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::new();
        vm.set_dialect_profile(profile);
        let valid = Value::list(vec![
            Value::string("1"),
            Value::string("+"),
            Value::string("2"),
        ]);
        assert!(vm.prepare_expression_value(&valid).is_ok(), "{engine}");
        assert!(!valid.has_list_representation(), "{engine}");
        assert_eq!(valid.string_bytes().as_ref(), b"1 + 2");
        let invalid = Value::list(vec![Value::string("1"), Value::string("+")]);
        assert!(vm.prepare_expression_value(&invalid).is_err(), "{engine}");
        assert_eq!(
            invalid.has_list_representation(),
            engine == "tcl8.4",
            "{engine}"
        );
        assert_eq!(invalid.string_bytes().as_ref(), b"1 +");
    }
}

#[test]
fn logical_parser_capability_has_explicit_host_cache_and_child_ownership() {
    use tcl_registry::invocation_words::LogicalExpressionParseProvider;
    let provider = LogicalExpressionParseProvider::Tcl84CoreSimulation;
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.expression_template_policy().is_none());
    assert!(vm.set_native_engine_profile(host));
    let policy = vm.native_compiler_policy();
    assert!(vm.set_logical_expression_parse_provider(provider));
    assert_ne!(vm.native_compiler_policy(), policy);
    assert!(vm.native_invocation_dialect().execution_point().is_none());
    assert!(vm.expression_template_policy().is_some());
    let object = Value::from_native_string_bytes(&b"{\xff\0tail}"[..]);
    vm.prepare_expression_value(&object)
        .expect("explicit logical parser");
    assert_eq!(object.string_bytes().as_ref(), b"{\xff\0tail}");
    let child = vm.fork_child_state();
    assert_eq!(child.logical_providers.expression_parse, Some(provider));
    let policy = vm.native_compiler_policy();
    let child_id = vm.new_interp_slot(child, vm.cur);
    vm.in_interp(child_id, |child| {
        assert_eq!(child.native_compiler_policy(), policy);
    });
    vm.active_native_profile = Some(host);
    assert_eq!(
        vm.expression_parse_context().native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V9_0)
    );
    vm.active_native_profile = None;
    assert_eq!(
        vm.expression_parse_context().native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4)
    );
    vm.set_dialect_profile(host);
    assert!(!vm.set_logical_expression_parse_provider(provider));
}

#[test]
fn explicit_host_activation_restores_user_policy_and_separates_cached_bodies() {
    struct PolicyProbe;
    impl NativeCommand for PolicyProbe {
        fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
            assert_eq!(
                vm.dialect_profile().cache_key(),
                tcl_dialect::DialectProfile::irules().cache_key(),
            );
            ok(Value::string(if vm.active_native_profile.is_some() {
                "HOST"
            } else {
                "USER"
            }))
        }
    }
    let source = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    let mut vm = Vm::new();
    vm.set_dialect_profile(source);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.expression_quote_control().is_none());
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_quote_provider(
        tcl_registry::invocation_words::LogicalExpressionQuoteProvider::Tcl84CoreSimulation,
    ));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(source)));
    vm.register_native_command("policy_probe", Rc::new(PolicyProbe));
    let user_stamp = vm.native_cache_stamp(ROOT_NS);
    let user_entry = without_literal_observation(
        vm.native_compilation_entry_for_namespace("", false)
            .unwrap(),
    );
    let generation = vm.profile_generation();
    let completion = vm
        .try_eval_native_host_source("policy_probe")
        .expect("host activation");
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"HOST");
    assert!(vm.active_native_profile.is_none());
    assert_eq!(vm.profile_generation(), generation);
    assert_eq!(vm.native_cache_stamp(ROOT_NS), user_stamp);
    let completion = vm
        .try_eval_source("policy_probe")
        .expect("logical user activation");
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"USER");
    assert_eq!(
        without_literal_observation(
            vm.native_compilation_entry_for_namespace("", false)
                .unwrap()
        ),
        user_entry
    );
    let child = vm.fork_child_state();
    assert_eq!(child.logical_providers.quote, vm.logical_providers.quote);
    assert_eq!(
        (
            child.actual_native_execution_profile().cache_key(),
            child.native_execution_profile().cache_key(),
            child.logical_providers.quote
        ),
        (
            vm.native_compiler_policy().engine.key(),
            vm.native_compiler_policy().invocation.key(),
            vm.native_compiler_policy().quote_provider
        ),
    );
    vm.active_native_profile = Some(host);
    let host_stamp = vm.native_cache_stamp(ROOT_NS);
    assert_ne!(host_stamp.policy, user_stamp.policy);
    let host_unit = vm.compile_script_cached("policy_probe").expect("host unit");
    assert_eq!(host_unit.native_cache.unwrap().policy, host_stamp.policy);
    let host_frame = crate::exec::Frame::new(host_unit, false);
    vm.active_native_profile = None;
    assert_eq!(
        vm.activation_stack_stale_message(&[host_frame], &vm.source_namespace_path()),
        Some("cannot continue bytecode after native invocation policy changed"),
    );
}

#[test]
fn explicit_host_source_keeps_parser_compiler_and_cache_profile_together() {
    let mut vm = Vm::new();
    let user = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    vm.set_dialect_profile(user);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    assert!(vm.set_logical_source_word_provider(
        tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation,
    ));
    let original = without_literal_observation(
        vm.native_compilation_entry_for_namespace("", false)
            .unwrap(),
    );
    assert_eq!(original.profile, user.cache_key());
    assert!(!vm.lexer_config().expand_syntax);
    assert_eq!(
        vm.source_string_protocol().unwrap().escape_syntax(),
        user.grammar.escapes,
    );

    vm.active_native_profile = Some(host);
    let entry = vm
        .native_compilation_entry_for_namespace("", false)
        .unwrap();
    assert_eq!(entry.profile, host.cache_key());
    assert_eq!(vm.source_profile().cache_key(), host.cache_key());
    assert_eq!(
        vm.lexer_config(),
        tcl_lexer::LexerConfig::from_grammar(host.grammar)
    );
    assert_eq!(vm.escape_syntax(), host.grammar.escapes);
    assert_eq!(vm.braced_var_style(), host.grammar.braced_var);
    assert_eq!(entry.lexer_grammar, Some(host.grammar));
    assert_eq!(
        vm.source_string_protocol().unwrap().escape_syntax(),
        host.grammar.escapes,
    );
    assert_eq!(vm.dialect_profile().cache_key(), user.cache_key());

    vm.active_native_profile = None;
    assert_eq!(
        without_literal_observation(
            vm.native_compilation_entry_for_namespace("", false)
                .unwrap()
        ),
        original
    );
    assert_eq!(vm.source_profile().cache_key(), user.cache_key());
    assert!(!vm.lexer_config().expand_syntax);
}

#[test]
fn host_script_and_its_nested_substitution_use_the_explicit_source_activation() {
    let mut vm = Vm::new();
    let user = tcl_dialect::DialectProfile::irules();
    let host = crate::environment::profile_for_dialect("tcl9.0");
    vm.set_dialect_profile(user);
    assert!(vm.set_command_surface_profile(host));
    assert!(vm.set_native_engine_profile(host));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(user)));
    let completion = vm
        .try_eval_native_host_source("set original [list {*}{A B}]; set original")
        .expect("explicit host source admission");
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.string_bytes().as_ref(), b"A B");
    assert!(vm.active_native_profile.is_none());
    assert_eq!(vm.dialect_profile().cache_key(), user.cache_key());
    assert!(!vm.lexer_config().expand_syntax);
    assert!(!vm.eval_cache.is_empty());
    assert!(
        vm.eval_cache
            .values()
            .all(|cached| cached.entry.profile == host.cache_key())
    );
}

#[test]
fn counted_syntax_failure_state_distinguishes_direct_from_script_preparation() {
    for &(engine, case, source, expected_message, direct_code, eval_code) in
        tcl_test_support::expressions::NATIVE_EXPRESSION_SYNTAX_STATES
    {
        let mut vm = Vm::new();
        let profile = tcl_registry::model::ingress::resolve_environment(engine).analyser_profile();
        vm.set_dialect_profile(profile);
        let seed = vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
            b"PROBE BEFORE".to_vec(),
        ));
        if engine == "jim" {
            vm.set_var("::errorCode", seed.clone()).unwrap();
        }
        let completion = vm.try_eval_expr_bytes(source).unwrap();
        assert_eq!(completion.code, Code::Error, "{engine}/{case}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected_message,
            "{engine}/{case}"
        );
        let original_code = crate::command::opt_get(&completion.options, "-errorcode").unwrap();
        assert_eq!(
            original_code.string_bytes().as_ref(),
            direct_code,
            "{engine}/{case} Direct"
        );
        if direct_code == b"PROBE BEFORE" {
            assert!(
                original_code.is_same_object(&seed),
                "{engine}/{case} original code object"
            );
        }
        let error = vm
            .prepare_expression_value(&Value::from_string_bytes(source))
            .unwrap_err();
        let completion = crate::command::completion_from_tcl_error(&mut vm, error);
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            expected_message,
            "{engine}/{case}"
        );
        let code = crate::command::opt_get(&completion.options, "-errorcode").unwrap();
        assert_eq!(
            code.string_bytes().as_ref(),
            eval_code,
            "{engine}/{case} Eval"
        );
    }
}

#[test]
fn primitive_unchanged_error_code_does_not_read_guest_global_or_restore_a_store() {
    let mut vm = Vm::new();
    let original = vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
        b"PRIOR \xff\0code".to_vec(),
    ));
    vm.set_var("::errorCode", Value::string("GUEST MUTATION"))
        .expect("guest state");
    let unchanged =
        vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Unchanged);
    assert!(unchanged.is_same_object(&original));
    assert!(
        vm.native_errors
            .primitive_error_code
            .as_ref()
            .expect("original state")
            .is_same_object(&original)
    );
    assert_eq!(
        vm.get_var("::errorCode")
            .expect("guest global")
            .string_bytes()
            .as_ref(),
        b"GUEST MUTATION"
    );
    let changed = vm.apply_primitive_error_code(tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(
        b"NEXT \xfe".to_vec(),
    ));
    assert_eq!(changed.string_bytes().as_ref(), b"NEXT \xfe");
    assert!(!changed.is_same_object(&original));
}

struct RefusingCompiler(tcl_runtime_api::CompileError);

impl tcl_runtime_api::CompileService for RefusingCompiler {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_script_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        _target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "procedure compilation unavailable".into(),
        ))
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        _target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
        _dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "procedure compilation unavailable".into(),
        ))
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        _profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        _profile: &'static tcl_dialect::DialectProfile,
        _entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
    }

    fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }

    fn compile_plain_script_for_profile(
        &self,
        _: tcl_runtime_api::ScriptCompileTarget<'_>,
        _: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(self.0.clone())
    }
}

#[test]
fn compile_service_capabilities_and_host_definitions_preserve_typed_refusal() {
    use crate::VmCompilationError;
    use tcl_runtime_api::{CompileError, NativeCompilationAdmissionScope, NativeExecutionError};
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(RefusingCompiler(CompileError::Unsupported(
        "provider capability unavailable".into(),
    ))));
    let error = vm
        .try_compile_function("set untouched CHANGED")
        .unwrap_err();
    let VmCompilationError::Host(NativeExecutionError::CompileServiceRefusal(payload)) = error
    else {
        panic!("typed script capability refusal expected: {error:?}")
    };
    assert_eq!(payload.source.bytes(), b"set untouched CHANGED");
    assert_eq!(payload.scope, NativeCompilationAdmissionScope::Script);
    assert_eq!(payload.compilation_namespace, NamespacePath::root());
    assert_eq!(
        payload.native_profile,
        vm.native_execution_profile().cache_key()
    );
    assert_eq!(payload.frame, 0);
    assert!(vm.get_var("untouched").is_none());

    // The service's procedure capability is independently unsupported. As
    // in C, the definition does not compile its body: the typed refusal
    // comes from the first call, before any of the body runs.
    vm.try_define_procedure("missing", &[], "set untouched CHANGED")
        .expect("a procedure definition defers body compilation");
    let error = vm.try_invoke_command("missing", &[]).unwrap_err();
    let NativeExecutionError::CompileServiceRefusal(payload) = error else {
        panic!("typed procedure capability refusal expected: {error:?}")
    };
    assert_eq!(payload.source.bytes(), b"set untouched CHANGED");
    assert_eq!(
        payload.scope,
        NativeCompilationAdmissionScope::ProcedureBody
    );
    assert_eq!(payload.compilation_namespace, NamespacePath::root());
    assert!(vm.get_var("untouched").is_none());

    vm.set_compiler(Box::new(RefusingCompiler(CompileError::Message(
        "provider capability unavailable".into(),
    ))));
    let error = vm
        .try_compile_function("set untouched CHANGED")
        .unwrap_err();
    assert!(matches!(error, VmCompilationError::Tcl(error)
            if error.message_bytes().is_ok_and(|message| message.as_ref() == b"provider capability unavailable")));
}

#[test]
fn jim_math_numeric_results_follow_the_measured_native_protocol() {
    let mut vm = crate::native_fixture::interpreter(release_profile("jim"));
    for (source, expected) in [
        ("pow(2,2)", "4"),
        ("pow(2.0,2)", "4.0"),
        ("pow(2,-1)", "0"),
        ("pow(-1,-3)", "-1"),
        ("pow(2,63)", "-9223372036854775808"),
        ("pow(2,64)", "0"),
        ("abs(-9223372036854775808)", "-9223372036854775808"),
        ("round(0.49999999999999994)", "1"),
        ("round(4503599627370497.0)", "4503599627370498"),
        ("int(1e20)", "-9223372036854775808"),
        ("sqrt(-1)", "NaN"),
        ("fmod(1,0)", "NaN"),
    ] {
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, Code::Ok, "{source}");
        assert_eq!(completion.result.to_str().as_ref(), expected, "{source}");
    }
    let completion = vm.try_eval_expr("pow(0,-1)").unwrap();
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "exponentiation of zero by negative power"
    );
}

#[test]
fn moved_math_handler_keeps_its_registered_implementation_identity() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(
        vm.try_invoke_command(
            "rename",
            &[
                Value::string("::tcl::mathfunc::abs"),
                Value::string("::savedAbs"),
            ]
        )
        .unwrap()
        .code,
        Code::Ok
    );
    assert_eq!(
        vm.try_invoke_command(
            "interp",
            &[
                Value::string("alias"),
                Value::empty(),
                Value::string("::tcl::mathfunc::abs"),
                Value::empty(),
                Value::string("::savedAbs"),
            ]
        )
        .unwrap()
        .code,
        Code::Ok
    );
    let result = vm.try_eval_expr("abs(-3)").unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.to_str().as_ref(), "3");
}

#[test]
fn jim_expression_preparation_uses_native_tree_and_eager_fixed_function_pass() {
    let mut vm = crate::native_fixture::interpreter(release_profile("jim"));
    for (source, expected) in [("abs(pow(2),2)", "4"), ("abs(-3)", "3")] {
        let value = Value::string(source);
        // A warm conventional AST must never bypass native reassociation.
        for _ in 0..2 {
            let node = vm.prepare_expression_value(&value).unwrap();
            let mut ops = ExprEval::new(&mut vm);
            assert_eq!(eval(&node, &mut ops).unwrap().to_str().as_ref(), expected);
        }
    }
    for (source, expected) in [
        ("pow(abs(2,2))", "too many arguments for math function"),
        ("pow(1)", "too few arguments for math function"),
        (
            "0 && future_function(1)",
            "syntax error in expression: \"0 && future_function(1)\"",
        ),
        (
            "1 ? 2 : future_function(1)",
            "syntax error in expression: \"1 ? 2 : future_function(1)\"",
        ),
    ] {
        let completion = vm.try_eval_expr(source).unwrap();
        assert_eq!(completion.code, Code::Error, "{source}");
        assert_eq!(completion.result.to_str().as_ref(), expected, "{source}");
    }
}

#[test]
fn compile_service_admission_preserves_the_neutral_host_channel() {
    use tcl_runtime_api::{CompileError, NativeCompilationAdmissionError, NativeExecutionError};
    let mut vm = Vm::new();
    vm.set_var("before", Value::string("PRIOR")).unwrap();
    vm.set_var("errorInfo", Value::string("ORIGINAL")).unwrap();
    vm.set_compiler(Box::new(RefusingCompiler(
        CompileError::NativeCompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired,
        ),
    )));
    assert_eq!(
        vm.try_eval_source("set after CHANGED").unwrap_err(),
        NativeExecutionError::CompilationAdmission(
            NativeCompilationAdmissionError::NativePreflightRequired,
        )
    );
    assert_eq!(vm.get_var("before").unwrap().to_str().as_ref(), "PRIOR");
    assert_eq!(
        vm.get_var("errorInfo").unwrap().to_str().as_ref(),
        "ORIGINAL"
    );
    assert!(vm.get_var("after").is_none());
    // An ordinary compiler diagnostic is still a Tcl failure. Its bytes
    // may equal a host diagnostic without acquiring the host's meaning.
    vm.set_compiler(Box::new(RefusingCompiler(CompileError::Message(
        NativeCompilationAdmissionError::NativePreflightRequired.to_string(),
    ))));
    let completion = vm.try_eval_source("set after CHANGED").unwrap();
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "native compiler preflight provider required"
    );
    assert!(vm.get_var("after").is_none());
}

#[test]
fn procedure_header_admission_retains_lookup_and_callable_incarnations() {
    use tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite;
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert!(
        vm.try_eval_source("proc noop args {}; namespace eval N {}")
            .unwrap()
            .code
            .is_ok()
    );
    let entry = vm
        .native_compilation_entry_for_namespace("N", false)
        .unwrap();
    let callable = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "noop")
        .unwrap();
    let required = NativeProcedureHeaderPrerequisite {
        interpreter: entry.interpreter,
        lookup_namespace_token: entry.current_namespace,
        invocation_word: "noop".into(),
        slot: callable.slot.clone(),
        namespace_token: callable.namespace_token,
        token: callable.token,
        implementation_generation: callable.implementation_generation,
        header: tcl_dialect::NativeProcedureHeaderCompilation::NoOp,
        guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
    };
    assert!(vm.native_procedure_header_prerequisite_matches(&required));
    let mut foreign = required.clone();
    foreign.lookup_namespace_token = u64::MAX;
    assert!(!vm.native_procedure_header_prerequisite_matches(&foreign));
    foreign = required.clone();
    foreign.interpreter.owner += 1;
    assert!(!vm.native_procedure_header_prerequisite_matches(&foreign));
    assert!(
        vm.try_eval_source("proc N::noop args {return LOCAL}")
            .unwrap()
            .code
            .is_ok()
    );
    assert!(!vm.native_procedure_header_prerequisite_matches(&required));
    let mut absolute = required.clone();
    absolute.invocation_word = "::noop".into();
    assert!(vm.native_procedure_header_prerequisite_matches(&absolute));
    assert!(
        vm.try_eval_source("rename noop {}; proc noop args {}")
            .unwrap()
            .code
            .is_ok()
    );
    assert!(!vm.native_procedure_header_prerequisite_matches(&absolute));
}

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

/// A child is another interpreter of the same build, so it states the
/// world its parent is pinned to, the overlay included.
#[test]
fn a_child_states_the_context_its_parent_is_pinned_to() {
    const OVERLAY: u64 = 0x0C0_1706;
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |_| {});
    let mut context = tcl_registry::model::runtime_context_for_profile(profile);
    context.overlay_generation = OVERLAY;
    context.packages = vec![("vendor".to_owned(), "2.1".to_owned())];
    let mut vm = Vm::new();
    vm.pin_context(&context).expect("installed, so it pins");

    let name = vm.create_child(Some("child".to_owned()), false);
    let id = vm.child_id(&name).expect("the child is in the arena");
    let (stated, held) = vm.in_interp(id, |child| {
        (
            child.runtime_context().clone(),
            child.held_identity().clone(),
        )
    });
    assert_eq!(stated, context);
    assert_eq!(&held, vm.held_identity());
}

#[test]
fn invokehidden_rechecks_a_hidden_builtin_against_the_child_profile() {
    let mut vm = Vm::new();
    let child_name = vm.create_child(Some("child".to_string()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, |child_vm| {
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        );
        child_vm.hide_command("lassign", "held").unwrap();
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
        );
    });
    let named = vm.invoke_hidden_in_child("child", "held", &[]).unwrap();
    assert_eq!(named.code, Code::Error);
    let by_id = vm.invoke_hidden_by_id(child, "held", &[]).unwrap();
    assert_eq!(by_id.code, Code::Error);
    assert!(
        vm.st_of(child)
            .unwrap()
            .hidden
            .commands
            .contains_key("held")
    );
    assert!(!vm.st_of(child).unwrap().commands.contains_key("held"));
}

#[test]
fn root_invokehidden_rechecks_identity_and_restores_hidden_state_after_error() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    );
    vm.hide_command("lassign", "held").unwrap();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );

    let result = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(result.code, Code::Error);
    assert!(vm.hidden.commands.contains_key("held"));
    assert!(!vm.commands.contains_key("held"));
    assert!(!vm.builtin_identities.contains_key("held"));
    assert!(!vm.imported_commands.contains_key("held"));
}

fn eval_value(vm: &mut Vm, script: &str) -> String {
    let completion = vm.eval_source(script).expect("script compiles");
    assert_eq!(completion.code, Code::Ok, "{script}: {completion:?}");
    completion.result.to_str().to_string()
}

#[test]
fn only_a_changed_root_profile_reestablishes_standard_channel_defaults() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    vm.set_dialect_profile(v90);

    assert_eq!(
        eval_value(
            &mut vm,
            "fconfigure stdout -encoding ascii -translation crlf -profile replace; \
                 list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace"
    );

    vm.set_dialect_profile(v90);
    assert_eq!(
        eval_value(
            &mut vm,
            "list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace",
        "re-pinning the root to the same profile reset live channel state"
    );

    let child = vm.create_child(Some("child".to_owned()), false);
    assert!(vm.set_child_dialect_profile(&child, v86));
    assert_eq!(
        eval_value(
            &mut vm,
            "list [fconfigure stdout -encoding] [fconfigure stdout -translation] \
                      [fconfigure stdout -profile]",
        ),
        "ascii crlf replace",
        "pinning a child profile reset the VM-wide standard channel"
    );

    vm.set_dialect_profile(v86);
    assert_eq!(
        eval_value(&mut vm, "fconfigure stdout -translation"),
        "lf",
        "changing the root profile did not establish release defaults"
    );
}

fn prepare_stale_hidden_proc(vm: &mut Vm) {
    select_release(vm, "tcl8.5");
    assert_eq!(eval_value(vm, "proc p {} { return hidden }"), "");
    vm.hide_command("p", "held").unwrap();
    assert_eq!(eval_value(vm, "proc p {} { return replacement }"), "");
    select_release(vm, "tcl8.6");
}

fn prepare_trace_deleted_hidden_proc(vm: &mut Vm) {
    select_release(vm, "tcl8.5");
    assert_eq!(eval_value(vm, "proc p {} { return hidden }"), "");
    assert_eq!(
        eval_value(
            vm,
            "proc replace args { rename p {}; interp expose {} held p; rename p {}; proc p {} { return replacement } }"
        ),
        ""
    );
    assert_eq!(eval_value(vm, "trace add execution p enter replace"), "");
    vm.hide_command("p", "held").unwrap();
    assert_eq!(eval_value(vm, "proc p {} { return initial }"), "");
    select_release(vm, "tcl8.6");
}

#[test]
fn refreshed_hidden_proc_stays_hidden_and_preserves_visible_replacement() {
    // Children replay their parent's constructor engine, which the
    // fixtures leave selected.
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_6);
    prepare_stale_hidden_proc(&mut vm);
    let hidden = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(hidden.code, Code::Ok);
    assert_eq!(hidden.result.to_str().as_ref(), "hidden");
    assert_eq!(eval_value(&mut vm, "p"), "replacement");
    assert!(matches!(
        vm.hidden.commands.get("held"),
        Some(Command::Proc(proc))
            if proc.body_src.native_bytecode_cache().is_some_and(|cache| cache.unit.profile_generation == vm.profile_generation)
    ));

    let child_name = vm.create_child(Some("child".to_owned()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, prepare_stale_hidden_proc);

    let named = vm.invoke_hidden_in_child("child", "held", &[]).unwrap();
    assert_eq!(named.code, Code::Ok);
    assert_eq!(named.result.to_str().as_ref(), "hidden");
    assert_eq!(
        vm.in_interp(child, |child| eval_value(child, "p")),
        "replacement"
    );

    vm.in_interp(child, |child| select_release(child, "tcl8.5"));
    let by_id = vm.invoke_hidden_by_id(child, "held", &[]).unwrap();
    assert_eq!(by_id.code, Code::Ok);
    assert_eq!(by_id.result.to_str().as_ref(), "hidden");
    assert_eq!(
        vm.in_interp(child, |child| eval_value(child, "p")),
        "replacement"
    );
    let state = vm.st_of(child).unwrap();
    assert!(matches!(
        state.hidden.commands.get("held"),
        Some(Command::Proc(proc))
            if proc.body_src.native_bytecode_cache().is_some_and(|cache| cache.unit.profile_generation == state.profile_generation)
    ));
}

#[test]
fn trace_deleted_hidden_refresh_never_resurrects_or_overwrites_replacement() {
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_6);
    prepare_trace_deleted_hidden_proc(&mut vm);
    let root = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(root.code, Code::Error);
    assert_eq!(
        root.result.to_str().as_ref(),
        "attempt to invoke a deleted command"
    );
    assert_eq!(eval_value(&mut vm, "p"), "replacement");
    assert!(!vm.hidden.commands.contains_key("held"));

    for by_id in [false, true] {
        let child_name = vm.create_child(None, false);
        let child = vm.child_id(&child_name).unwrap();
        vm.in_interp(child, prepare_trace_deleted_hidden_proc);
        let result = if by_id {
            vm.invoke_hidden_by_id(child, "held", &[]).unwrap()
        } else {
            vm.invoke_hidden_in_child(&child_name, "held", &[]).unwrap()
        };
        assert_eq!(result.code, Code::Error);
        assert_eq!(
            result.result.to_str().as_ref(),
            "attempt to invoke a deleted command"
        );
        assert_eq!(
            vm.in_interp(child, |child| eval_value(child, "p")),
            "replacement"
        );
        assert!(
            !vm.st_of(child)
                .unwrap()
                .hidden
                .commands
                .contains_key("held")
        );
    }
}

#[test]
fn relocated_hidden_refresh_persists_at_the_live_visible_key() {
    let mut vm = Vm::new();
    select_release(&mut vm, "tcl8.5");
    assert_eq!(eval_value(&mut vm, "proc p {} { return hidden }"), "");
    assert_eq!(
        eval_value(
            &mut vm,
            "proc expose args { rename p {}; interp expose {} held p; trace remove execution p enter expose }"
        ),
        ""
    );
    assert_eq!(
        eval_value(&mut vm, "trace add execution p enter expose"),
        ""
    );
    vm.hide_command("p", "held").unwrap();
    assert_eq!(eval_value(&mut vm, "proc p {} { return replacement }"), "");
    select_release(&mut vm, "tcl8.6");

    let result = vm.invoke_hidden_in_child("", "held", &[]).unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.to_str().as_ref(), "hidden");
    assert!(!vm.hidden.commands.contains_key("held"));
    assert_eq!(eval_value(&mut vm, "p"), "hidden");
}

#[test]
fn imported_proc_profile_refresh_updates_the_resolved_import_not_its_source_name() {
    let mut vm = Vm::new();
    select_release(&mut vm, "tcl8.5");
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval src {proc p {} {return original}; namespace export p}; \\
                 namespace eval dst {namespace import ::src::p}; \\
                 rename ::src::p ::src::q; proc ::src::p {} {return replacement}",
        ),
        ""
    );

    select_release(&mut vm, "tcl8.6");
    assert_eq!(eval_value(&mut vm, "::dst::p"), "original");
    assert_eq!(eval_value(&mut vm, "::src::p"), "replacement");
    assert_eq!(eval_value(&mut vm, "::src::q"), "original");
    assert!(matches!(
        vm.commands.get("dst::p"),
        Some(Command::Proc(proc))
            if proc.body_src.native_bytecode_cache().is_some_and(|cache| cache.unit.profile_generation == vm.profile_generation)
    ));
}

struct TestNative;

impl NativeCommand for TestNative {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        ok(Value::empty())
    }
}

struct OriginNative(Rc<RefCell<Option<String>>>);

impl NativeCommand for OriginNative {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        *self.0.borrow_mut() = Some(vm.command_origin_key("held"));
        ok(Value::empty())
    }
}

#[test]
fn invokehidden_rechecks_a_hidden_native_identity_for_both_child_paths() {
    let mut vm = Vm::new();
    let child_name = vm.create_child(Some("child".to_string()), false);
    let child = vm.child_id(&child_name).unwrap();
    vm.in_interp(child, |child_vm| {
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        child_vm.register_native_command("isfinite", Rc::new(TestNative));
        child_vm.restore_builtin_identity("isfinite", "tcl::mathfunc::isfinite".to_string());
        child_vm.hide_command("isfinite", "held").unwrap();
        child_vm.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
    });

    assert_eq!(
        vm.invoke_hidden_in_child("child", "held", &[])
            .unwrap()
            .code,
        Code::Error
    );
    assert_eq!(
        vm.invoke_hidden_by_id(child, "held", &[]).unwrap().code,
        Code::Error
    );
    let state = vm.st_of(child).unwrap();
    assert!(state.hidden.commands.contains_key("held"));
    assert!(!state.commands.contains_key("held"));
    assert!(!state.builtin_identities.contains_key("held"));
}

#[test]
fn invokehidden_temporarily_restores_imported_command_provenance() {
    let observed = Rc::new(RefCell::new(None));
    let mut vm = Vm::new();
    vm.register_native_command("src::probe", Rc::new(OriginNative(Rc::clone(&observed))));
    vm.declare_namespace_exports("src", &["*"]);
    assert_eq!(
        vm.import_commands("::src::*", false).unwrap(),
        vec!["probe"]
    );
    vm.hide_command("probe", "held").unwrap();

    assert!(
        vm.invoke_hidden_in_child("", "held", &[])
            .unwrap()
            .code
            .is_ok()
    );
    // Direct hidden dispatch must not publish hidden provenance under the
    // token: normal command lookup from the body sees only visible state.
    assert_eq!(observed.borrow().as_deref(), Some("held"));
    assert_eq!(
        vm.hidden
            .imports
            .get("held")
            .map(|binding| binding.origin.key.name()),
        Some("src::probe")
    );
    assert!(!vm.imported_commands.contains_key("held"));
    assert!(!vm.commands.contains_key("held"));
}

fn guarded_builtin(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    ok(Value::empty())
}

#[test]
fn command_guard_requires_identity_and_has_exact_lifecycle() {
    let mut vm = Vm::new();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    assert_eq!(
        vm.prepare_command_guard("guarded", GuardIdentity::new(1, 42), domains),
        Err(GuardError::IdentityMismatch)
    );
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .unwrap();
    assert!(vm.check_command_guard(token, "guarded"));
    assert!(vm.release_command_guard(token));
    assert!(!vm.release_command_guard(token));
    assert!(!vm.check_command_guard(token, "guarded"));
}

/// A request for a Family-B intrinsic covers the variable-trace domain or
/// is refused, whatever the caller asked for, and a Value member needs no
/// such domain. Covered, the guard is refused while a variable trace
/// exists and goes stale when one is added.
#[test]
fn a_family_b_guard_request_must_cover_the_variable_trace_domain() {
    use tcl_registry::IntrinsicId;
    let mut vm = Vm::new();
    let version = vm.runtime_version();
    let identity = |member: IntrinsicId| {
        GuardIdentity::registry_intrinsic_with_semantics(
            member.stable_id(),
            member.guard_semantics_key(version),
        )
    };
    let (stores, value) = (
        identity(IntrinsicId::DictSet),
        identity(IntrinsicId::ListLength),
    );
    vm.register_guarded_builtin("stores", guarded_builtin, stores);
    vm.register_guarded_builtin("pure", guarded_builtin, value);
    let command = GuardDomains::one(GuardDomain::CommandEnvironment);
    let traced = command.with(GuardDomain::VariableTrace);

    assert_eq!(
        vm.prepare_command_guard("stores", stores, command),
        Err(GuardError::DomainsInsufficient)
    );
    let token = vm
        .prepare_command_guard("pure", value, command)
        .expect("a Value member requires no variable-trace domain");
    assert!(vm.release_command_guard(token));

    let token = vm
        .prepare_command_guard("stores", stores, traced)
        .expect("a request covering the family's domain");
    assert!(vm.check_command_guard(token, "stores"));
    vm.add_var_trace(
        "watched",
        vec!["write".to_owned()],
        "callback".to_owned(),
        false,
    );
    assert!(!vm.check_command_guard(token, "stores"));
    assert_eq!(
        vm.prepare_command_guard("stores", stores, traced),
        Err(GuardError::PrerequisiteUnsatisfied)
    );
    assert!(vm.prepare_command_guard("pure", value, command).is_ok());
}

/// The registry's members that only observe a variable and still run its
/// traces do so here: `info exists` a read trace, the array queries an
/// array trace, as `tclsh` does.
#[test]
fn every_trace_firing_intrinsic_fires_a_trace_here() {
    use tcl_registry::IntrinsicId;
    for member in IntrinsicId::ALL
        .iter()
        .copied()
        .filter(|member| member.family().fires_traces())
    {
        let (setup, op, command) = match member {
            IntrinsicId::InfoExists => ("set v 1", "read", "info exists v"),
            IntrinsicId::ArrayExists => ("array set v {k 1}", "array", "array exists v"),
            IntrinsicId::ArrayNames => ("array set v {k 1}", "array", "array names v"),
            IntrinsicId::ArraySize => ("array set v {k 1}", "array", "array size v"),
            other => panic!("{other:?} fires traces and needs a probe here"),
        };
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        eval_value(
            &mut vm,
            &format!(
                "proc note {{n1 n2 op}} {{lappend ::fired $op}}; set ::fired {{}}; \
                     {setup}; trace add variable v {op} note"
            ),
        );
        eval_value(&mut vm, command);
        assert_eq!(eval_value(&mut vm, "set ::fired"), op, "{member:?}");
    }
}

/// A guard is bound to its command's token. A definition, rename or alias
/// of another command leaves it and its attestation alone; replacing,
/// renaming away or hiding the command drops it, and restoring the same
/// token brings it back; a profile pin keeps the attestation.
#[test]
fn an_unrelated_mutation_keeps_the_guard_and_a_rebinding_drops_it() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .unwrap();

    vm.register("unrelated", guarded_builtin);
    eval_value(&mut vm, "proc foo {} {return 1}");
    eval_value(&mut vm, "rename foo bar");
    eval_value(&mut vm, "interp alias {} baz {} bar");
    eval_value(&mut vm, "rename baz {}");
    assert!(vm.check_command_guard(token, "guarded"));
    assert!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
            .is_ok()
    );

    // A change to the lookup environment itself stales the token over it,
    // and the attestation stays.
    eval_value(&mut vm, "namespace path ::");
    assert!(!vm.check_command_guard(token, "guarded"));
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .expect("still attested");

    // The profile pin keeps the attestation and a token that does not
    // depend on interpreter policy.
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(vm.check_command_guard(token, "guarded"));
    assert!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
            .is_ok()
    );

    // Renaming the command away drops the guard at its name, and the
    // attestation goes with the command; restoring the name restores it.
    eval_value(&mut vm, "rename guarded moved");
    assert!(!vm.check_command_guard(token, "guarded"));
    assert_eq!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains),
        Err(GuardError::IdentityUnavailable)
    );
    assert!(
        vm.prepare_command_guard("moved", GUARDED_IDENTITY, domains)
            .is_ok()
    );
    eval_value(&mut vm, "rename moved guarded");
    assert!(vm.check_command_guard(token, "guarded"));

    // Hiding it drops the guard, exposing it restores it.
    eval_value(&mut vm, "interp hide {} guarded");
    assert!(!vm.check_command_guard(token, "guarded"));
    eval_value(&mut vm, "interp expose {} guarded");
    assert!(vm.check_command_guard(token, "guarded"));

    // A different command at the name is never attested.
    eval_value(&mut vm, "proc guarded {} {return 1}");
    assert!(!vm.check_command_guard(token, "guarded"));
    assert_eq!(
        vm.prepare_command_guard("guarded", GUARDED_IDENTITY, domains),
        Err(GuardError::IdentityUnavailable)
    );
}

/// Each lookup event moves the domain that covers it, as in the runtime
/// (#2292): namespace membership moves `Namespace`, and the unknown
/// handler or the command it names moves `UnknownHandling`.
#[test]
fn lookup_events_move_the_namespace_and_unknown_handling_domains() {
    let setup = "namespace eval src {proc p {} {}; namespace export p}";
    let rows: [(GuardDomain, &str, &str); 10] = [
        (GuardDomain::Namespace, "", "namespace eval created {}"),
        (GuardDomain::Namespace, "", "namespace delete src"),
        (GuardDomain::Namespace, "", "namespace export guarded"),
        (GuardDomain::Namespace, "", "namespace path ::src"),
        (GuardDomain::Namespace, "", "namespace import src::p"),
        (
            GuardDomain::Namespace,
            "namespace import src::p",
            "namespace forget src::p",
        ),
        (
            GuardDomain::UnknownHandling,
            "",
            "namespace unknown handler",
        ),
        (
            GuardDomain::UnknownHandling,
            "",
            "proc unknown args {return 0}",
        ),
        (
            GuardDomain::UnknownHandling,
            "proc unknown args {}",
            "rename unknown {}",
        ),
        (
            GuardDomain::UnknownHandling,
            "proc unknown args {}",
            "rename unknown moved",
        ),
    ];
    for (domain, prepare, event) in rows {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
        eval_value(&mut vm, setup);
        eval_value(&mut vm, prepare);
        let token = vm
            .prepare_command_guard("guarded", GUARDED_IDENTITY, GuardDomains::one(domain))
            .unwrap();
        assert!(vm.check_command_guard(token, "guarded"));
        eval_value(&mut vm, event);
        assert!(
            !vm.check_command_guard(token, "guarded"),
            "{domain:?} kept across {event}"
        );
    }
}

thread_local! {
    static PROBED_TOKEN: std::cell::Cell<Option<GuardToken>> =
        const { std::cell::Cell::new(None) };
}

/// Check the probed token against the command `guarded` from wherever this
/// runs.
fn probe_guarded(vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    let token = PROBED_TOKEN.get().expect("a token to probe");
    ok(Value::string(if vm.check_command_guard(token, "guarded") {
        "1"
    } else {
        "0"
    }))
}

/// The attestation belongs to the token the name reaches, so a definition
/// in a namespace that shadows the command drops the guard for calls made
/// from that namespace and only from it.
#[test]
fn a_shadowing_definition_drops_the_guard_only_for_calls_from_its_namespace() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);
    vm.register("probe", probe_guarded);
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let token = vm
        .prepare_command_guard("guarded", GUARDED_IDENTITY, domains)
        .unwrap();
    PROBED_TOKEN.set(Some(token));

    assert_eq!(eval_value(&mut vm, "probe"), "1");
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval ns {proc guarded {} {return shadow}; probe}"
        ),
        "0"
    );
    assert_eq!(eval_value(&mut vm, "probe"), "1");
    assert!(vm.check_command_guard(token, "guarded"));
}

#[test]
fn trace_registration_invalidates_the_matching_guard_domain() {
    let mut vm = Vm::new();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);

    let command_token = vm
        .prepare_command_guard(
            "guarded",
            GUARDED_IDENTITY,
            GuardDomains::one(GuardDomain::CommandTrace),
        )
        .unwrap();
    let added = vm.add_cmd_trace(
        false,
        "guarded",
        vec!["rename".to_owned()],
        "callback".to_owned(),
    );
    assert_eq!(added.code, Code::Ok);
    assert!(!vm.check_command_guard(command_token, "guarded"));

    let variable_token = vm
        .prepare_command_guard(
            "guarded",
            GUARDED_IDENTITY,
            GuardDomains::one(GuardDomain::VariableTrace),
        )
        .unwrap();
    vm.add_var_trace(
        "watched",
        vec!["write".to_owned()],
        "callback".to_owned(),
        false,
    );
    assert!(!vm.check_command_guard(variable_token, "guarded"));
}

#[test]
fn host_intrinsic_contract_keeps_domains_and_original_binding_owner() {
    let mut vm = Vm::new();
    let intrinsic = tcl_registry::IntrinsicId::StringLength;
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key(vm.runtime_version()),
    );
    vm.register_guarded_builtin("host", guarded_builtin, identity);
    let added = vm.add_cmd_trace(
        false,
        "host",
        vec!["rename".to_owned()],
        "callback".to_owned(),
    );
    assert_eq!(added.code, Code::Ok);
    let domains = GuardDomains::one(GuardDomain::VariableTrace);
    let host = vm.prepare_command_guard("host", identity, domains).unwrap();
    assert!(vm.check_command_guard(host, "host"));
    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::PrerequisiteUnsatisfied)
    );

    // A replacement with the same explicit contract cannot inherit the
    // token even though the requested variable-trace epoch is unchanged.
    vm.register_guarded_builtin("host", guarded_builtin, identity);
    assert!(!vm.check_command_guard(host, "host"));
    let replacement = vm.prepare_command_guard("host", identity, domains).unwrap();
    assert!(vm.check_command_guard(replacement, "host"));
}

#[test]
fn unsupported_guard_domains_are_poisoned() {
    let mut vm = Vm::new();
    vm.register_guarded_builtin("guarded", guarded_builtin, GUARDED_IDENTITY);

    for domain in [GuardDomain::Interpreter, GuardDomain::ObjectDispatch] {
        assert_eq!(
            vm.prepare_command_guard("guarded", GUARDED_IDENTITY, GuardDomains::one(domain)),
            Err(GuardError::Poisoned),
            "{domain:?} must decline until its mutation ownership is centralised",
        );
    }
}

#[test]
fn string_length_base_guard_declines_and_generic_dispatch_remains_available() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment)
        .with(GuardDomain::Namespace)
        .with(GuardDomain::CommandTrace)
        .with(GuardDomain::Interpreter);

    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::Poisoned),
    );

    let completion = vm
        .dispatch("string", &[Value::string("length"), Value::string("abc")])
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.as_int().unwrap(), 3);
}

#[test]
fn string_length_intrinsic_uses_the_selected_runtime_character_model() {
    let mut vm = Vm::new();
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    for intrinsic in [
        tcl_registry::IntrinsicId::StringLength,
        tcl_registry::IntrinsicId::StringIndex,
    ] {
        let identity = GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(vm.runtime_version()),
        );
        let token = vm
            .prepare_command_guard("string", identity, domains)
            .unwrap();
        assert!(vm.check_command_guard(token, "string"));
    }

    let value = Value::string("é🙂");
    let completion = vm
        .execute_intrinsic(
            tcl_registry::IntrinsicId::StringLength,
            std::slice::from_ref(&value),
        )
        .expect("StringLength is implemented");
    assert_eq!(completion.result.as_int().unwrap(), 2);

    let mut tcl86 = release_vm(tcl_dialect::TclVersion::V8_6);
    let completion = tcl86
        .execute_intrinsic(
            tcl_registry::IntrinsicId::StringLength,
            &[Value::string("é🙂")],
        )
        .expect("StringLength is implemented for Tcl 8");
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.as_int().unwrap(), 3);
    let completion = tcl86
        .dispatch("string", &[Value::string("length"), Value::string("é🙂")])
        .unwrap();
    assert_eq!(completion.result.as_int().unwrap(), 3);

    let completion = vm
        .dispatch("string", &[Value::string("length"), Value::string("é🙂")])
        .unwrap();
    assert_eq!(completion.result.as_int().unwrap(), 2);
    assert!(
        vm.execute_intrinsic(tcl_registry::IntrinsicId::ListLength, &[])
            .is_none()
    );
}

#[test]
fn renaming_spec_registered_string_stales_its_intrinsic_guard() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let token = vm
        .prepare_command_guard(
            "string",
            identity,
            GuardDomains::one(GuardDomain::CommandEnvironment),
        )
        .unwrap();
    let command = vm
        .take_command("string")
        .expect("registered string command");
    vm.register_command("moved", command);
    assert!(!vm.check_command_guard(token, "string"));
    assert!(!vm.check_command_guard(token, "moved"));
    assert_eq!(
        vm.prepare_command_guard(
            "string",
            identity,
            GuardDomains::one(GuardDomain::CommandEnvironment)
        ),
        Err(GuardError::IdentityUnavailable)
    );
}

/// The pin keeps the spec-registered `string`'s attestation for the
/// release it pins: a fresh guard is issued after it, where a pinned VM
/// used to hold none.
#[test]
fn a_profile_pin_keeps_the_spec_registered_string_attested() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandEnvironment);
    let token = vm
        .prepare_command_guard("string", identity, domains)
        .expect("string stays attested after the pin");
    assert!(vm.check_command_guard(token, "string"));
}

/// The sweep reads the generation the VM is pinned to and nothing an
/// overlay installed: a pack's command is not a runtime implementation, and
/// only the runtime may attest.
#[test]
fn identities_come_from_the_pinned_generation_never_an_overlay() {
    use tcl_registry::{CommandSpec, IntrinsicId, SemanticOperationId};
    // Tcl 8.4's `string` is a builtin the sweep attests; later releases
    // install an ensemble, whose stock attestation is checked live.
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_4);
    // A generation an overlay installed under this VM's own profile,
    // holding a spec that declares an intrinsic, and a builtin of that name
    // in the VM.
    let overlaid =
        tcl_registry::registry_for_profile_with_overlay(vm.dialect_profile(), 0xC0DE, |registry| {
            registry.insert(CommandSpec {
                name: "overlay_length",
                semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringLength)),
                ..CommandSpec::DEFAULT
            });
        });
    vm.register_stock_builtin("overlay_length", guarded_builtin);

    vm.attach_identities();
    assert!(
        vm.attested_identities("overlay_length").is_none(),
        "the sweep of the pinned generation attests nothing an overlay declares"
    );
    assert!(
        vm.attested_identities("string").is_some(),
        "and still attests the shipped command"
    );

    // Control: the fixture is sound, a sweep over the overlay's own store
    // would attest the builtin.
    vm.attach_identities_from(&overlaid);
    assert!(vm.attested_identities("overlay_length").is_some());
}

/// A name is attested only while the command bound at it is the builtin the
/// VM registered there: a procedure defined over it, or another builtin
/// renamed into it, gains nothing, and the original, put back, has what it
/// had.
#[test]
fn the_sweep_attests_only_the_builtin_it_registered() {
    use tcl_registry::IntrinsicId;
    // Tcl 8.4's `string` is a builtin the sweep attests.
    let mut vm = release_vm(tcl_dialect::TclVersion::V8_4);
    let length = GuardIdentity::registry_intrinsic_with_semantics(
        IntrinsicId::StringLength.stable_id(),
        IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let attested = |vm: &Vm| {
        vm.attested_identities("string")
            .is_some_and(|identities| identities.contains(&length))
    };
    assert!(attested(&vm));

    eval_value(&mut vm, "rename string original");
    assert!(!attested(&vm), "moved away, the name reaches nothing");
    eval_value(&mut vm, "proc string args {return x}");
    vm.attach_identities();
    assert!(
        !attested(&vm),
        "a procedure at the name gains no attestation"
    );

    eval_value(&mut vm, "rename string {}; rename puts string");
    vm.attach_identities();
    assert!(
        !attested(&vm),
        "another builtin renamed into the name is attested as itself and not as `string`"
    );

    eval_value(&mut vm, "rename string puts; rename original string");
    assert!(
        attested(&vm),
        "the original, restored, is attested as it was"
    );
}

/// Every builtin this VM registers at the name of a spec that declares an
/// intrinsic is attested for every identity the spec's intrinsics have.
#[test]
fn every_registered_builtin_with_an_intrinsic_is_attested_for_all_of_them() {
    let vm = Vm::new();
    let registry = crate::environment::store_for_profile(vm.dialect_profile());
    let report = tcl_runtime_api::BackingReport::from_entries(vm.backing_report());
    let mut attested_names = 0;
    for name in registry.command_names() {
        let spec = registry.get_exact(name).expect("a named spec");
        let expected: BTreeSet<GuardIdentity> = spec
            .intrinsic_ids()
            .into_iter()
            .flat_map(|id| {
                id.guard_semantics_variants().iter().map(move |semantics| {
                    GuardIdentity::registry_intrinsic_with_semantics(id.stable_id(), *semantics)
                })
            })
            .collect();
        if expected.is_empty() || report.of(name) != RegisteredBacking::Builtin {
            continue;
        }
        let actual = vm
            .attested_identities(name)
            .unwrap_or_else(|| panic!("{name} is registered and has intrinsics"));
        assert!(expected.is_subset(&actual), "{name}");
        attested_names += 1;
    }
    assert!(attested_names >= 10, "{attested_names}");
    assert!(
        vm.attested_identities("set").is_none(),
        "no intrinsic, nothing"
    );
}

/// Every builtin registered under a spelling the registry names it by has
/// that spelling in the registry, which is what the release surface gates
/// it with.
#[test]
fn a_spelled_builtin_names_a_spec_the_registry_has() {
    let vm = Vm::new();
    let registry = crate::environment::store_for_profile(vm.dialect_profile());
    // Identities are stored unrooted, and a qualified spec may be spelled
    // from the root: the spec the surface gate reaches names the identity.
    let mut qualified = 0;
    for identity in vm.builtin_identities.values() {
        let Some(spec) = registry.get(identity) else {
            continue;
        };
        assert_eq!(
            tcl_syntax::naming::unroot_rooted_key(spec.name).unwrap_or(spec.name),
            identity.as_str()
        );
        qualified += usize::from(identity.contains("::"));
    }
    assert!(qualified >= 15, "{qualified}");
}

/// The backing report says what the command table holds: handlers and the
/// engine's `TclOO` roots, absences, and nothing a script defines.
#[test]
fn the_backing_report_says_what_the_command_table_holds() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let before = tcl_runtime_api::BackingReport::from_entries(vm.backing_report());
    assert_eq!(before.of("set"), RegisteredBacking::Builtin);
    assert_eq!(before.of("::tcl::dict::get"), RegisteredBacking::Builtin);
    assert_eq!(before.of("oo::class"), RegisteredBacking::Object);
    assert_eq!(before.of("oo::object"), RegisteredBacking::Object);
    assert_eq!(before.of("tclLog"), RegisteredBacking::Absent);

    eval_value(&mut vm, "proc mine {} {}");
    eval_value(&mut vm, "oo::class create Mine");
    eval_value(&mut vm, "interp alias {} aliased {} set");
    let after = tcl_runtime_api::BackingReport::from_entries(vm.backing_report());
    for name in ["mine", "Mine", "aliased"] {
        assert_eq!(after.of(name), RegisteredBacking::Absent, "{name}");
    }
    assert_eq!(after, before, "a script adds nothing to the report");
}

/// The pin keeps every attestation and the pinned surface decides which of
/// them a guard reaches: a command the release lacks has none, and answers
/// again under a release that has it.
#[test]
fn a_pin_to_a_release_without_the_command_leaves_it_unattested() {
    let mut vm = Vm::new();
    // `lassign` is a command Tcl 8.4 does not have. This is the shipped
    // command: a host handler at the name has no stock release floor.
    let intrinsic = tcl_registry::IntrinsicId::ListAssign;
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key(vm.runtime_version()),
    );
    let domains =
        GuardDomains::one(GuardDomain::CommandEnvironment).with(GuardDomain::VariableTrace);
    let token = vm
        .prepare_command_guard("lassign", identity, domains)
        .expect("attested in the default release");
    assert!(vm.check_command_guard(token, "lassign"));

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    assert!(!vm.check_command_guard(token, "lassign"));
    assert_eq!(
        vm.prepare_command_guard("lassign", identity, domains),
        Err(GuardError::IdentityUnavailable)
    );

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert!(
        vm.prepare_command_guard("lassign", identity, domains)
            .is_ok()
    );
}

#[test]
fn command_trace_stales_the_string_intrinsic_guard() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let token = vm
        .prepare_command_guard(
            "string",
            identity,
            GuardDomains::one(GuardDomain::CommandTrace),
        )
        .unwrap();
    let added = vm.add_cmd_trace(
        false,
        "string",
        vec!["rename".to_owned()],
        "callback".to_owned(),
    );
    assert_eq!(added.code, Code::Ok);
    assert!(!vm.check_command_guard(token, "string"));
}

#[test]
fn stock_guard_reissues_after_unrelated_birth_but_not_worker_replacement() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandTrace);
    let original = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    // An unrelated command birth leaves the guard alone; a namespace birth
    // moves the structural domain every stock guard depends on.
    vm.register("unrelated", guarded_builtin);
    assert!(vm.check_command_guard(original, "string"));
    vm.declare_namespace_key("unrelated");
    assert!(!vm.check_command_guard(original, "string"));
    let fresh = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    assert!(vm.check_command_guard(fresh, "string"));
    vm.register("::tcl::string::length", guarded_builtin);
    assert!(!vm.check_command_guard(fresh, "string"));
    assert!(
        vm.prepare_command_guard("string", identity, domains)
            .is_err()
    );
    let index = tcl_registry::IntrinsicId::StringIndex;
    let index_identity = GuardIdentity::registry_intrinsic_with_semantics(
        index.stable_id(),
        index.guard_semantics_key(vm.runtime_version()),
    );
    let unrelated_worker = vm
        .prepare_command_guard("string", index_identity, domains)
        .unwrap();
    assert!(vm.check_command_guard(unrelated_worker, "string"));
}

#[test]
fn stock_guard_requires_original_ensemble_configuration_at_every_issuance() {
    let mut vm = Vm::new();
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        tcl_registry::IntrinsicId::StringLength.stable_id(),
        tcl_registry::IntrinsicId::StringLength.guard_semantics_key(vm.runtime_version()),
    );
    let domains = GuardDomains::one(GuardDomain::CommandTrace);
    let original = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    let Command::Ensemble(ensemble) = vm.commands.get("string").unwrap().clone() else {
        panic!("modern stock string is an actual ensemble");
    };
    let config = ensemble.config();
    let mut changed = config.clone();
    changed.prefixes = !changed.prefixes;
    ensemble.configure(changed);
    vm.note_ensemble_configuration_changed(&ensemble);
    assert!(!vm.check_command_guard(original, "string"));
    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::IdentityUnavailable)
    );
    ensemble.configure(config);
    vm.note_ensemble_configuration_changed(&ensemble);
    assert!(!vm.check_command_guard(original, "string"));
    let restored = vm
        .prepare_command_guard("string", identity, domains)
        .unwrap();
    assert!(vm.check_command_guard(restored, "string"));
    vm.register("string", guarded_builtin);
    assert!(!vm.check_command_guard(restored, "string"));
    assert_eq!(
        vm.prepare_command_guard("string", identity, domains),
        Err(GuardError::IdentityUnavailable)
    );
}

#[test]
fn runtime_version_threads_into_the_release_globals() {
    // §5.4 VM parity: the default is the 9.0.4 reference release the
    // VM's semantics are re-derived from…
    let vm = Vm::new();
    assert_eq!(vm.runtime_version(), tcl_dialect::TclVersion::V9_0);
    assert_eq!(
        vm.get_var("tcl_version").map(|v| v.to_str().to_string()),
        Some("9.0".to_owned())
    );
    assert_eq!(
        vm.get_var("tcl_patchLevel").map(|v| v.to_str().to_string()),
        Some("9.0.4".to_owned())
    );
    // …while a profile selects its own emulated core release. iRules is
    // an embedded Tcl 8.4 surface, so routing it through the profile must
    // also update the release globals rather than retaining the default.
    let mut vm = Vm::new();
    vm.set_runtime_version(tcl_dialect::DialectProfile::irules().vm_runtime_version);
    assert_eq!(vm.runtime_version(), tcl_dialect::TclVersion::V8_4);
    assert_eq!(
        vm.get_var("tcl_version").map(|v| v.to_str().to_string()),
        Some("8.4".to_owned())
    );
    assert_eq!(
        vm.get_var("tcl_patchLevel").map(|v| v.to_str().to_string()),
        Some("8.4.20".to_owned())
    );
}

#[test]
fn introspect_level_and_argv() {
    let mut vm = Vm::new();
    // Top level: depth 0, the global frame has no invoking call.
    assert_eq!(Introspect::level(&vm), 0);
    assert!(Introspect::level_argv(&vm, 0).unwrap().is_none());
    // A proc-call frame with its invoking words.
    vm.push_call_frame(
        Some("p".to_string()),
        vec![Value::string("p"), Value::string("x")],
    );
    assert_eq!(Introspect::level(&vm), 1);
    assert_eq!(
        &*Introspect::level_argv(&vm, 1).unwrap().unwrap().to_str(),
        "p x"
    );
    vm.pop_call_frame();
    assert_eq!(Introspect::level(&vm), 0);
}

#[test]
fn varstore_honours_frame_id() {
    let mut vm = Vm::new();
    // Write into the global frame while it is current.
    vm.set(GLOBAL_FRAME, "g", Value::string("global"));
    // Enter a proc-call frame; the global var is not in it.
    vm.push_call_frame(Some("p".to_string()), vec![Value::string("p")]);
    let here = FrameId(vm.current_level());
    assert_ne!(here, GLOBAL_FRAME);
    vm.set(here, "loc", Value::string("local"));
    // FrameId is honoured: each frame sees only its own var.
    assert_eq!(
        vm.get(GLOBAL_FRAME, "g").map(|v| v.to_str().to_string()),
        Some("global".to_string())
    );
    assert!(vm.get(here, "g").is_none());
    assert!(vm.exists(here, "loc"));
    assert!(!vm.exists(GLOBAL_FRAME, "loc"));
    // Reach back into the global frame from the child frame.
    vm.set(GLOBAL_FRAME, "g2", Value::string("two"));
    vm.pop_call_frame();
    assert_eq!(
        vm.get(GLOBAL_FRAME, "g2").map(|v| v.to_str().to_string()),
        Some("two".to_string())
    );
    assert!(vm.unset(GLOBAL_FRAME, "g2"));
    assert!(!vm.exists(GLOBAL_FRAME, "g2"));
}

#[test]
fn varstore_array_elements() {
    let mut vm = Vm::new();
    assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));
    vm.set_elem(GLOBAL_FRAME, "a", "k", Value::string("v"));
    assert!(vm.exists_elem(GLOBAL_FRAME, "a", "k"));
    assert_eq!(
        vm.get_elem(GLOBAL_FRAME, "a", "k")
            .map(|v| v.to_str().to_string()),
        Some("v".to_string())
    );
    assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "nope"));
    assert!(vm.unset_elem(GLOBAL_FRAME, "a", "k"));
    assert!(!vm.exists_elem(GLOBAL_FRAME, "a", "k"));

    // The pair is never recomposed and reparsed: every parenthesis shape is
    // a valid array base when the caller already supplied it separately.
    for base in ["z(b", "z)b", "z(b)c"] {
        vm.set_elem(GLOBAL_FRAME, base, "a", Value::string(base));
        assert_eq!(
            vm.get_elem(GLOBAL_FRAME, base, "a")
                .map(|value| value.to_str().to_string()),
            Some(base.to_owned())
        );
        assert!(vm.exists_elem(GLOBAL_FRAME, base, "a"));
        assert!(vm.unset_elem(GLOBAL_FRAME, base, "a"));
    }
}

#[test]
fn raw_scalar_replacement_retires_array_element_trace_cells() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    assert!(vm.set_array_elem("a", "k", Value::string("old")).is_ok());
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    assert_eq!(vm.var_arena.len(), baseline + 2);

    vm.write_scalar_raw("a", Value::string("scalar"));

    assert_eq!(vm.get_var("a").unwrap().to_str().as_ref(), "scalar");
    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline + 1);
}

#[test]
fn noncurrent_varstore_unset_retires_array_element_trace_cells() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    assert!(vm.set_array_elem("a", "k", Value::string("old")).is_ok());
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);
    assert!(vm.unset(GLOBAL_FRAME, "a"));

    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline);
    vm.pop_call_frame();
}

#[test]
fn noncurrent_varstore_unset_elem_retires_the_reached_trace_cell() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    vm.set_elem(GLOBAL_FRAME, "a", "k", Value::string("old"));
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);

    assert!(vm.unset_elem(GLOBAL_FRAME, "a", "k"));
    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline + 1, "the empty array remains");
    assert_eq!(vm.array_keys(GLOBAL_FRAME, "a"), Some(Vec::new()));
    vm.pop_call_frame();
}

#[test]
fn noncurrent_varstore_unset_reports_trace_only_cells_missing() {
    let mut vm = Vm::new();
    let baseline = vm.var_arena.len();
    vm.add_var_trace("x", vec!["unset".to_owned()], "callback".to_owned(), false);
    assert!(vm.ensure_array("a").is_ok());
    vm.add_var_trace(
        "a(k)",
        vec!["unset".to_owned()],
        "callback".to_owned(),
        false,
    );
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);

    assert!(!vm.unset(GLOBAL_FRAME, "x"));
    assert!(!vm.unset_elem(GLOBAL_FRAME, "a", "k"));
    assert!(vm.variable_observers.script_traces.is_empty());
    assert_eq!(vm.var_arena.len(), baseline + 1, "the empty array remains");
    vm.pop_call_frame();
}

#[test]
fn varstore_array_keys_honours_the_requested_frame() {
    let mut vm = Vm::new();
    vm.set_elem(GLOBAL_FRAME, "same", "global-a", Value::string("A"));
    vm.set_elem(GLOBAL_FRAME, "same", "global-b", Value::string("B"));
    vm.push_call_frame(Some("p".to_owned()), vec![Value::string("p")]);
    let local = FrameId(vm.current_level());
    vm.set_elem(local, "same", "local", Value::string("L"));

    assert!(vm.exists(GLOBAL_FRAME, "same"));
    assert!(vm.exists(local, "same"));
    // C's string-hash order: tclsh 8.4-9.1 report `global-b global-a`.
    assert_eq!(
        vm.array_keys(GLOBAL_FRAME, "same"),
        Some(vec!["global-b".to_owned(), "global-a".to_owned()])
    );
    assert_eq!(vm.array_keys(local, "same"), Some(vec!["local".to_owned()]));
    vm.pop_call_frame();
}

#[test]
fn native_double_strings_observe_precision_only_until_first_materialisation() {
    let source = "set tcl_precision 12; set x 1.0; set d [expr {$x/3}]; set tcl_precision 4; set first [list $d]; set tcl_precision 12; set second [list $d]; set e [expr {$x/3}]; expr {$e+0}; set tcl_precision 4; set third [list $e]; list $first $second $third";
    for profile in variable_link_profiles() {
        let mut vm = crate::native_fixture::interpreter(profile);
        let wanted = match profile.double_string_policy().unwrap() {
            tcl_dialect::DoubleStringPolicy::Tcl84Precision
            | tcl_dialect::DoubleStringPolicy::TclPrecision => "0.3333 0.3333 0.3333",
            tcl_dialect::DoubleStringPolicy::Shortest => {
                "0.3333333333333333 0.3333333333333333 0.3333333333333333"
            }
            tcl_dialect::DoubleStringPolicy::JimTwelve => {
                "0.333333333333 0.333333333333 0.333333333333"
            }
        };
        let result = vm.eval_source(source).unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(result.result.to_str().as_ref(), wanted, "{}", profile.name);
    }
}

#[test]
fn native_scan_and_binary_fields_preserve_lazy_double_representations() {
    let cases = [
        (
            "set tcl_precision 4; set d [expr {1.0/3}]; binary scan [binary format d $d] d copy; set tcl_precision 12; list $d $copy",
            "0.333333333333 0.333333333333",
            "0.3333333333333333 0.3333333333333333",
        ),
        (
            "set tcl_precision 4; set d [expr {1.0/3}]; binary scan [binary format d* [list $d $d]] d* copy; set tcl_precision 12; list $d $copy",
            "0.333333333333 {0.333333333333 0.333333333333}",
            "0.3333333333333333 {0.3333333333333333 0.3333333333333333}",
        ),
        (
            "set tcl_precision 12; scan 0.3333333333333333 %f d; set tcl_precision 4; list $d",
            "0.3333",
            "0.3333333333333333",
        ),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, precision_wanted, shortest_wanted) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let wanted = match profile.double_string_policy().unwrap() {
                tcl_dialect::DoubleStringPolicy::Shortest => shortest_wanted,
                _ => precision_wanted,
            };
            let result = vm.eval_source(source).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_observer_follows_storage_and_validates_writes() {
    let cases = [
        (
            "set tcl_precision 12; proc localPrecision {} {set tcl_precision 4; expr {1.0/3}}; localPrecision",
            "0.333333333333",
        ),
        (
            "set tcl_precision 4; upvar 0 tcl_precision alias; set alias 6; set status [catch {set tcl_precision 18} message]; list $status $message $tcl_precision",
            "1 {can't set \"tcl_precision\": improper value for precision} 6",
        ),
        (
            "set tcl_precision 4; set child [interp create -safe]; set status [$child eval {catch {set tcl_precision 8} message}]; set message [$child eval {set message}]; interp delete $child; list $status $message $tcl_precision",
            "1 {can't set \"tcl_precision\": can't modify precision from a safe interpreter} 4",
        ),
    ];
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, wanted) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let result = vm.eval_source(source).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_array_trace_runs_before_element_callbacks() {
    let cases = [
        (
            "set tcl_precision 12; unset tcl_precision; set tcl_precision(x) 12; upvar 0 tcl_precision(x) p; set p 4; set d [expr {1.0/3}]; list $d $p $tcl_precision(x)",
            "0.333333333333 4 12",
        ),
        (
            "set tcl_precision 12; unset tcl_precision; set tcl_precision(x) 12; proc change {name key op} {upvar #0 $name a; set a($key) 8}; trace variable tcl_precision(x) w change; set tcl_precision(x) 4; list [expr {1.0/3}] $tcl_precision(x)",
            "0.3333 4",
        ),
    ];
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        for (source, wanted) in cases {
            let mut vm = crate::native_fixture::interpreter(profile);
            let result = vm.eval_source(source).unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                result.result.to_str().as_ref(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn native_precision_is_shared_with_children_and_survives_unset() {
    let source = "set tcl_precision 4; set x 1.0; set d [expr {$x/7}]; set child [interp create]; $child eval {set tcl_precision 12}; set result [list $tcl_precision $d]; interp delete $child; unset tcl_precision; list $result [info exists tcl_precision] $tcl_precision";
    for version in [
        tcl_dialect::TclVersion::V8_4,
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut vm = crate::native_fixture::interpreter(profile);
        let result = vm.eval_source(source).unwrap();
        assert_eq!(
            result.code,
            Code::Ok,
            "{}: {}",
            profile.name,
            result.result.to_str()
        );
        assert_eq!(
            result.result.to_str().as_ref(),
            "{12 0.142857142857} 1 12",
            "{}",
            profile.name
        );
    }
}

#[test]
fn native_variable_updates_retain_exact_keys_and_failed_read_options() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"k\0z", Value::int(7)).unwrap();
        let increment = vm
            .increment_captured_bytes(b"k\0z", None, &Value::int(2))
            .unwrap();
        assert_eq!(increment.value.as_int().unwrap(), 9);
        assert_eq!(vm.get_var_bytes(b"k\0z").unwrap().as_int().unwrap(), 9);
        assert!(crate::command::opt_get(&increment.options, "-errorcode").is_none());
        let appended = vm
            .lappend_list_update_bytes(b"missing", None, &[Value::string("x")])
            .unwrap();
        assert_eq!(appended.value.string_bytes().as_ref(), b"x");
        let code = crate::command::opt_get(&appended.options, "-errorcode");
        if *version >= tcl_dialect::TclVersion::V8_6 {
            assert_eq!(code.unwrap().string_bytes().as_ref(), b"TCL READ VARNAME");
        } else {
            assert!(
                code.is_none(),
                "{version:?}: {:?}",
                code.map(|code| code.to_str().to_string())
            );
        }
        vm.set_array_elem_bytes(b"a", b"key\0tail", Value::int(3))
            .unwrap();
        let increment = vm
            .increment_captured_bytes(b"a", Some(b"key\0tail"), &Value::int(4))
            .unwrap();
        assert_eq!(increment.value.as_int().unwrap(), 7);
        assert_eq!(
            vm.get_array_elem_bytes_from(0, b"a", b"key\0tail")
                .unwrap()
                .as_int()
                .unwrap(),
            7
        );
    }
}

#[test]
fn native_read_result_reports_the_actual_failure_site_and_operand_form() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = Vm::new();
        vm.set_runtime_version(*version);
        vm.ensure_array("a").unwrap();
        let error = vm
            .read_variable_result_bytes(b"a(k\0tail)", None)
            .unwrap_err();
        assert_eq!(
            error.result.string_bytes().as_ref(),
            b"can't read \"a(k\": no such element in array"
        );
        if *version >= tcl_dialect::TclVersion::V8_6 {
            assert_eq!(
                crate::command::opt_get(&error.options, "-errorcode")
                    .unwrap()
                    .string_bytes()
                    .as_ref(),
                b"TCL READ VARNAME"
            );
        }
        let error = vm
            .read_variable_result_bytes(b"a", Some(b"k\0tail"))
            .unwrap_err();
        assert_eq!(
            error.result.string_bytes().as_ref(),
            b"can't read \"a(k)\": no such element in array"
        );
    }
}

#[test]
fn native_variable_bytes_preserve_full_keys_and_nul_qualification() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"k\0z", Value::string("FULL")).unwrap();
        vm.set_var_bytes(b"N\0z::k", Value::string("LATE")).unwrap();
        assert_eq!(
            vm.get_var_bytes(b"k\0z").unwrap().string_bytes().as_ref(),
            b"FULL"
        );
        assert_eq!(
            vm.get_var_bytes(b"N\0z::k")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"LATE"
        );
        assert_eq!(
            vm.get_var_bytes(b"k").is_some(),
            *version == tcl_dialect::TclVersion::V8_4
        );
        vm.set_var_bytes(b"\xff", Value::string("INVALID")).unwrap();
        assert_eq!(
            vm.get_var_bytes(b"\xff").unwrap().string_bytes().as_ref(),
            b"INVALID"
        );
        assert!(
            vm.frame_var_name_bytes(true)
                .iter()
                .any(|name| name == b"\xff")
        );
    }
}

#[test]
fn native_combined_and_separate_array_inputs_select_distinct_receivers() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"a\0z(k)", Value::string("COMBINED"))
            .unwrap();
        vm.set_array_elem_bytes_from(0, b"a\0z", b"k", Value::string("SEPARATE"), false)
            .unwrap();
        let combined = vm.get_var_bytes(b"a\0z(k)").unwrap();
        assert_eq!(
            combined.string_bytes().as_ref(),
            if *version >= tcl_dialect::TclVersion::V9_0 {
                b"COMBINED".as_slice()
            } else {
                b"SEPARATE".as_slice()
            }
        );
        assert_eq!(
            vm.get_array_elem_bytes_from(0, b"a\0z", b"k")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"SEPARATE"
        );
        vm.set_var_bytes(b"b(k\0z)", Value::string("ELEMENT"))
            .unwrap();
        let target = VarStore::array_target_bytes(&vm, FrameId(0), b"b").unwrap();
        let keys = VarStore::array_key_bytes_checked_at(&vm, &target)
            .unwrap()
            .unwrap();
        assert_eq!(
            keys,
            vec![if *version >= tcl_dialect::TclVersion::V9_0 {
                b"k\0z".to_vec()
            } else {
                b"k".to_vec()
            }]
        );
    }
}

#[test]
fn native_variable_links_keep_byte_names_and_retired_cell_identity() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        vm.set_var_bytes(b"x", Value::string("OLD")).unwrap();
        vm.push_call_frame(Some("P".into()), Vec::new());
        vm.link_upvar_bytes(0, b"x", b"a\0z")
            .unwrap_or_else(|_| panic!("byte upvar publication succeeds"));
        assert_eq!(
            vm.get_var_bytes(b"a\0z").unwrap().string_bytes().as_ref(),
            b"OLD"
        );
        assert_eq!(
            vm.get_var_bytes(b"a").is_some(),
            *version == tcl_dialect::TclVersion::V8_4
        );
        let target = vm
            .resolve_var_from_bytes(b"a\0z", vm.current_level())
            .unwrap()
            .id
            .unwrap();
        vm.unset_from(0, "x");
        vm.write_scalar_from(0, "x", Value::string("NEW"));
        assert_eq!(
            vm.resolve_var_from_bytes(b"a\0z", vm.current_level())
                .unwrap()
                .id,
            Some(target)
        );
        assert_eq!(
            vm.get_var_bytes(b"a\0z").unwrap().string_bytes().as_ref(),
            b"NEW"
        );
        vm.pop_call_frame();
    }
}

#[test]
fn native_trace_registration_projects_its_own_name_and_keeps_prefix_value() {
    for version in &tcl_dialect::TclVersion::ALL {
        let mut vm = release_vm(*version);
        let prefix = Value::from_string_bytes(b"capture\xff".as_slice());
        vm.ensure_trace_variable_bytes(b"k\0z").unwrap();
        vm.add_var_trace_bytes(b"k\0z", vec!["read".into()], &prefix, false);
        let registered = vm.registered_trace_cell_bytes(b"k\0z").unwrap().id.unwrap();
        assert_eq!(
            vm.registered_trace_cell_bytes(b"k").unwrap().id,
            Some(registered)
        );
        assert_eq!(
            vm.trace_cell_bytes(b"k\0z")
                .and_then(|cell| cell.id)
                .is_some(),
            *version == tcl_dialect::TclVersion::V8_4
        );
        let entries = vm.var_trace_info_bytes(b"k\0z");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].1.string_bytes().as_ref(), b"capture\xff");
        vm.remove_var_trace_bytes(b"k\0z", &["read".into()], b"capture\xff");
        assert!(vm.var_trace_info_bytes(b"k").is_empty());
    }
}

#[test]
fn confined_jim_dictionary_lappend_refuses_foreign_write_and_preserves_reads() {
    let mut vm = Vm::new();
    let jim = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    vm.set_dialect_profile(jim);
    assert!(vm.dictionary_variable_containers());
    vm.set_var("::held", Value::string("one")).expect("seed");
    vm.set_stores_confined(true);
    assert!(
        vm.lappend_list_update_bytes(b"::held", None, &[Value::string("two")])
            .is_err()
    );
    assert_eq!(vm.get_var("::held").unwrap().to_str().as_ref(), "one");
    let read = vm
        .lappend_list_update_bytes(b"::held", None, &[])
        .expect("read-only");
    assert_eq!(read.value.to_str().as_ref(), "one");
    assert!(
        vm.lappend_list_update_bytes(b"::missing", None, &[])
            .is_err()
    );
    assert!(vm.get_var("::missing").is_none());
}

/// The selected engine with the scripted distribution its shell loads, so
/// Jim's `namespace` helpers are present as they are under jimsh.
fn variable_link_vm(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    let mut vm = crate::native_fixture::interpreter(profile);
    crate::native_fixture::install_distribution(&mut vm);
    vm
}

fn variable_link_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    let jim = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    assert_eq!(
        jim.core_point
            .map(tcl_dialect::model::DialectPoint::release),
        Some(tcl_dialect::model::Release::JIM_0_84),
        "the VM fixture and pinned native oracle must select the same release"
    );
    profiles.push(jim);
    profiles
}

#[test]
fn variable_links_follow_authored_dialect_lifetimes() {
    let cases = [
        (
            "set x OLD; upvar 0 x y; unset x; set y NEW; list $x $y",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "set x OLD; upvar 0 x y; unset x; set x NEW; list $x $y",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "set results {}; upvar 0 x y; foreach n {1 2 3} {set x $n; lappend results $y; unset x}; set results",
            "1 2 3",
            "1 2 3",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a; set a(k) NEW; list [catch {set y} error] $error $a(k)",
            "1 {can't read \"y\": no such variable} NEW",
            "0 NEW NEW",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a(k); set a(k) NEW; list $y $a(k)",
            "NEW NEW",
            "NEW NEW",
        ),
        (
            "namespace eval ::life {variable x OLD}; upvar #0 ::life::x y; namespace delete ::life; namespace eval ::life {variable x NEW}; list [catch {set y} error] $error $::life::x",
            "1 {can't read \"y\": no such variable} NEW",
            "0 NEW NEW",
        ),
        (
            "namespace eval ::life2 {variable x OLD}; upvar #0 ::life2::x y; unset ::life2::x; set y NEW; list $y $::life2::x",
            "NEW NEW",
            "NEW NEW",
        ),
    ];
    for profile in variable_link_profiles() {
        for (source, cell_want, name_want) in cases {
            let mut vm = variable_link_vm(profile);
            let wanted = match profile.variable_link_binding().unwrap() {
                tcl_dialect::VariableLinkBinding::StableCell => cell_want,
                tcl_dialect::VariableLinkBinding::SelectedFrameName => name_want,
            };
            let result = vm
                .eval_source(&format!("proc probe {{}} {{{source}}}; probe"))
                .unwrap();
            assert_eq!(
                result.code,
                Code::Ok,
                "{}: {}",
                profile.name,
                result.result.to_str()
            );
            assert_eq!(
                &*result.result.to_str(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn variable_link_target_rebinding_and_materialisation_follow_the_policy() {
    let cases = [
        (
            "set x OLD; set z NEW; upvar 0 x a; upvar 0 a b; upvar 0 z a; list $a $b",
            "0 {NEW OLD}",
            "0 {NEW NEW}",
        ),
        (
            "upvar 0 a(k) y; list [info exists a] [array exists a] [info exists y]",
            "0 {1 1 0}",
            "0 {0 0 0}",
        ),
        (
            "set a(k) OLD; upvar 0 a(k) y; unset a; set y NEW; list $a(k) $y",
            "1 {can't set \"y\": upvar refers to element in deleted array}",
            "0 {NEW NEW}",
        ),
        (
            "upvar 0 x x",
            "1 {can't upvar from variable to itself}",
            "1 {can't upvar from variable to itself}",
        ),
        (
            "upvar 0 a b; upvar 0 b a",
            "1 {can't upvar from variable to itself}",
            "1 {can't upvar from variable to itself}",
        ),
    ];
    for profile in variable_link_profiles() {
        for (source, cell_want, name_want) in cases {
            let mut vm = variable_link_vm(profile);
            let result = vm
                .eval_source(&format!(
                    "proc probe {{}} {{{source}}}; list [catch probe result] $result"
                ))
                .unwrap();
            let wanted = if profile.variable_link_binding()
                == Some(tcl_dialect::VariableLinkBinding::StableCell)
            {
                cell_want
            } else {
                name_want
            };
            assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
            assert_eq!(
                &*result.result.to_str(),
                wanted,
                "{}: {source}",
                profile.name
            );
        }
    }
}

#[test]
fn namespace_var_enumeration_uses_the_exact_retained_token() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(
        eval_value(
            &mut vm,
            "namespace eval N {\
                     variable old OLD;\
                     proc p {} {\
                         namespace delete ::N;\
                         namespace eval ::N {variable fresh NEW};\
                         yield paused\
                     }\
                 }",
        ),
        ""
    );
    let old = Namespaces::find_namespace(&vm, ROOT_NS, "::N").unwrap();
    assert_eq!(eval_value(&mut vm, "coroutine c ::N::p"), "paused");
    let fresh = Namespaces::find_namespace(&vm, ROOT_NS, "::N").unwrap();

    assert_ne!(old, fresh);
    assert_eq!(Namespaces::vars_in(&vm, old), vec!["old".to_owned()]);
    assert_eq!(Namespaces::vars_in(&vm, fresh), vec!["fresh".to_owned()]);

    assert_eq!(eval_value(&mut vm, "c"), "");
}

#[test]
fn commands_dispatch_builtin_and_unknown() {
    let mut vm = Vm::new();
    // A builtin runs inline and yields its result.
    let c = vm
        .dispatch("list", &[Value::string("a"), Value::string("b c")])
        .unwrap();
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "a {b c}");
    // An unknown command name is an error completion.
    let c = vm.dispatch("no_such_command", &[]).unwrap();
    assert_eq!(c.code, Code::Error);
    assert_eq!(
        &*c.result.to_str(),
        "invalid command name \"no_such_command\""
    );
}

#[test]
fn frames_push_pop_current_link() {
    let mut vm = Vm::new();
    let to_s = |v: Option<Value>| v.map(|v| v.to_str().to_string());
    // A global, set while the global frame is current.
    vm.set(GLOBAL_FRAME, "g", Value::string("orig"));
    let outer = Frames::current(&vm);
    assert_eq!(outer, GLOBAL_FRAME);
    // Push a proc-call frame in a fresh namespace (interned to an NsId, which
    // round-trips through the arena back to its name on push).
    let ns = vm.intern_ns("foo");
    let inner = Frames::push(&mut vm, ns);
    assert_ne!(inner, outer);
    assert_eq!(Frames::current(&vm), inner);
    // `upvar`: link `gg` (inner) to the outer frame's global `g`; reads and
    // writes through `gg` reach `g`.
    Frames::link(&mut vm, inner, outer, "gg", "g");
    assert_eq!(to_s(vm.get(inner, "gg")), Some("orig".to_string()));
    vm.set(inner, "gg", Value::string("changed"));
    // Pop back to the global frame: the link is gone, the global updated.
    Frames::pop(&mut vm);
    assert_eq!(Frames::current(&vm), outer);
    assert_eq!(to_s(vm.get(GLOBAL_FRAME, "g")), Some("changed".to_string()));
}

#[test]
fn namespaces_current_and_find_command() {
    let mut vm = Vm::new();
    // At the top level the current namespace is the global root.
    assert_eq!(Namespaces::current(&vm), ROOT_NS);
    // Builtins resolve from the global namespace to stable, distinct ids.
    let a = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
    assert_eq!(a, Namespaces::find_command(&vm, ROOT_NS, "list").unwrap());
    assert_ne!(
        a,
        Namespaces::find_command(&vm, ROOT_NS, "llength").unwrap()
    );
    assert!(Namespaces::find_command(&vm, ROOT_NS, "no_such_command").is_none());
    // `current` tracks the namespace pushed by `Frames::push`.
    let foo = vm.intern_ns("foo");
    Frames::push(&mut vm, foo);
    assert_eq!(Namespaces::current(&vm), foo);
    Frames::pop(&mut vm);
    assert_eq!(Namespaces::current(&vm), ROOT_NS);
}

#[test]
fn commands_dispatch_id_composes() {
    let mut vm = Vm::new();
    // Resolve a command to a handle, then invoke it *by that handle* — the
    // find_command -> dispatch_id composition.
    let id = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
    let c = vm
        .dispatch_id(id, &[Value::string("a"), Value::string("b")])
        .unwrap();
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "a b");
    // A fabricated id yields an error completion (no such command).
    let c = vm.dispatch_id(CommandId(9999), &[]).unwrap();
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "invalid command id");
}

#[test]
fn smoke_eval_canonical_snippet() {
    let mut vm = Vm::new();
    let result = vm
        .eval_expr("21 * 2")
        .expect("canonical expr must evaluate");
    assert_eq!(result.to_str().as_ref(), "42");
}
