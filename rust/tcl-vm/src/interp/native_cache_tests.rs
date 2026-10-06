// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual raw command attachments and native compiler epoch transitions.

use super::*;
use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;

fn native_vm(engine: &str) -> Vm {
    Vm::with_native_core(
        Box::new(Vec::<u8>::new()),
        Rc::new(crate::host_native::NativeHost::new()),
        tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual native core")
}

fn invoke(vm: &mut Vm, command: &str, arguments: &[&str]) -> Code {
    let words = arguments
        .iter()
        .map(|word| Value::string(*word))
        .collect::<Vec<_>>();
    vm.invoke_command(command, &words).code
}

fn native_epoch_baseline(reports: &str) -> u64 {
    reports
        .lines()
        .nth(1)
        .unwrap()
        .split('\t')
        .nth(2)
        .unwrap()
        .parse::<u64>()
        .unwrap()
}

#[test]
fn nested_info_compilation_retains_original_maps_and_late_worker_binding() {
    use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;

    fn replacement(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::string("REPLACED"))
    }

    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native_vm(engine);
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(vm.source_profile()),
        ));
        assert_eq!(invoke(&mut vm, "oo::class", &["create", "C"]), Code::Ok);
        let source = tcl_lexer::SourceImage::native(b"info class superclasses ::C".as_slice());
        let unit = vm.compile_script_cached_bytes(&source).unwrap();
        let required = unit
            .asm
            .native_compiler_prerequisites
            .iter()
            .find_map(|prerequisite| match prerequisite {
                NativeCompilerSelectionPrerequisite::Ensemble(required)
                    if !required.nested_compilers.is_empty() =>
                {
                    Some(std::sync::Arc::clone(required))
                }
                _ => None,
            })
            .expect("original nested info compiler registrations");
        assert_eq!(required.invocation_word.as_bytes(), b"info", "{engine}");
        let [inner] = required.nested_compilers.as_slice() else {
            panic!("one actual nested InfoClass ensemble");
        };
        assert_eq!(
            inner.invocation_word.as_bytes(),
            b"::oo::InfoClass",
            "{engine}"
        );
        assert!(vm.native_ensemble_compiler_prerequisite_matches(&required));

        // The captured final invocation is late bound; replacing its handler
        // leaves the original public and intermediate compiler registrations.
        vm.register("::oo::InfoClass::superclasses", replacement);
        assert!(vm.native_ensemble_compiler_prerequisite_matches(&required));
        let completion = vm.run_compiled_unit(unit);
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert_eq!(
            completion.result.string_bytes().as_ref(),
            b"REPLACED",
            "{engine}"
        );

        let Command::Ensemble(token) = vm.commands.get("oo::InfoClass").unwrap().clone() else {
            panic!("actual private InfoClass ensemble");
        };
        let mut configuration = token.config();
        configuration.prefixes = !configuration.prefixes;
        token.configure(configuration);
        assert!(
            !vm.native_ensemble_compiler_prerequisite_matches(&required),
            "{engine}"
        );
    }
}

fn raw_hook(vm: &Vm, key: &str) -> i32 {
    let key = CommandSidecarKey::visible(key);
    if vm.command_at_sidecar_key(&key).is_none() {
        return -1;
    }
    match vm.native_hook_at_sidecar(&key) {
        Hook::Absent => 0,
        Hook::Present => 1,
        Hook::Unknown => panic!("raw native attachment unavailable at {key:?}"),
    }
}

