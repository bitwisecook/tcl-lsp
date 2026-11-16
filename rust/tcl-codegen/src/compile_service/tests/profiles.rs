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

//! Profile-backed services: shared registries, pack overlays, fresh-interpreter modules and the world a module states.

use super::*;

#[test]
fn profile_backed_service_follows_a_new_profiles_shared_registry() {
    let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let tcl86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let service = BytecodeCompileService::for_profile(tcl86);
    let module = service
        .compile_for_profile("foreachLine line file.txt {}", tcl90)
        .unwrap();

    assert!(std::ptr::eq(module.profile, tcl90));
    assert_eq!(
        module.top_level.command_bindings,
        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
    );
    assert!(module.top_level.instructions.iter().any(|instruction| {
        matches!(
            instruction.op,
            tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
        ) && instruction.source_cmd_text == "foreachLine line file.txt {}"
    }));
}

/// A service over a pack overlay is built only if the packs are
/// installed; nothing builds one over a generation that is not there.
#[test]
fn a_service_for_an_uninstalled_overlay_is_not_built() {
    const OVERLAY: u64 = 0x0C5E_0001;
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("catalogue profile");
    assert_eq!(
        BytecodeCompileService::for_profile_with_overlay(profile, OVERLAY)
            .err()
            .expect("nothing installed the overlay"),
        OverlayMiss {
            environment: "tcl9.0".to_owned(),
            overlay: OVERLAY,
        }
    );
    assert!(
        BytecodeCompileService::for_profile_with_overlay(profile, 0).is_ok(),
        "no overlay is the plain profile service"
    );
}

/// A command only the overlay declares is a known command to a service
/// built over it, whether it compiles for its own profile or is asked for
/// another release's, and an unknown one to the plain service: the packs
/// reach the compile through the very generation they were installed as.
#[test]
fn a_service_over_an_installed_overlay_compiles_its_commands() {
    const OVERLAY: u64 = 0x0C5E_0002;
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("catalogue profile");
    let installed = tcl_registry::registry_for_profile_with_overlay(profile, OVERLAY, |r| {
        let mut custom = r.get("llength").expect("llength spec").clone();
        custom.name = "overlay::length";
        custom.surface = None;
        r.insert(custom);
    });
    let known = |module: &tcl_bytecode::ModuleAsm| {
        module
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "overlay::length")
    };
    let source = "overlay::length {a b c}";

    let service = BytecodeCompileService::for_profile_with_overlay(profile, OVERLAY)
        .expect("installed, so the service is built");
    assert!(std::ptr::eq(
        service.registry.registry().expect("resolves").as_ref(),
        Arc::as_ptr(&installed)
    ));
    assert!(known(&service.compile(source).unwrap()));
    assert!(known(
        &service.compile_for_profile(source, profile).unwrap()
    ));

    let plain = BytecodeCompileService::for_profile(profile);
    assert!(
        !known(&plain.compile(source).unwrap()),
        "the plain service has never heard of it"
    );
}

/// The overlay is looked up at every compile, so a service built while
/// its packs were installed declines each compile entry once the
/// generation is gone, and names the overlay: it does not compile against
/// the plain registry under the packs' name.
#[test]
fn a_service_whose_overlay_is_gone_declines_every_compile() {
    const OVERLAY: u64 = 0x0C5E_0003;
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("catalogue profile");
    let service = BytecodeCompileService {
        registry: RegistryTarget::Overlay {
            profile,
            overlay: OVERLAY,
        },
        config: LexerConfig::from_grammar(profile.grammar),
        profile: Some(profile),
    };
    let source = "set x 1";
    let script = ScriptCompileTarget {
        source,
        namespace: "",
        evaluation: CommandEvaluation::Compiled,
    };
    let parameters = Vec::new();
    let procedure = ProcedureCompileTarget {
        source,
        parameters: &parameters,
        namespace: "",
    };
    let declined = |what: &str, result: Result<tcl_bytecode::ModuleAsm, CompileError>| {
        let error = result.err().unwrap_or_else(|| panic!("{what} compiled"));
        let CompileError::Unsupported(message) = error else {
            panic!("{what}: a missing overlay must be an operational refusal: {error:?}");
        };
        assert!(
            message.contains("0xc5e0003") && message.contains("tcl9.0"),
            "{what}: {message}"
        );
    };
    declined("compile", service.compile(source));
    declined("compile_traced", service.compile_traced(source));
    declined(
        "compile_for_profile",
        service.compile_for_profile(source, profile),
    );
    declined(
        "compile_traced_for_profile",
        service.compile_traced_for_profile(source, profile),
    );
    declined(
        "compile_script_for_profile",
        service.compile_script_for_profile(script, profile),
    );
    declined(
        "compile_plain_script_for_profile",
        service.compile_plain_script_for_profile(script, profile),
    );
    for dispatch in [ProcedureDispatch::Optimised, ProcedureDispatch::Plain] {
        declined(
            "compile_procedure_for_profile",
            service.compile_procedure_for_profile(procedure, profile, dispatch),
        );
    }
}

