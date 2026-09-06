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

//! Compiler-backed implementation of the runtime compilation seam.
//!
//! A VM host must compile both optimised bytecode and bytecode whose command
//! invocations remain ordinary runtime dispatches. Keeping those paths here
//! makes the lowering mode, parse grammar, registry, expression dialect, and
//! bytecode profile one indivisible target selection.

#[cfg(test)]
use crate::cfg_builder::build_cfg_codegen_with_registry;
use crate::cfg_builder::{build_cfg_codegen_with_registry_and_context, prepare_cfg_context_bundle};
#[cfg(test)]
use crate::codegen::codegen_module;
use crate::codegen::codegen_module_with_command_mutations;
use crate::codegen::emitter::codegen_procedure_module_with_command_mutations;
use crate::lowering::{
    first_fatal_parse_error_with_config, lower_proc_body_module_for_bytecode,
    lower_script_module_for_bytecode,
};
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use tcl_dialect::DialectProfile;
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::{
    CompileError, CompileService, ProcedureCompileTarget, ProcedureDispatch, ScriptCommandPlan,
    ScriptCompileTarget,
};

enum RegistryTarget {
    Owned {
        registry: CommandRegistry,
        profile_views: Mutex<FxHashMap<&'static str, Arc<CommandRegistry>>>,
    },
    Profile(&'static CommandRegistry),
}

enum ProfileRegistry<'a> {
    Borrowed(&'a CommandRegistry),
    Cached(Arc<CommandRegistry>),
}

impl AsRef<CommandRegistry> for ProfileRegistry<'_> {
    fn as_ref(&self) -> &CommandRegistry {
        match self {
            Self::Borrowed(registry) => registry,
            Self::Cached(registry) => registry,
        }
    }
}

impl RegistryTarget {
    fn registry(&self) -> &CommandRegistry {
        match self {
            Self::Owned { registry, .. } => registry,
            Self::Profile(registry) => registry,
        }
    }

    /// Select the registry axis for an explicit profile compile.
    ///
    /// An owned registry is an embedder's semantic command surface (including
    /// dynamically installed `SpecTcl` hooks), so changing the target profile
    /// must not replace it. A profile-backed service has no such override and
    /// follows the newly requested profile's shared registry generation.
    fn registry_for_profile(&self, profile: &'static DialectProfile) -> ProfileRegistry<'_> {
        match self {
            Self::Owned {
                registry,
                profile_views,
            } => {
                let mut views = profile_views.lock().expect("profile registry view mutex");
                let view = views
                    .entry(profile.name)
                    .or_insert_with(|| Arc::new(registry.project_for_profile(profile)));
                ProfileRegistry::Cached(Arc::clone(view))
            }
            Self::Profile(_) => ProfileRegistry::Borrowed(
                tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
            ),
        }
    }
}

/// A complete compiler-backed [`CompileService`] for the Tcl bytecode VM.
///
/// Construct profile-less/default-registry consumers with [`Self::new`].
/// Release- or dialect-aware consumers use [`Self::for_profile`], which obtains
/// the registry and lexer grammar through the resolved-profile ingress seam.
/// Both forms support optimised and plain-dispatch compilation.
pub struct BytecodeCompileService {
    registry: RegistryTarget,
    config: LexerConfig,
    profile: Option<&'static DialectProfile>,
}

impl BytecodeCompileService {
    /// Build a service for a profile-less registry and the default Tcl grammar.
    #[must_use]
    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            registry: RegistryTarget::Owned {
                registry,
                profile_views: Mutex::new(FxHashMap::default()),
            },
            config: LexerConfig::default(),
            profile: None,
        }
    }

    /// Build a service for one resolved dialect profile.
    #[must_use]
    pub fn for_profile(profile: &'static DialectProfile) -> Self {
        Self {
            registry: RegistryTarget::Profile(
                tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
            ),
            config: LexerConfig::from_grammar(profile.grammar),
            profile: Some(profile),
        }
    }

    fn compile_target(
        &self,
        source: &str,
        plain_command_dispatch: bool,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        Self::compile_target_with(
            source,
            "",
            plain_command_dispatch,
            self.registry.registry(),
            self.config,
            self.profile,
        )
    }

    fn compile_target_for_profile(
        &self,
        source: &str,
        plain_command_dispatch: bool,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_target_with(
            source,
            "",
            plain_command_dispatch,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
        )
    }

    fn compile_target_with(
        source: &str,
        namespace: &str,
        plain_command_dispatch: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: Option<&'static DialectProfile>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(message) = first_fatal_parse_error_with_config(source, config) {
            return Err(CompileError(message));
        }
        let ir = lower_script_module_for_bytecode(
            source,
            namespace,
            registry,
            config,
            profile,
            plain_command_dispatch,
        );
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let command_mutations = crate::command_binding::scan_module_command_mutations_with_bindings(
            &ir,
            registry,
            prepared.command_bindings(),
        );
        Ok(codegen_module_with_command_mutations(
            &cfg,
            &ir,
            registry,
            &command_mutations,
        ))
    }

    fn compile_procedure_target_with(
        target: ProcedureCompileTarget<'_>,
        plain_command_dispatch: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(message) = first_fatal_parse_error_with_config(target.source, config) {
            return Err(CompileError(message));
        }
        let ir = lower_proc_body_module_for_bytecode(
            target.source,
            target.namespace,
            registry,
            config,
            Some(profile),
            plain_command_dispatch,
        );
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let command_mutations = crate::command_binding::scan_module_command_mutations_with_bindings(
            &ir,
            registry,
            prepared.command_bindings(),
        );
        let params: Vec<&str> = target.parameters.iter().map(String::as_str).collect();
        Ok(codegen_procedure_module_with_command_mutations(
            &cfg,
            &ir,
            &params,
            registry,
            &command_mutations,
        ))
    }
}