#[test]
fn file_worker_runtime_headers_match_all_190_original_native_snapshots() {
    let fixtures = [
        (
            "tcl8.4",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/8.4.20.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/8.5.19.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/8.6.18.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/9.0.4.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tcl-registry/tests/data/native_file_compilers/9.1.0.tsv"),
        ),
    ];
    let mut checked = 0;
    for (engine, fixture) in fixtures {
        let vm = native_vm(engine);
        for row in fixture.lines().skip(1) {
            let (identity, native_hook) = row.split_once('\t').unwrap();
            assert_eq!(
                raw_hook(&vm, identity.trim_start_matches("::")),
                native_hook.parse::<i32>().unwrap(),
                "{engine}/{identity}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 190);
}

const IMPORTED_COMPILER_FIXTURES: [(&str, &str); 5] = [
    (
        "tcl8.4",
        include_str!("../../tests/data/native_import_hook_copy/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../tests/data/native_import_hook_copy/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../tests/data/native_import_hook_copy/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../tests/data/native_import_hook_copy/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../tests/data/native_import_hook_copy/9.1.0.tsv"),
    ),
];

fn check_imported_compiler_snapshot(vm: &Vm, engine: &str, fields: &[&str]) {
    let event = fields[0];
    let hooks = [
        raw_hook(vm, "N::e"),
        raw_hook(vm, "old"),
        raw_hook(vm, "fresh"),
    ];
    for (column, actual) in hooks.into_iter().enumerate() {
        assert_eq!(
            actual,
            fields[column + 3].parse::<i32>().unwrap(),
            "{engine} {event}: raw hook {column}"
        );
    }
    for name in ["old", "fresh"] {
        if raw_hook(vm, name) < 0 {
            continue;
        }
        let binding = vm
            .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from(name))
            .expect("actual imported compiler lookup row");
        assert_eq!(
            binding.token,
            vm.visible_command_generation(name).unwrap(),
            "{engine} {event}: original imported token"
        );
        assert_eq!(
            binding.slot.simple.as_bytes(),
            name.as_bytes(),
            "{engine} {event}: original imported slot"
        );
        assert_eq!(
            binding.compiler_hook,
            vm.native_hook_at_sidecar(&CommandSidecarKey::visible(name)),
            "{engine} {event}: copied compiler function"
        );
        assert!(
            matches!(
                binding.implementation,
                tcl_runtime_api::native_compilation::NativeCommandImplementation::Imported { .. }
            ),
            "{engine} {event}: separate callable origin"
        );
    }
    for (column, imported) in [hooks[1], hooks[2]].into_iter().enumerate() {
        let same = if hooks[0] < 0 || imported < 0 {
            -1
        } else {
            i32::from(hooks[0] == imported)
        };
        assert_eq!(
            same,
            fields[column + 6].parse::<i32>().unwrap(),
            "{engine} {event}: compiler function copy {column}"
        );
    }
}

type ImportedSourceEnsemble = Rc<tcl_cmd_core::ensemble::EnsembleToken<EnsembleDef, NameBytes>>;

fn imported_source_ensemble(
    vm: &mut Vm,
    engine: &str,
    namespace: NsId,
) -> Option<ImportedSourceEnsemble> {
    if engine == "tcl8.4" {
        assert_eq!(invoke(vm, "proc", &["N::e", "", "return SOURCE"]), Code::Ok);
        None
    } else {
        let token = Rc::new(tcl_cmd_core::ensemble::EnsembleToken::new(
            EnsembleDef {
                originals: crate::command::native_ensemble_objects::NativeEnsembleObjects::default(
                ),
                native: None,
                namespace,
                map: vec![(NameBytes::from("x"), vec![Some(NameBytes::from("::list"))])],
                subcommands: None,
                prefixes: true,
                parameters: Vec::new(),
                unknown: None,
            },
            NameBytes::from("::N::e"),
        ));
        vm.register_namespace_ensemble("N::e", &token);
        Some(token)
    }
}

#[test]
fn imported_compiler_attachments_match_all_57_original_native_snapshots() {
    let engines = IMPORTED_COMPILER_FIXTURES;
    let mut completed = 0;
    for (engine, reports) in engines {
        let mut vm = native_vm(engine);
        vm.declare_namespace_exports("N", &["e"]);
        let namespace = vm.namespace_id_from_written(ROOT_NS, "N").unwrap();
        let ensemble = imported_source_ensemble(&mut vm, engine, namespace);
        let mut baseline = None;
        let native_baseline = native_epoch_baseline(reports);
        for row in reports.lines().skip(1) {
            let fields = row.split('\t').collect::<Vec<_>>();
            let event = fields[0];
            let code = match event {
                "import_absent" | "import_present" => {
                    vm.import_commands(b"::N::e", false).unwrap();
                    invoke(
                        &mut vm,
                        "rename",
                        &[
                            "e",
                            if event == "import_absent" {
                                "old"
                            } else {
                                "fresh"
                            },
                        ],
                    )
                }
                "attach_source" | "reattach_source" | "detach_source" => {
                    vm.set_native_ensemble_compiler_attachment(
                        ensemble.as_ref().unwrap(),
                        event != "detach_source",
                    )
                    .unwrap();
                    Code::Ok
                }
                "configure_unhooked" | "configure_hooked" => invoke(
                    &mut vm,
                    "namespace",
                    &["ensemble", "configure", "::N::e", "-prefixes", "0"],
                ),
                "rename_old" => {
                    assert_eq!(invoke(&mut vm, "rename", &["old", "moved"]), Code::Ok);
                    invoke(&mut vm, "rename", &["moved", "old"])
                }
                "hide_old" | "hide_fresh" => {
                    let (name, hidden) = if event == "hide_old" {
                        ("old", "hiddenOld")
                    } else {
                        ("fresh", "hiddenFresh")
                    };
                    vm.hide_command(name, hidden).unwrap();
                    Code::Ok
                }
                "expose_old" | "expose_fresh" => {
                    let (hidden, name) = if event == "expose_old" {
                        ("hiddenOld", "old")
                    } else {
                        ("hiddenFresh", "fresh")
                    };
                    vm.expose_own_command(hidden, name).unwrap();
                    Code::Ok
                }
                "delete_source" => invoke(&mut vm, "rename", &["::N::e", ""]),
                _ => panic!("unhandled native event {event}"),
            };
            assert_eq!(
                code,
                Code::from_int(fields[1].parse().unwrap()),
                "{engine} {event}: completion"
            );
            let epoch = vm.native_cache_stamp(ROOT_NS).interpreter_epoch.0;
            let baseline = *baseline.get_or_insert(epoch);
            assert_eq!(
                epoch - baseline,
                fields[2].parse::<u64>().unwrap() - native_baseline,
                "{engine} {event}: compiler epoch"
            );
            check_imported_compiler_snapshot(&vm, engine, &fields);
            completed += 1;
        }
    }
    assert_eq!(completed, 57);
}

#[test]
fn imported_builtin_keeps_its_raw_compiler_after_origin_handler_replacement() {
    fn replacement(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::string("REPLACED"))
    }

    let mut vm = native_vm("tcl8.6");
    vm.declare_namespace_exports("N", &["set"]);
    assert_eq!(invoke(&mut vm, "rename", &["set", "N::set"]), Code::Ok);
    vm.import_commands(b"::N::set", false).unwrap();
    let original = vm
        .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from("set"))
        .unwrap();
    assert_eq!(original.compiler_hook, Hook::Present);
    assert_eq!(original.compiler.as_ref().unwrap().registry_identity, "set");
    vm.register_command("N::set", Command::Builtin(replacement));
    let current = vm
        .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from("set"))
        .unwrap();
    assert_eq!(raw_hook(&vm, "N::set"), 0);
    assert_eq!(current.token, original.token);
    assert_eq!(current.compiler_hook, Hook::Present);
    assert_eq!(current.compiler, original.compiler);
    assert!(matches!(
        current.implementation,
        tcl_runtime_api::native_compilation::NativeCommandImplementation::Imported { .. }
    ));
}