/// A module states the world its profile resolves to — environment,
/// release, build, package floors — beside this build's ABI, intrinsic
/// table and embedded library, whichever entry compiled it.
#[test]
fn a_module_states_the_world_it_was_compiled_for() {
    for (name, release) in [
        ("tcl8.4", "8.4"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("f5-irules", "tmm"),
    ] {
        let profile = tcl_dialect::DialectProfile::find(name).expect("catalogue profile");
        let service = BytecodeCompileService::for_profile(profile);
        let expected = tcl_registry::model::runtime_context_for_profile(profile)
            .identity(&[], tcl_registry::intrinsic_table_hash());
        assert_eq!(expected.environment, name);
        assert_eq!(expected.release, release);
        assert_eq!(
            expected.abi_version,
            tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION
        );
        assert_eq!(
            expected.embedded_stdlib_revision,
            tcl_runtime_api::manifest::EMBEDDED_STDLIB_REVISION
        );
        let parameters = Vec::new();
        let script = ScriptCompileTarget {
            source: "set x 1",
            namespace: "",
            evaluation: CommandEvaluation::Compiled,
        };
        let procedure = ProcedureCompileTarget {
            source: "set x 1",
            parameters: &parameters,
            namespace: "",
        };
        for (what, module) in [
            ("compile", service.compile("set x 1")),
            (
                "compile_for_profile",
                service.compile_for_profile("set x 1", profile),
            ),
            ("compile_traced", service.compile_traced("set x 1")),
            (
                "compile_plain_script_for_profile",
                service.compile_plain_script_for_profile(script, profile),
            ),
            (
                "compile_procedure_for_profile",
                service.compile_procedure_for_profile(
                    procedure,
                    profile,
                    ProcedureDispatch::Optimised,
                ),
            ),
        ] {
            let module = module.unwrap_or_else(|error| panic!("{name} {what}: {error}"));
            assert_eq!(module.manifest.as_deref(), Some(&expected), "{name} {what}");
        }
    }
}

/// A profile-less service compiles a plain script for the fallback
/// profile's fresh interpreter, which the VM runs such a module as: an
/// admitted entry, with `set` compiled by its release's compiler.
#[test]
fn a_profile_less_whole_module_compiles_for_a_fresh_interpreter() {
    for source in ["set x 1", "set x 1\nlist $x", "proc p {} {return 1}\np"] {
        for plain in [false, true] {
            let service = BytecodeCompileService::default();
            let module = if plain {
                service.compile_traced(source)
            } else {
                service.compile(source)
            }
            .unwrap_or_else(|error| panic!("{source:?} plain={plain}: {error:?}"));
            assert_eq!(
                module.top_level.native_compilation_preflight,
                tcl_runtime_api::NativeCompilationPreflight::NotRequired,
                "{source:?} plain={plain}"
            );
            assert!(module.top_level.validate_native_compilation_entry().is_ok());
            assert!(std::ptr::eq(module.profile, DialectProfile::plain_tcl()));
        }
    }
    let module = BytecodeCompileService::default()
        .compile("set x 1")
        .unwrap();
    assert!(
        module
            .top_level
            .instructions
            .iter()
            .all(|instruction| !matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            )),
        "a fresh interpreter's `set` is compiled: {:?}",
        module.top_level.instructions
    );
}

/// A whole document compiled for a fresh interpreter of its profile has
/// the procedures it defines, each with its provenance, and the commands
/// the profile compiles specialised and bound, as `tclsh file.tcl`
/// compiles the file as one script object.
#[test]
fn a_fresh_whole_module_defines_its_procedures_and_specialises_its_commands() {
    let source = "set x 1\nlassign {1 2} a b\nproc caller {n} {set y [llength $n]; return $y}\nlist $a $b [caller $x]\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).expect("catalogue profile");
        let module = BytecodeCompileService::for_profile(profile)
            .compile_for_profile(source, profile)
            .unwrap_or_else(|error| panic!("{dialect}: {error:?}"));
        assert_eq!(
            module.top_level.native_compilation_preflight,
            tcl_runtime_api::NativeCompilationPreflight::NotRequired,
            "{dialect}"
        );
        let caller = fixture_id(&module.procedures, "::caller");
        assert_eq!(module.procedure_provenance[&caller].name, "::caller");
        for command in ["set", "lassign"] {
            assert!(
                module
                    .top_level
                    .command_bindings
                    .iter()
                    .any(|binding| binding.name == command && binding.identity == command),
                "{dialect}: {command} is bound: {:?}",
                module.top_level.command_bindings
            );
        }
        assert!(
            !module
                .top_level
                .instructions
                .iter()
                .any(|instruction| matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                ) && (instruction.source_cmd_text.bytes().starts_with(b"set ")
                    || instruction.source_cmd_text.bytes().starts_with(b"lassign "))),
            "{dialect}: {:?}",
            module.top_level.instructions
        );
    }
}

/// The raw lowering pipeline with no entry is the same fresh
/// interpreter, so its module is admitted; an explicitly unknown entry
/// keeps the chunk's compiler provider unresolved.
#[test]
fn an_entry_less_pipeline_is_admitted_and_an_unknown_entry_is_not() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").expect("catalogue profile");
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let config = LexerConfig::default();
    let source = "set l {1 2 3}\nlassign $l a b\nlist $a $b\n";
    let ir = tcl_compiler::lowering::lower_script_module_for_bytecode(
        source, "", registry, config, None, false,
    );
    let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry_and_config(
        &ir, false, registry, config,
    );
    let module = codegen_module(&cfg, &ir, registry).expect("original source namespace");
    assert_eq!(
        module.top_level.native_compilation_preflight,
        tcl_runtime_api::NativeCompilationPreflight::NotRequired
    );
    assert!(
        module
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.identity == "lassign"),
        "{:?}",
        module.top_level.command_bindings
    );

    let config = LexerConfig::from_grammar(profile.grammar);
    let input = fresh_document_input(registry, config, profile).expect("fresh input");
    let entry =
        tcl_compiler::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input);
    let mut options = entry.options();
    options.native_compilation.mode =
        tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
    let ir = lower_script_module_for_bytecode_with_options(
        source,
        "",
        registry,
        config,
        Some(profile),
        false,
        Some(options),
    );
    assert!(
        ir.top_level
            .native_compilation_admission
            .as_ref()
            .is_some_and(|admission| admission.provider_required),
        "an unknown entry's compiler table is not the profile's"
    );
}