impl Default for BytecodeCompileService {
    fn default() -> Self {
        Self::new(CommandRegistry::build_default())
    }
}

impl CompileService for BytecodeCompileService {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, source: &str) -> Result<Self::Module, CompileError> {
        self.compile_target(source, false)
    }

    fn compile_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile_target_for_profile(source, false, profile)
    }

    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_target_with(
            target.source,
            target.namespace,
            false,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
        )
    }

    fn compile_traced(&self, source: &str) -> Result<Self::Module, CompileError> {
        self.compile_target(source, true)
    }

    fn compile_traced_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile_target_for_profile(source, true, profile)
    }

    fn compile_plain_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_target_with(
            target.source,
            target.namespace,
            true,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
        )
    }

    fn script_command_plan_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
    ) -> ScriptCommandPlan {
        let segmented = crate::lowering::command_at_time_script_with_config(
            source,
            LexerConfig::from_grammar(profile.grammar),
        );
        match segmented.fatal_tail {
            Some((start, message)) => ScriptCommandPlan {
                complete_prefix_len: start,
                fatal_tail: Some(CompileError(message)),
            },
            None => ScriptCommandPlan::complete(source.len()),
        }
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_procedure_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir_for_bytecode_with_dialect;

    fn registry_with_custom_list_expr_hook() -> CommandRegistry {
        let mut registry = CommandRegistry::build_default();
        let mut custom = registry.get("list").expect("list spec").clone();
        // SpecTcl overrides are commonly profile-independent authored rows.
        // They must still replace the core row after an explicit-profile
        // compiler projection.
        custom.surface = None;
        custom.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
        custom.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
        registry.insert(custom);
        registry
    }

    #[test]
    fn service_marks_plain_dispatch_and_preserves_profile() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let fast = service.compile("expr {1 + 2}").unwrap();
        let plain = service.compile_traced("expr {1 + 2}").unwrap();

        assert!(std::ptr::eq(fast.profile, profile));
        assert!(std::ptr::eq(plain.profile, profile));
        assert!(!fast.plain_command_dispatch);
        assert!(plain.plain_command_dispatch);
        assert!(plain.top_level.plain_command_dispatch);
        assert!(plain.top_level.command_bindings.is_empty());
    }

    #[test]
    fn plain_dispatch_does_not_append_builtin_error_semantics() {
        let service = BytecodeCompileService::default();
        let plain = service.compile_traced("error \"boom\"").unwrap();
        let ops: Vec<_> = plain
            .top_level
            .instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect();
        assert_eq!(
            ops.iter()
                .filter(|op| matches!(
                    op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                ))
                .count(),
            1,
            "plain compilation must contain one runtime dispatch: {ops:?}"
        );
        assert!(
            !ops.contains(&tcl_bytecode::Op::RETURN_IMM),
            "plain compilation must not append the registry builtin's completion: {ops:?}"
        );
    }

    #[test]
    fn owned_registry_tcl84_projection_keeps_user_throw_fallthrough() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
        let service = BytecodeCompileService::new(CommandRegistry::build_default());
        let module = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    // Tcl 8.4 has no builtin `throw`, so both spellings can be
                    // user procedures and neither may terminate CFG lowering.
                    source: "throw; after_throw",
                    parameters: &[],
                    namespace: "",
                },
                profile,
                ProcedureDispatch::Optimised,
            )
            .unwrap();
        let invoke_count = module
            .top_level
            .instructions
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                )
            })
            .count();
        assert_eq!(
            invoke_count, 2,
            "the call after Tcl 8.4's user-bindable `throw` must remain reachable: {:?}",
            module.top_level.instructions
        );
    }

    #[test]
    fn plain_procedure_dispatch_does_not_specialise_internal_dict_loops() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        for command in ["::tcl::dict::for", "::tcl::dict::map"] {
            let source = format!("{command} {{k v}} {{a 1}} {{set seen $k}}");
            let module = service
                .compile_procedure_for_profile(
                    ProcedureCompileTarget {
                        source: &source,
                        parameters: &[],
                        namespace: "",
                    },
                    profile,
                    ProcedureDispatch::Plain,
                )
                .unwrap();
            let ops: Vec<_> = module
                .top_level
                .instructions
                .iter()
                .map(|instruction| instruction.op)
                .collect();
            assert!(
                ops.iter().any(|op| matches!(
                    op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                )),
                "{command} must retain runtime command dispatch: {ops:?}"
            );
            assert!(
                !ops.contains(&tcl_bytecode::Op::DICT_FIRST)
                    && !ops.contains(&tcl_bytecode::Op::DICT_NEXT),
                "{command} must not use builtin dict-loop opcodes: {ops:?}"
            );
        }
    }

    #[test]
    fn procedure_target_seeds_params_and_supports_both_dispatch_modes() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let parameters = vec!["value".to_owned(), "suffix".to_owned()];
        let target = ProcedureCompileTarget {
            source: "append value $suffix; return $value",
            parameters: &parameters,
            namespace: "example",
        };
        let fast = service
            .compile_procedure_for_profile(target, profile, ProcedureDispatch::Optimised)
            .unwrap();
        let plain = service
            .compile_procedure_for_profile(target, profile, ProcedureDispatch::Plain)
            .unwrap();

        assert_eq!(&fast.top_level.lvt.entries()[..2], ["value", "suffix"]);
        assert!(std::ptr::eq(fast.profile, profile));
        assert!(!fast.top_level.plain_command_dispatch);
        assert!(
            fast.top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == "append" && binding.identity == "append")
        );
        assert!(plain.top_level.plain_command_dispatch);
        assert!(plain.top_level.command_bindings.is_empty());

        let static_proc = service
            .compile("proc p {} {mutate; if {1} {return yes}}")
            .unwrap();
        assert_eq!(
            static_proc.procedure_provenance["::p"],
            tcl_bytecode::ProcedureProvenance {
                name: "::p".to_owned(),
                parameters: String::new(),
                body: "mutate; if {1} {return yes}".to_owned(),
            }
        );
    }

    #[test]
    fn procedure_target_matches_static_proc_across_profiles_and_dispatches() {
        // Keep the cases together: each exercises module state that a
        // procedure-target lowering used to assemble separately from a static
        // `proc` body (procedure frame/LVT, namespace resolution, nested and
        // const-materialised procedures, command aliases and rename, and
        // namespace directives).
        let body = "namespace import ::source::*\n\
                    namespace export exposed\n\
                    interp alias {} mirror {} append\n\
                    mirror value $suffix\n\
                    rename mirror appended\n\
                    appended value !\n\
                    set child_body {return child}\n\
                    proc child {} $child_body\n\
                    return $value";
        let parameters = vec!["value".to_owned(), "suffix".to_owned()];

        for environment in ["tcl8.4", "tcl9.0"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
            let service = BytecodeCompileService::for_profile(profile);
            let static_source = format!("proc ::matrix::p {{value suffix}} {{{body}}}");
            let static_module = service
                .compile_for_profile(&static_source, profile)
                .unwrap();
            let static_ir = lower_to_ir_for_bytecode_with_dialect(
                &static_source,
                service.registry.registry_for_profile(profile).as_ref(),
                LexerConfig::from_grammar(profile.grammar),
                Some(profile),
            );

            for dispatch in [ProcedureDispatch::Optimised, ProcedureDispatch::Plain] {
                let direct_ir = lower_proc_body_module_for_bytecode(
                    body,
                    "matrix",
                    service.registry.registry_for_profile(profile).as_ref(),
                    LexerConfig::from_grammar(profile.grammar),
                    Some(profile),
                    dispatch == ProcedureDispatch::Plain,
                );
                let direct_module = service
                    .compile_procedure_for_profile(
                        ProcedureCompileTarget {
                            source: body,
                            parameters: &parameters,
                            namespace: "matrix",
                        },
                        profile,
                        dispatch,
                    )
                    .unwrap();
                let static_proc = &static_module.procedures["::matrix::p"];
                let direct_proc = &direct_module.top_level;

                if dispatch == ProcedureDispatch::Optimised {
                    assert_eq!(
                        static_proc.lvt.entries(),
                        direct_proc.lvt.entries(),
                        "{environment:?} procedure parameters/LVT"
                    );
                    assert_eq!(
                        static_proc.literals.entries(),
                        direct_proc.literals.entries(),
                        "{environment:?} literal materialisation"
                    );
                    let mut static_instructions = static_proc.instructions.clone();
                    let mut direct_instructions = direct_proc.instructions.clone();
                    // Static proc spans refer to the enclosing module while a
                    // direct target starts at offset zero; every codegen fact
                    // other than that coordinate system must agree.
                    for instruction in static_instructions
                        .iter_mut()
                        .chain(direct_instructions.iter_mut())
                    {
                        instruction.source_span = None;
                    }
                    assert_eq!(
                        static_instructions, direct_instructions,
                        "{environment:?} procedure instruction shape"
                    );
                    assert_eq!(
                        static_proc.labels, direct_proc.labels,
                        "{environment:?} procedure labels"
                    );
                    assert_eq!(
                        static_proc.loop_targets, direct_proc.loop_targets,
                        "{environment:?} loop targets"
                    );
                    assert_eq!(
                        static_proc.command_bindings, direct_proc.command_bindings,
                        "{environment:?} command bindings"
                    );
                    assert!(
                        direct_module.procedures.contains_key("::matrix::child"),
                        "{environment:?} lost materialised nested proc"
                    );
                } else {
                    assert!(direct_proc.plain_command_dispatch);
                    assert!(direct_proc.command_bindings.is_empty());
                }
                assert_eq!(
                    static_ir.namespace_imports, direct_ir.namespace_imports,
                    "{environment:?} {dispatch:?} namespace imports"
                );
                assert_eq!(
                    static_ir.namespace_exports, direct_ir.namespace_exports,
                    "{environment:?} {dispatch:?} namespace exports"
                );
            }
        }
    }

    #[test]
    fn procedure_target_roots_literal_colon_constructed_namespace_once() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let registry = service.registry.registry_for_profile(profile);
        let config = LexerConfig::from_grammar(profile.grammar);
        let body = "namespace export exposed\n\
                    proc child {} {return [namespace current]}\n\
                    return [namespace current]";
        let direct_ir = lower_proc_body_module_for_bytecode(
            body,
            ":",
            registry.as_ref(),
            config,
            Some(profile),
            false,
        );
        let direct_module = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    source: body,
                    parameters: &[],
                    namespace: ":",
                },
                profile,
                ProcedureDispatch::Optimised,
            )
            .expect("literal-colon procedure target compiles");

        assert_eq!(
            direct_ir.namespace_exports,
            [(":::".to_owned(), "exposed".to_owned())]
        );
        assert!(direct_ir.procedures.contains_key(":::::child"));
        assert!(direct_module.procedures.contains_key(":::::child"));

        let static_source = "namespace eval : {\
            proc p {} {\
                namespace export exposed\n\
                proc child {} {return [namespace current]}\n\
                return [namespace current]\
            }\
        }";
        let static_ir = lower_to_ir_for_bytecode_with_dialect(
            static_source,
            registry.as_ref(),
            config,
            Some(profile),
        );
        assert_eq!(static_ir.namespace_exports, direct_ir.namespace_exports);
        assert!(static_ir.procedures.contains_key(":::::p"));
    }

    #[test]
    fn tcl84_profile_projection_preserves_owned_hooks_in_fast_and_plain_modes() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
        let service = BytecodeCompileService::new(registry_with_custom_list_expr_hook());
        let fast = service
            .compile_for_profile("list {1 + 2}", profile)
            .unwrap();
        let plain = service
            .compile_traced_for_profile("list {1 + 2}", profile)
            .unwrap();

        assert!(std::ptr::eq(fast.profile, profile));
        assert!(std::ptr::eq(plain.profile, profile));
        assert!(
            fast.top_level
                .command_bindings
                .iter()
                .any(|binding| { binding.name == "list" && binding.identity == "list" })
        );
        assert!(
            fast.top_level
                .instructions
                .iter()
                .all(|instruction| instruction.op != tcl_bytecode::Op::INVOKE_STK1),
            "the owned Expr hook must lower the custom list spec: {:?}",
            fast.top_level.instructions,
        );
        assert!(plain.plain_command_dispatch);
        assert!(plain.top_level.plain_command_dispatch);
        assert!(plain.top_level.command_bindings.is_empty());
        assert!(
            plain
                .top_level
                .instructions
                .iter()
                .any(|instruction| instruction.op == tcl_bytecode::Op::INVOKE_STK1)
        );
    }

    #[test]
    fn lowering_provenance_is_exact_and_escaped_heads_stay_generic() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let direct = service.compile_for_profile("set x OLD", profile).unwrap();
        assert!(
            direct
                .top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == "set" && binding.identity == "set"),
            "a consumed lowering hook must retain its exact registry binding: {:?}",
            direct.top_level.command_bindings,
        );

        let escaped = service
            .compile_for_profile(r"se\x74 x OLD", profile)
            .unwrap();
        assert!(
            escaped.top_level.command_bindings.is_empty(),
            "lowering must not forge a binding for a head it did not resolve: {:?}",
            escaped.top_level.command_bindings,
        );
        assert!(
            escaped
                .top_level
                .instructions
                .iter()
                .any(|instruction| matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                )),
            "the escaped head must remain on live runtime dispatch: {:?}",
            escaped.top_level.instructions,
        );
    }

    #[test]
    fn fallback_lowering_shape_does_not_claim_a_binding() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let specialised = service
            .compile_for_profile("return value", profile)
            .unwrap();
        assert!(
            specialised
                .top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == "return" && binding.identity == "return")
        );

        let fallback = service
            .compile_for_profile("return -code ok value", profile)
            .unwrap();
        assert!(
            !fallback
                .top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == "return"),
            "a runtime-dispatched Barrier must not claim typed-lowering provenance: {:?}",
            fallback.top_level.command_bindings,
        );
        assert!(
            fallback
                .top_level
                .instructions
                .iter()
                .any(|instruction| matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                ))
        );

        // `llength` reaches bytecode as a Call, then its bytecode hook
        // specialises that exact shape. The later consumer must retain the
        // binding itself rather than inheriting one from lowering.
        let codegen_specialised = service
            .compile_for_profile("llength {a b}", profile)
            .unwrap();
        assert!(
            codegen_specialised
                .top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == "llength" && binding.identity == "llength"),
            "a codegen specialisation must retain its own exact binding: {:?}",
            codegen_specialised.top_level.command_bindings,
        );
    }

    #[test]
    fn profile_backed_service_follows_a_new_profiles_shared_registry() {
        let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let tcl86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let service = BytecodeCompileService::for_profile(tcl86);
        let module = service
            .compile_for_profile("foreachLine line file.txt {}", tcl90)
            .unwrap();

        assert!(std::ptr::eq(module.profile, tcl90));
        assert!(
            module.top_level.command_bindings.iter().any(|binding| {
                binding.name == "foreachLine" && binding.identity == "foreachLine"
            })
        );
    }

    #[test]
    fn runtime_script_command_plan_uses_the_requested_release_grammar() {
        let tcl84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
        let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(tcl90);
        let source = "set side 1; {*}{set x 2}";

        let old = service.script_command_plan_for_profile(source, tcl84);
        assert_eq!(&source[..old.complete_prefix_len], "set side 1; ");
        assert_eq!(
            old.fatal_tail.expect("8.4 rejects expansion syntax").0,
            "extra characters after close-brace"
        );

        let current = service.script_command_plan_for_profile(source, tcl90);
        assert_eq!(current.complete_prefix_len, source.len());
        assert!(current.fatal_tail.is_none());
    }

    #[test]
    fn catch_keeps_its_binding_and_exact_replay_boundary_after_cfg_lowering() {
        let source = "proc p {} {mutate; catch {error x} msg; return $msg}";
        let catch_source = "catch {error x} msg";
        let module = BytecodeCompileService::default().compile(source).unwrap();
        let procedure = &module.procedures["::p"];

        assert!(
            procedure
                .command_bindings
                .iter()
                .any(|binding| { binding.name == "catch" && binding.identity == "catch" })
        );
        assert!(procedure.instructions.iter().any(|instruction| {
            instruction.op == tcl_bytecode::Op::START_CMD
                && instruction.source_cmd_text == catch_source
        }));
    }

    #[test]
    fn nested_inline_bodies_retain_every_consumed_command_binding() {
        let module = BytecodeCompileService::default()
            .compile(
                "proc ret {} {return [expr {1 + 2}]}\n\
                 proc caught {} {set rc [catch {expr {1 + 2}} value]; list $rc $value}\n\
                 proc tried {} {set rc [catch {try {expr {1 + 2}} on error {m} {set m handled}} value]; list $rc $value}\n\
                 proc returning {} {set rc [catch {return value} result]; list $rc $result}\n\
                 proc erroring {} {set rc [catch {error boom} result]; list $rc $result}\n\
                 proc breaking {} {set rc [catch {break} result]; list $rc $result}\n\
                 proc continuing {} {set rc [catch {continue} result]; list $rc $result}",
            )
            .unwrap();

        for procedure in ["::ret", "::caught", "::tried"] {
            let bindings = &module.procedures[procedure].command_bindings;
            assert!(
                bindings
                    .iter()
                    .any(|binding| binding.name == "expr" && binding.identity == "expr"),
                "{procedure} lost its nested expr dependency: {bindings:?}",
            );
        }
        assert!(
            module.procedures["::tried"]
                .command_bindings
                .iter()
                .any(|binding| binding.name == "try" && binding.identity == "try"),
            "the catch-body try specialisation must retain its own dependency",
        );
        assert!(
            module.procedures["::tried"]
                .command_bindings
                .iter()
                .any(|binding| binding.name == "set" && binding.identity == "set"),
            "the directly-emitted try handler must retain its lowering dependency",
        );
        for (procedure, dependency) in [
            ("::returning", "return"),
            ("::erroring", "error"),
            ("::breaking", "break"),
            ("::continuing", "continue"),
        ] {
            let bindings = &module.procedures[procedure].command_bindings;
            assert!(
                bindings.iter().any(|binding| {
                    binding.name == dependency && binding.identity == dependency
                }),
                "{procedure} lost its {dependency} inline dependency: {bindings:?}",
            );
        }
    }

    #[test]
    fn entered_command_metadata_is_specialisation_scoped() {
        let service = BytecodeCompileService::default();
        let eligible = service.compile("set result [llength [mutate]]").unwrap();
        let generic = service.compile("set result [foo [mutate]]").unwrap();
        let wrong_arity = service
            .compile("set result [llength [mutate] extra]")
            .unwrap();
        let traced = service
            .compile_traced("set result [llength [mutate]]")
            .unwrap();

        let entered_names = |module: &tcl_bytecode::ModuleAsm| {
            module
                .top_level
                .instructions
                .iter()
                .filter_map(|instruction| {
                    instruction
                        .entered_command
                        .as_ref()
                        .map(|entered| entered.binding.name.clone())
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(entered_names(&eligible), ["llength".to_owned()]);
        assert!(entered_names(&generic).is_empty());
        assert!(entered_names(&wrong_arity).is_empty());
        assert!(entered_names(&traced).is_empty());
    }

    #[test]
    fn every_applicable_typed_codegen_hook_uses_entered_binding_metadata() {
        let cases = [
            ("lassign", "lassign {a b} a b"),
            // Keep const-foldable hooks dynamic: this test exercises the
            // entered-token codegen path, while fold provenance is covered by
            // the constant-substitution tests.
            ("llength", "llength $value"),
            ("lset", "lset value 0 x"),
            ("dict", "dict set value key x"),
            ("array", "array for {k v} value {}"),
            ("namespace", "namespace eval N {}"),
            ("append", "append value x"),
            ("lappend", "lappend value x"),
            ("unset", "unset value"),
            ("tailcall", "tailcall target"),
            ("concat", "concat $value b"),
            ("global", "global value"),
            ("upvar", "upvar 1 remote local"),
        ];
        let service = BytecodeCompileService::default();
        for (name, command) in cases {
            let module = service
                .compile(&format!(
                    "proc target {{}} {{}}; proc p {{}} {{set result [{command}]}}"
                ))
                .unwrap();
            let entered = module.procedures["::p"]
                .instructions
                .iter()
                .filter_map(|instruction| instruction.entered_command.as_ref())
                .find(|entered| entered.binding.name == name);
            assert_eq!(
                entered.map(|entered| entered.binding.identity.as_str()),
                Some(name),
                "{name} lost the applicability-probed entered binding"
            );
            assert!(
                module.procedures["::p"]
                    .command_bindings
                    .iter()
                    .any(|binding| binding.name == name && binding.identity == name),
                "{name} metadata was not retained as a function dependency"
            );
        }
    }

    #[test]
    fn inline_hooks_with_whole_unit_mutation_specialise_and_retain_binding() {
        let cases = [
            (
                "lrange",
                "lrange {a b c} 0 1",
                tcl_bytecode::Op::LIST_RANGE_IMM,
            ),
            ("linsert", "linsert {a b} 1 x", tcl_bytecode::Op::LREPLACE4),
        ];
        let service = BytecodeCompileService::default();
        for (name, command, expected_op) in cases {
            let module = service
                .compile(&format!(
                    "proc mutate {{}} {{rename {name} saved_{name}}}; \
                     proc p {{}} {{set result [{command}]}}"
                ))
                .unwrap();
            assert!(
                module.procedures["::p"]
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == expected_op),
                "{name} lost its inline specialisation"
            );
            assert!(
                module.procedures["::p"]
                    .command_bindings
                    .iter()
                    .any(|binding| binding.name == name && binding.identity == name),
                "{name} lost its runtime-validated command binding"
            );
            assert!(
                module.procedures["::p"]
                    .instructions
                    .iter()
                    .all(|instruction| {
                        !matches!(
                            instruction.op,
                            tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                        )
                    }),
                "{name} unexpectedly fell back to generic dispatch"
            );
        }
    }

    #[test]
    fn return_expr_alias_retains_the_source_binding_identity() {
        let module = BytecodeCompileService::default()
            .compile("interp alias {} e {} expr; proc p {} {return [e {1 + 2}]}")
            .unwrap();
        assert!(
            module.procedures["::p"]
                .command_bindings
                .iter()
                .any(|binding| binding.name == "e" && binding.identity == "expr"),
            "a fused return dependency belongs to the alias source spelling: {:?}",
            module.procedures["::p"].command_bindings,
        );
    }

    #[test]
    fn cfg_edges_keep_ranges_but_are_not_runtime_recompile_sites() {
        let source = "if {[incr i] > 3} { proc continue {} {return -code break} }\ncontinue";
        let module = BytecodeCompileService::default().compile(source).unwrap();
        let edge = module
            .top_level
            .instructions
            .iter()
            .find(|instruction| {
                matches!(
                    instruction.op,
                    tcl_bytecode::Op::JUMP1 | tcl_bytecode::Op::JUMP4
                ) && instruction.source_span.is_some()
            })
            .expect("the if body has a source-mapped CFG edge");
        assert!(edge.source_cmd_text.is_empty());
        assert!(module.top_level.instructions.iter().any(|instruction| {
            instruction.source_cmd_text == "continue"
                && matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                )
        }));
    }

    #[test]
    fn structured_heads_survive_cfg_consumption_as_binding_dependencies() {
        let module = BytecodeCompileService::default()
            .compile("eval {while {0} {}}")
            .unwrap();
        for name in ["eval", "while"] {
            assert!(
                module
                    .top_level
                    .command_bindings
                    .iter()
                    .any(|binding| binding.name == name && binding.identity == name),
                "missing {name:?} dependency: {:?}",
                module.top_level.command_bindings,
            );
        }
    }

    #[test]
    fn foreach_line_binding_dependency_follows_the_resolved_release_profile() {
        let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let tcl86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let source = "foreachLine line file.txt {set seen $line}";

        let current = BytecodeCompileService::for_profile(tcl90)
            .compile(source)
            .unwrap();
        assert!(
            current.top_level.command_bindings.iter().any(|binding| {
                binding.name == "foreachLine" && binding.identity == "foreachLine"
            })
        );

        let legacy = BytecodeCompileService::for_profile(tcl86)
            .compile(source)
            .unwrap();
        assert!(
            legacy
                .top_level
                .command_bindings
                .iter()
                .all(|binding| binding.name != "foreachLine"),
            "an unavailable Tcl 9 command must not acquire an 8.6 binding: {:?}",
            legacy.top_level.command_bindings,
        );
    }

    #[test]
    fn inlining_preserves_callee_structured_bindings_through_cfg_and_codegen() {
        let source = "proc direct {enabled} {while {$enabled} {}}\n\
                      proc wrapped {enabled} {if {$enabled} {set seen 1}; return $enabled}\n\
                      direct 0\n\
                      wrapped 1\n\
                      set done 1";
        let registry = CommandRegistry::build_default();
        let lowered = crate::lowering::lower_to_ir_for_bytecode(source, &registry);
        let inlined = crate::inlining::inline_module(lowered, &registry);

        assert!(
            inlined.top_level.statements.iter().all(|statement| {
                !matches!(
                    statement,
                    crate::ir::Statement::Call { command, .. }
                        if command == "direct" || command == "wrapped"
                )
            }),
            "the regression must exercise both actual inline splices: {:?}",
            inlined.top_level.statements,
        );

        let cfg = build_cfg_codegen_with_registry(&inlined, false, &registry);
        let module = codegen_module(&cfg, &inlined, &registry);
        for name in ["while", "if"] {
            assert!(
                module
                    .top_level
                    .command_bindings
                    .iter()
                    .any(|binding| binding.name == name && binding.identity == name),
                "inlining dropped {name:?} before codegen: {:?}",
                module.top_level.command_bindings,
            );
        }
    }

    #[test]
    fn synthetic_structured_boundaries_have_the_exact_owning_command() {
        let source = "set marker 1; if {1} {set marker 2}; proc p {} {set marker 1; foreach x {a} {if {1} {append marker $x}}; while {0} {}}";
        let module = BytecodeCompileService::default().compile(source).unwrap();
        let boundaries: Vec<&str> = module
            .top_level
            .instructions
            .iter()
            .chain(module.procedures["::p"].instructions.iter())
            .filter(|instruction| instruction.op == tcl_bytecode::Op::START_CMD)
            .map(|instruction| instruction.source_cmd_text.as_str())
            .collect();

        for expected in [
            "if {1} {set marker 2}",
            "foreach x {a} {if {1} {append marker $x}}",
            "if {1} {append marker $x}",
            "while {0} {}",
        ] {
            assert!(
                boundaries.contains(&expected),
                "missing boundary for {expected:?}: {boundaries:?}"
            );
        }
    }

    #[test]
    fn final_constant_if_boundary_keeps_its_replay_continuation() {
        let module = BytecodeCompileService::default()
            .compile("mutate; if {1} {set ::body_ran 1}")
            .unwrap();
        let boundary = module
            .top_level
            .instructions
            .iter()
            .find(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "if {1} {set ::body_ran 1}"
            })
            .expect("constant if has a runtime boundary");
        let tcl_bytecode::Operand::Label(label) = &boundary.operands[0] else {
            panic!("START_CMD continuation is a label: {boundary:?}");
        };
        assert!(
            module.top_level.labels.contains_key(label),
            "missing {label:?}: {:?}",
            module.top_level.labels
        );
    }

    #[test]
    fn proc_constant_if_boundary_has_one_tcl_shaped_owner_in_both_compile_paths() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let body = "mutate; if {1} {set ::body_ran 1}";
        let if_source = "if {1} {set ::body_ran 1}";
        let static_module = service
            .compile_for_profile(&format!("proc p {{}} {{{body}}}"), profile)
            .unwrap();
        let parameters = Vec::new();
        let procedure_module = service
            .compile_procedure_for_profile(
                ProcedureCompileTarget {
                    source: body,
                    parameters: &parameters,
                    namespace: "",
                },
                profile,
                ProcedureDispatch::Optimised,
            )
            .unwrap();

        let static_proc = &static_module.procedures["::p"];
        let direct_proc = &procedure_module.top_level;
        for (path, procedure) in [("static", static_proc), ("direct", direct_proc)] {
            let owners: Vec<_> = procedure
                .instructions
                .iter()
                .filter(|instruction| {
                    instruction.op == tcl_bytecode::Op::START_CMD
                        && instruction.source_cmd_text == if_source
                })
                .collect();
            assert_eq!(owners.len(), 1, "{path} proc owners: {owners:?}");
            let [
                tcl_bytecode::Operand::Label(end),
                tcl_bytecode::Operand::Imm(count),
            ] = owners[0].operands.as_slice()
            else {
                panic!("{path} proc owner has wrong operands: {:?}", owners[0]);
            };
            assert_eq!(*count, 2, "{path} proc owner must count if + body");
            assert!(
                procedure.labels.contains_key(end),
                "{path} proc owner has no replay continuation {end:?}"
            );
            assert!(
                procedure.instructions.iter().all(|instruction| {
                    instruction.op != tcl_bytecode::Op::START_CMD
                        || instruction.source_cmd_text != "set ::body_ran 1"
                }),
                "{path} proc kept a nested marker under the owning if"
            );
        }

        let boundary_shape = |procedure: &tcl_bytecode::FunctionAsm| {
            procedure
                .instructions
                .iter()
                .filter(|instruction| instruction.op == tcl_bytecode::Op::START_CMD)
                .map(|instruction| {
                    (
                        instruction.source_cmd_text.clone(),
                        instruction.operands.get(1).cloned(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(boundary_shape(static_proc), boundary_shape(direct_proc));
    }

    #[test]
    fn proc_constant_if_owner_is_absent_without_an_earlier_generic_invoke() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let cases = [
            "if {1} {set ::body_ran 1}; mutate",
            "set local 0; if {1} {set ::body_ran 1}",
        ];

        for body in cases {
            let static_module = service
                .compile_for_profile(&format!("proc p {{}} {{{body}}}"), profile)
                .unwrap();
            let parameters = Vec::new();
            let direct_module = service
                .compile_procedure_for_profile(
                    ProcedureCompileTarget {
                        source: body,
                        parameters: &parameters,
                        namespace: "",
                    },
                    profile,
                    ProcedureDispatch::Optimised,
                )
                .unwrap();
            for procedure in [&static_module.procedures["::p"], &direct_module.top_level] {
                assert!(
                    procedure.instructions.iter().all(|instruction| {
                        instruction.op != tcl_bytecode::Op::START_CMD
                            || instruction.source_cmd_text != "if {1} {set ::body_ran 1}"
                    }),
                    "unexpected owning if marker for {body:?}: {:?}",
                    procedure.instructions
                );
            }
        }
    }
}