#[test]
fn operation_sites_keep_raw_imported_compilers_and_replay_failed_prerequisites() {
    use std::sync::Arc;
    use tcl_bytecode::{Instruction, NativeOperationSelectionSite, Op, Operand};
    use tcl_runtime_api::native_compilation::{
        NativeCommandCompilerPrerequisite, NativeCompilerSelectionPrerequisite,
    };
    use tcl_runtime_api::{CommandBindingGuard, CommandBindingIdentity};

    fn replacement(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::string("REPLAYED"))
    }

    for guard in [
        CommandBindingGuard::ChunkEntry,
        CommandBindingGuard::BeforeArguments,
    ] {
        let mut vm = native_vm("tcl8.6");
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(vm.source_profile()),
        ));
        vm.declare_namespace_exports("N", &["list"]);
        assert_eq!(invoke(&mut vm, "rename", &["list", "N::list"]), Code::Ok);
        vm.import_commands(b"::N::list", false).unwrap();
        let binding = vm
            .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from("list"))
            .unwrap();
        let required = Arc::new(NativeCommandCompilerPrerequisite {
            interpreter: vm.native_interpreter_identity(),
            lookup_namespace_token: u64::from(ROOT_NS.0),
            invocation_word: NameBytes::from("list"),
            slot: binding.slot,
            namespace_token: binding.namespace_token,
            token: binding.token,
            implementation_generation: binding.implementation_generation,
            compiler: binding.compiler.unwrap(),
            selected_worker: None,
            nested_compilers: Vec::new(),
            guard,
        });
        vm.register_command("N::list", Command::Builtin(replacement));
        assert!(vm.native_compiler_selection_prerequisite_matches(
            &NativeCompilerSelectionPrerequisite::Command(Arc::clone(&required)),
        ));

        for (stale, auxiliary, expected) in [
            (false, false, "VALUE"),
            (true, false, "REPLAYED"),
            (false, true, "REPLAYED"),
        ] {
            let mut receipt = (*required).clone();
            if stale {
                receipt.token = receipt.token.checked_add(1).unwrap();
            }
            let mut asm = FunctionAsm::default();
            let literal = asm.literals.intern("VALUE");
            let mut first = Instruction::new(
                Op::PUSH1,
                vec![Operand::Imm(i32::try_from(literal).unwrap())],
            );
            first.offset = 0;
            first
                .native_operation_selections
                .push(NativeOperationSelectionSite {
                    compiler_prerequisite: Some(Arc::new(receipt)),
                    requirements: if auxiliary {
                        vec![CommandBindingIdentity::new("list", "set").with_guard(guard)]
                    } else {
                        Vec::new()
                    },
                    guard,
                    end: "end".into(),
                    source: tcl_lexer::SourceImage::document("list VALUE"),
                    span: tcl_lexer::Span::new(0, 10),
                    namespace: tcl_runtime_api::ByteNamespacePath::root(),
                    namespace_context: None,
                });
            let mut done = Instruction::new(Op::DONE, Vec::new());
            done.offset = 2;
            asm.instructions = vec![first, done];
            asm.labels.insert("end".into(), 3);
            assert!(asm.validate_native_compilation_entry().is_ok());
            let mut mismatched_guard = asm.clone();
            mismatched_guard.instructions[0].native_operation_selections[0].guard = match guard {
                CommandBindingGuard::ChunkEntry => CommandBindingGuard::BeforeArguments,
                CommandBindingGuard::BeforeArguments => CommandBindingGuard::ChunkEntry,
            };
            assert!(
                mismatched_guard
                    .validate_native_compilation_entry()
                    .is_err()
            );
            let unit = vm.compiled_unit(Rc::new(asm), vm.source_namespace_path());
            let completion = vm.run_compiled_unit(unit);
            assert_eq!(
                completion.code,
                Code::Ok,
                "{guard:?} stale={stale} auxiliary={auxiliary}"
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected.as_bytes()
            );
        }
    }
}

#[test]
fn imported_compiler_failure_retains_raw_registration_before_any_body_store() {
    fn replacement(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::string("REPLACED"))
    }

    let mut vm = native_vm("tcl8.4");
    vm.set_compiler(Box::new(
        tcl_compiler::compile_service::BytecodeCompileService::for_profile(vm.source_profile()),
    ));
    vm.declare_namespace_exports("N", &["set"]);
    assert_eq!(invoke(&mut vm, "rename", &["set", "N::set"]), Code::Ok);
    vm.import_commands(b"::N::set", false).unwrap();
    vm.register_command("N::set", Command::Builtin(replacement));
    let body = Value::string("set earlier 1; set x extra bad");
    let unit = vm
        .prepare_procedure_body_value(None, &[], ROOT_NS, &body)
        .unwrap();
    assert!(unit.asm.native_compilation_failure.is_some());
    assert!(
        !unit
            .asm
            .command_bindings
            .iter()
            .any(|binding| binding.name == "set")
    );
    let [required] = unit.asm.native_compiler_prerequisites.as_slice() else {
        panic!("actual raw set compiler failure prerequisite");
    };
    let tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(required) =
        required
    else {
        panic!("ordinary set compiler prerequisite");
    };
    assert_eq!(required.invocation_word.as_bytes(), b"set");
    assert_eq!(required.compiler.registry_identity, "set");
    assert!(vm.function_command_bindings_match(&unit.asm));

    let mut stale = (*unit.asm).clone();
    let tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(required) =
        &mut stale.native_compiler_prerequisites[0]
    else {
        unreachable!();
    };
    let stale_token = required.token.checked_add(1).unwrap();
    std::sync::Arc::make_mut(required).token = stale_token;
    assert!(!vm.function_command_bindings_match(&stale));
    assert!(matches!(
        vm.try_run_function(&stale),
        Err(tcl_runtime_api::NativeExecutionError::CompilationAdmission(
            tcl_runtime_api::NativeCompilationAdmissionError::NativePreflightRequired,
        )),
    ));
    let completion = vm.run_compiled_unit(unit);
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.string_bytes().as_ref(),
        b"wrong # args: should be \"set varName ?newValue?\"",
    );
    assert!(vm.get_var("earlier").is_none());
}

#[test]
fn emitted_array_operation_retains_actual_ensemble_compiler() {
    for engine in ["tcl9.0", "tcl9.1"] {
        let mut vm = native_vm(engine);
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(vm.source_profile()),
        ));
        let source = tcl_lexer::SourceImage::native(b"array set payload {}".as_slice());
        let unit = vm.compile_script_cached_bytes(&source).unwrap();
        assert_eq!(unit.asm.validate_native_compilation_entry(), Ok(()));
        let actual = unit
            .asm
            .instructions
            .iter()
            .flat_map(|instruction| &instruction.native_operation_selections)
            .filter_map(tcl_bytecode::NativeOperationSelectionSite::compiler_selection_prerequisite)
            .find(|required| matches!(
                required,
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(_)
            ))
            .expect("array set retains its actual public ensemble compiler");
        assert!(vm.native_compiler_selection_prerequisite_matches(&actual));
        let completion = vm.run_compiled_unit(unit);
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert!(completion.result.string_bytes().is_empty(), "{engine}");
        let completion = vm.invoke_command(
            "array",
            &[Value::string("exists"), Value::string("payload")],
        );
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert_eq!(completion.result.string_bytes().as_ref(), b"1", "{engine}");
    }
}

fn ensemble_operation_asm(
    required: std::sync::Arc<
        tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    >,
    worker: tcl_runtime_api::CommandBindingIdentity,
    guard: tcl_runtime_api::CommandBindingGuard,
    require_worker: bool,
) -> tcl_bytecode::FunctionAsm {
    let mut asm = tcl_bytecode::FunctionAsm::default();
    let literal = asm.literals.intern("VALUE");
    let mut first = tcl_bytecode::Instruction::new(
        tcl_bytecode::Op::PUSH1,
        vec![tcl_bytecode::Operand::Imm(i32::try_from(literal).unwrap())],
    );
    first.offset = 0;
    first
        .native_operation_selections
        .push(tcl_bytecode::NativeOperationSelectionSite {
            compiler_prerequisite: Some(required),
            requirements: if require_worker {
                vec![worker]
            } else {
                Vec::new()
            },
            guard,
            end: "end".into(),
            source: tcl_lexer::SourceImage::document("array set state {}"),
            span: tcl_lexer::Span::new(0, 18),
            namespace: tcl_runtime_api::ByteNamespacePath::root(),
            namespace_context: None,
        });
    let mut done = tcl_bytecode::Instruction::new(tcl_bytecode::Op::DONE, Vec::new());
    done.offset = 2;
    asm.instructions = vec![first, done];
    asm.labels.insert("end".into(), 3);
    assert_eq!(asm.validate_native_compilation_entry(), Ok(()));
    let mut mismatched_guard = asm.clone();
    mismatched_guard.instructions[0].native_operation_selections[0].guard = match guard {
        tcl_runtime_api::CommandBindingGuard::ChunkEntry => {
            tcl_runtime_api::CommandBindingGuard::BeforeArguments
        }
        tcl_runtime_api::CommandBindingGuard::BeforeArguments => {
            tcl_runtime_api::CommandBindingGuard::ChunkEntry
        }
    };
    assert!(
        mismatched_guard
            .validate_native_compilation_entry()
            .is_err()
    );
    asm
}

#[test]
fn ensemble_operation_checks_configuration_and_independent_worker_binding() {
    use std::sync::Arc;
    use tcl_runtime_api::native_compilation::{
        NativeCommandCompilerPrerequisite, NativeCompilerSelectionPrerequisite,
    };
    use tcl_runtime_api::{CommandBindingGuard, CommandBindingIdentity};

    fn replacement(_vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        crate::interp::ok(Value::string("REPLAYED"))
    }

    for guard in [
        CommandBindingGuard::ChunkEntry,
        CommandBindingGuard::BeforeArguments,
    ] {
        for (replace_worker, require_worker, change_configuration, expected) in [
            (false, true, false, "VALUE"),
            (true, false, false, "VALUE"),
            (true, true, false, "REPLAYED"),
            (true, false, true, "REPLAYED"),
        ] {
            let mut vm = native_vm("tcl9.0");
            vm.set_compiler(Box::new(
                tcl_compiler::compile_service::BytecodeCompileService::for_profile(
                    vm.source_profile(),
                ),
            ));
            let binding = vm
                .native_compiler_binding_at_lookup(u64::from(ROOT_NS.0), &NameBytes::from("array"))
                .unwrap();
            let required = Arc::new(NativeCommandCompilerPrerequisite {
                interpreter: vm.native_interpreter_identity(),
                lookup_namespace_token: u64::from(ROOT_NS.0),
                invocation_word: "array".into(),
                slot: binding.slot,
                namespace_token: binding.namespace_token,
                token: binding.token,
                implementation_generation: binding.implementation_generation,
                compiler: binding.compiler.unwrap(),
                selected_worker: None,
                nested_compilers: Vec::new(),
                guard,
            });
            let typed = NativeCompilerSelectionPrerequisite::from_command_registration(Arc::clone(
                &required,
            ));
            assert!(matches!(
                typed,
                NativeCompilerSelectionPrerequisite::Ensemble(_)
            ));
            assert!(vm.native_compiler_selection_prerequisite_matches(&typed));
            let worker = CommandBindingIdentity::in_rooted_namespace(
                "::tcl::array",
                "set",
                "tcl::array::set",
            )
            .with_guard(guard);
            assert!(vm.command_binding_matches(&worker));
            if replace_worker {
                vm.register("::tcl::array::set", replacement);
                assert!(!vm.command_binding_matches(&worker));
                assert!(
                    vm.native_compiler_selection_prerequisite_matches(&typed),
                    "the worker replacement does not replace the public compiler"
                );
            }
            if change_configuration {
                let Command::Ensemble(token) = vm.commands.get("array").unwrap().clone() else {
                    panic!("actual array ensemble");
                };
                let mut configuration = token.config();
                configuration.prefixes = !configuration.prefixes;
                token.configure(configuration);
                assert!(!vm.native_compiler_selection_prerequisite_matches(&typed));
            }
            let asm = ensemble_operation_asm(required, worker, guard, require_worker);
            let unit = vm.compiled_unit(Rc::new(asm), vm.source_namespace_path());
            let completion = vm.run_compiled_unit(unit);
            assert_eq!(
                completion.code,
                Code::Ok,
                "{guard:?} worker={replace_worker} auxiliary={require_worker} configuration={change_configuration}"
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected.as_bytes(),
                "{guard:?} worker={replace_worker} auxiliary={require_worker} configuration={change_configuration}"
            );
        }
    }
}

const COMPILER_EPOCH_FIXTURES: [(&str, &str); 5] = [
    (
        "tcl8.4",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/8.4.20.tsv"),
    ),
    (
        "tcl8.5",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/8.5.19.tsv"),
    ),
    (
        "tcl8.6",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/8.6.18.tsv"),
    ),
    (
        "tcl9.0",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/9.0.4.tsv"),
    ),
    (
        "tcl9.1",
        include_str!("../../../../runtime/rust/tests/data/native_compiler_epochs/9.1.0.tsv"),
    ),
];

fn apply_compiler_epoch_event(vm: &mut Vm, namespace: &mut Option<NsId>, event: &str) -> Code {
    match event {
        "initial" => Code::Ok,
        "namespace_create" | "namespace_recreate" => {
            *namespace = Some(vm.activate_namespace_operand(b"N").unwrap());
            Code::Ok
        }
        "plain_create" => invoke(vm, "proc", &["p", "", "return P"]),
        "plain_rename" => invoke(vm, "rename", &["p", "q"]),
        "plain_delete" => invoke(vm, "rename", &["q", ""]),
        "compiled_shadow" => invoke(vm, "proc", &["N::set", "", "return LOCAL"]),
        "shadow_delete" => invoke(vm, "rename", &["N::set", ""]),
        "compiled_rename" => invoke(vm, "rename", &["set", "stamp_saved_set"]),
        "replacement_plain" => invoke(vm, "proc", &["set", "args", "return REPLACEMENT"]),
        "replacement_delete" => invoke(vm, "rename", &["set", ""]),
        "compiled_restore" => invoke(vm, "rename", &["stamp_saved_set", "set"]),
        "compiled_hide" => {
            vm.hide_command("set", "set").unwrap();
            Code::Ok
        }
        "compiled_expose" => {
            vm.expose_own_command("set", "set").unwrap();
            Code::Ok
        }
        "namespace_path" | "same_path" | "clear_path" => {
            let path = if event == "clear_path" {
                Vec::new()
            } else {
                vec![vm.activate_namespace_operand(b"M").unwrap()]
            };
            vm.push_ns_eval_token_frame(namespace.unwrap(), Vec::new());
            vm.ns_path_set(path);
            vm.pop_call_frame();
            vm.pop_ns();
            Code::Ok
        }
        "namespace_delete" => {
            assert!(vm.delete_namespace_operand(b"N").unwrap());
            *namespace = None;
            Code::Ok
        }
        _ => panic!("unhandled native event {event}"),
    }
}

#[test]
fn compiler_and_resolver_epochs_match_all_87_original_native_mutation_rows() {
    let engines = COMPILER_EPOCH_FIXTURES;
    let mut completed = 0;
    for (engine, reports) in engines {
        let mut vm = native_vm(engine);
        let baseline = vm.native_cache_stamp(ROOT_NS).interpreter_epoch.0;
        let native_baseline = native_epoch_baseline(reports);
        let mut namespace = None;
        for row in reports.lines().skip(1) {
            let fields = row.split('\t').collect::<Vec<_>>();
            let event = fields[0];
            let code = apply_compiler_epoch_event(&mut vm, &mut namespace, event);
            assert_eq!(
                code,
                Code::from_int(fields[1].parse().unwrap()),
                "{engine} {event}: completion"
            );
            let global = vm.native_cache_stamp(ROOT_NS);
            assert_eq!(
                global.interpreter_epoch.0 - baseline,
                fields[2].parse::<u64>().unwrap() - native_baseline,
                "{engine} {event}: compiler epoch"
            );
            assert_eq!(
                global.resolver_epoch,
                fields[3].parse::<u64>().unwrap(),
                "{engine} {event}: global resolver"
            );
            if let Some(namespace) = namespace {
                let stamp = vm.native_cache_stamp(namespace);
                assert_eq!(
                    stamp.resolver_epoch,
                    fields[4].parse::<u64>().unwrap(),
                    "{engine} {event}: namespace resolver"
                );
                assert_eq!(
                    vm.name_world
                        .borrow()
                        .command_reference_epochs
                        .get(&namespace)
                        .copied()
                        .unwrap_or(0),
                    fields[5].parse::<u64>().unwrap(),
                    "{engine} {event}: namespace command references"
                );
            } else {
                assert_eq!(fields[4], "-1", "{engine} {event}: absent namespace");
                assert_eq!(
                    fields[5], "-1",
                    "{engine} {event}: absent namespace command references"
                );
            }
            assert_eq!(
                raw_hook(&vm, "set"),
                fields[6].parse::<i32>().unwrap(),
                "{engine} {event}: raw set compiler"
            );
            completed += 1;
        }
    }
    assert_eq!(completed, 87);
}

#[test]
fn empty_literal_world_entry_is_original_capture_and_registration_scoped() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let vm = native_vm(engine);
        let first = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
        let receipt = first.empty_literal_world.as_ref().expect(engine);
        assert!(receipt.is_current_for(first.interpreter, first.epoch));
        assert!(!receipt.is_current_for(first.interpreter, first.epoch + 1));
        let second = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
        assert!(!receipt.is_current());
        let current = second.empty_literal_world.as_ref().unwrap();
        assert!(current.is_current_for(second.interpreter, second.epoch));
        assert!(!first.same_compilation_world(&second));

        let mut table = tcl_bytecode::LiteralTable::new();
        let index = table.intern_bytes(b"original ordinary registration");
        let pool = crate::literal_pool::NativeLiteralPool::create(
            &vm.native_literal_world,
            &table,
            vm.actual_native_invocation_dialect()
                .native_string_protocol(),
            ROOT_NS,
            &tcl_runtime_api::ByteNamespacePath::root(),
        )
        .unwrap();
        assert!(!current.is_current());
        let references = pool
            .with_original(index, Value::native_object_reference_count)
            .unwrap();
        let populated = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
        assert!(populated.empty_literal_world.is_none());
        assert_eq!(
            pool.with_original(index, Value::native_object_reference_count),
            Some(references)
        );
        drop(pool);
        let fresh = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
        let final_receipt = fresh.empty_literal_world.unwrap();
        assert!(final_receipt.is_current());
        drop(vm);
        assert!(!final_receipt.is_current());
    }
}

#[test]
fn compiler_pass_capture_uses_actual_parent_limits_and_state_lifetime() {
    let mut vm = native_vm("tcl8.6");
    let entry = vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false);
    let environment = entry.compiler_pass_environment.as_ref().unwrap();
    assert!(environment.is_current_for(entry.interpreter));
    assert!(environment.is_root());
    assert!(!environment.has_enabled_limits());
    vm.set_command_limit_value(Some(10_000));
    assert!(
        vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false)
            .compiler_pass_environment
            .unwrap()
            .has_enabled_limits()
    );
    vm.set_command_limit_value(None);
    vm.set_time_limit_deadline(Some(i128::from(i64::MAX)));
    assert!(
        vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false)
            .compiler_pass_environment
            .unwrap()
            .has_enabled_limits()
    );
    vm.set_time_limit_deadline(None);
    let name = vm.create_child(Some("child".into()), false);
    let id = vm.children[&name];
    let child_entry = vm.in_interp(id, |vm| {
        vm.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false)
    });
    assert!(!child_entry.compiler_pass_environment.unwrap().is_root());
    drop(vm);
    assert!(!environment.is_current_for(entry.interpreter));
    let jim = native_vm("jimtcl");
    assert!(
        jim.native_compilation_entry_for_namespace_token(Some(ROOT_NS), false)
            .compiler_pass_environment
            .is_none()
    );
}

#[test]
fn discarded_compiler_array_executes_cache_actions_then_releases_registrations() {
    use tcl_syntax::scalar_getter::NativeScalarCache;
    let mut vm = native_vm("tcl8.6");
    let mut resident = tcl_bytecode::FunctionAsm::default();
    resident.literals.intern("7");
    let owner = vm
        .create_native_literal_pool(&resident, &NamespacePath::root())
        .unwrap();
    let original = owner.value(0).unwrap();
    assert_eq!(original.native_scalar_cache(), None);
    let baseline = original.native_object_reference_count();
    let mut first = tcl_bytecode::LiteralTable::default();
    first.intern_expression_number(
        b"7",
        tcl_dialect::TclVersion::V8_6,
        tcl_bytecode::NativeExpressionNumberLiteral::Integer(7),
    );
    first.intern("discarded-only");
    let mut final_pass = tcl_bytecode::FunctionAsm::default();
    final_pass.literals.intern("7");
    final_pass.literals.retain_discarded_native_pass(first);
    let final_pool = vm
        .create_native_literal_pool(&final_pass, &NamespacePath::root())
        .unwrap();
    assert_eq!(
        original.native_scalar_cache(),
        Some(NativeScalarCache::Number(tcl_syntax::number::Number::Int(
            7
        )))
    );
    assert!(
        final_pool
            .with_original(0, |value| value.is_same_object(&original))
            .unwrap()
    );
    assert_eq!(original.native_object_reference_count(), baseline + 1);
    drop(final_pool);
    assert_eq!(original.native_object_reference_count(), baseline);
    drop(original);
    drop(owner);
    let interpreter = vm.native_interpreter_identity();
    assert!(
        vm.native_literal_world
            .borrow()
            .capture_empty_world(interpreter, vm.trace_deopt_epoch())
            .is_some()
    );
}

fn syntax_first_pass() -> tcl_bytecode::LiteralTable {
    use tcl_runtime_api::native_return_literal::{
        NativeKnownWordLiteral, NativeReturnOptionsLiteral,
    };
    let mut literals = tcl_bytecode::LiteralTable::new();
    let message = literals.register_unshared(b"missing close-brace");
    let options = literals.register_private_return_options(NativeReturnOptionsLiteral {
        protocol: tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V9_1),
        words: [b"-errorcode".as_slice(), b"NONE"]
            .into_iter()
            .map(|word| NativeKnownWordLiteral {
                pieces: vec![word.to_vec()],
                composite: false,
            })
            .collect(),
        code: 0,
        level: 1,
        size: 1,
    });
    literals.retain_syntax_error_info(options, message);
    literals
}

#[test]
fn first_array_callback_revalidates_replay_before_final_array_publication() {
    fn enable_limit(vm: &mut Vm, _arguments: &[Value]) -> Completion<Value> {
        vm.set_command_limit_value(Some(1_000_000));
        crate::interp::ok(Value::string(""))
    }
    for changes_limit in [false, true] {
        let mut vm = native_vm("tcl9.1");
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(vm.source_profile()),
        ));
        if changes_limit {
            vm.register("enableReplayLimit", enable_limit);
            assert_eq!(
                invoke(
                    &mut vm,
                    "trace",
                    &[
                        "add",
                        "variable",
                        "::errorInfo",
                        "write",
                        "enableReplayLimit"
                    ],
                ),
                Code::Ok
            );
        }
        let mut resident = tcl_bytecode::FunctionAsm::default();
        resident.literals.intern("final-array-only");
        let resident_pool = vm
            .create_native_literal_pool(&resident, &NamespacePath::root())
            .unwrap();
        let original = resident_pool.value(0).unwrap();
        let baseline = original.native_object_reference_count();
        let environment = vm.capture_native_compiler_pass_environment(None).unwrap();
        let mut asm = tcl_bytecode::FunctionAsm::default();
        asm.literals.intern("final-array-only");
        asm.literals
            .retain_discarded_native_pass(syntax_first_pass());
        asm.literals.retain_compiler_replay_environment(environment);
        let result = vm.create_native_literal_pool(&asm, &NamespacePath::root());
        if changes_limit {
            let error = result
                .err()
                .expect("changed reached environment refuses replay");
            assert_eq!(
                error.to_string(),
                "native compiler replay environment changed during first-pass publication"
            );
            assert!(vm.limits.cmd_value.is_some());
            assert_eq!(original.native_object_reference_count(), baseline);
        } else {
            let pool = result.unwrap();
            assert_eq!(original.native_object_reference_count(), baseline + 1);
            assert_eq!(
                pool.value(0)
                    .unwrap()
                    .resident_string_bytes()
                    .unwrap()
                    .as_ref(),
                b"final-array-only"
            );
        }
    }
}

#[test]
fn planned_replay_requires_same_live_environment_owner() {
    let mut vm = native_vm("tcl9.1");
    let foreign = native_vm("tcl9.1");
    let environment = foreign
        .capture_native_compiler_pass_environment(None)
        .unwrap();
    let mut asm = tcl_bytecode::FunctionAsm::default();
    asm.literals.intern("final-array-only");
    asm.literals
        .retain_discarded_native_pass(tcl_bytecode::LiteralTable::default());
    asm.literals.retain_compiler_replay_environment(environment);
    assert!(
        vm.create_native_literal_pool(&asm, &NamespacePath::root())
            .is_err()
    );
    drop(foreign);
    assert!(
        vm.create_native_literal_pool(&asm, &NamespacePath::root())
            .is_err()
    );
    vm.set_command_limit_value(Some(100));
    asm.literals.retain_compiler_replay_environment(
        vm.capture_native_compiler_pass_environment(None).unwrap(),
    );
    assert!(
        vm.create_native_literal_pool(&asm, &NamespacePath::root())
            .is_err()
    );
}
