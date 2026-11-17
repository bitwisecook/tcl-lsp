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
use crate::codegen::codegen_module;
use crate::codegen::emitter::{
    ModuleEmissionScope, codegen_module_with_emission_scope,
    codegen_procedure_module_with_emission_scope,
};
#[cfg(test)]
use tcl_compiler::cfg_builder::build_cfg_codegen_with_registry;
use tcl_compiler::cfg_builder::{
    build_cfg_codegen_with_registry_and_context, prepare_cfg_context_bundle,
};
mod application_input;
mod native_procedure_source;
pub use application_input::ApplicationInputCompileService;
pub use native_procedure_source::NativeCompiledProcedure;

use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use tcl_compiler::lowering::{
    lower_original_procedure_bytes_module_for_bytecode_with_options,
    lower_procedure_bytes_module_for_bytecode_with_options,
    lower_procedure_target_module_for_bytecode_with_options,
    lower_script_bytes_module_for_bytecode_with_options,
    lower_script_module_for_bytecode_with_options,
};
use tcl_dialect::{DialectProfile, DialectProfileKey};
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;
use tcl_registry::model::{KeyedVersions, OverlayMiss};
use tcl_runtime_api::{
    CommandEvaluation, CompileError, CompileService, FatalTail, ProcedureCompileTarget,
    ProcedureCompileTargetBytes, ProcedureDispatch, ScriptCommandPlan, ScriptCompileTarget,
    ScriptCompileTargetBytes,
};

/// The complete source input a fresh interpreter of `profile` gives a whole
/// document: the profile's default availability over the caller's exact
/// command store and its lexer configuration. `None` when the store's
/// metadata cannot correspond to that profile's availability.
pub(crate) fn fresh_document_input(
    registry: &CommandRegistry,
    config: LexerConfig,
    profile: &'static DialectProfile,
) -> Option<tcl_compiler::analyser::ResolvedAnalysisInput> {
    let environment = tcl_registry::model::ingress::resolve_environment(profile.name);
    let mut context = environment.default_context_registry();
    if context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key() {
        context = Arc::new(
            tcl_registry::model::ingress::static_context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
    }
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        &input,
        config,
        Some(profile),
    )?;
    Some(input)
}

/// The native engine a profile-less module runs under: the permissive
/// fallback profile pins its modules to its runtime release's interpreter,
/// whose fresh command world the module is compiled for.
fn profile_less_engine() -> &'static DialectProfile {
    tcl_registry::model::ingress::resolve_environment(
        DialectProfile::plain_tcl()
            .vm_runtime_version
            .dialect_name(),
    )
    .unit_profile()
}

/// Whether the VM's own compile of a procedure body splices pack reference
/// bodies. Off until an error raised inside a spliced body presents as the
/// called procedure's does (its `errorInfo` frame lines and errorstack entry,
/// from the definition's own text); the splice itself is admitted and correct.
pub const LIVE_REFERENCE_INLINING: bool = false;

fn validate_native_function(function: &tcl_bytecode::FunctionAsm) -> Result<(), CompileError> {
    function
        .validate_native_compilation_entry()
        .map_err(CompileError::NativeCompilationAdmission)
}

enum RegistryTarget {
    Owned {
        // The owned command store is larger than the profile handle.
        registry: Box<CommandRegistry>,
        profile_views: Mutex<FxHashMap<DialectProfileKey, Arc<CommandRegistry>>>,
    },
    Profile(&'static CommandRegistry),
    /// The generation a workspace's packs installed under `overlay`, looked
    /// up at every compile: the packs' declarations are part of what a
    /// compile means, so a generation that is not there is a refusal to
    /// compile, never the plain registry under the packs' name.
    Overlay {
        profile: &'static DialectProfile,
        overlay: u64,
    },
    /// One exact shared store, fixed to its profile: a compile for another
    /// profile is refused rather than projected.
    Shared {
        profile: &'static DialectProfile,
        registry: Arc<CommandRegistry>,
    },
}

/// The registry generation `overlay` names for `profile`, if its packs are
/// installed.
fn overlaid_registry(
    profile: &'static DialectProfile,
    overlay: u64,
) -> Result<Arc<CommandRegistry>, OverlayMiss> {
    let generation = tcl_registry::model::ingress::resolve_environment(profile.name)
        .context_registry(&KeyedVersions::default(), overlay)?;
    Ok(Arc::clone(generation.commands()))
}

/// The refusal a missing overlay becomes: the compile does not go ahead.
fn overlay_refusal(miss: &OverlayMiss) -> CompileError {
    CompileError::Unsupported(format!(
        "{miss}: a compile without the workspace's packs would give commands the wrong \
         meaning, so it is declined"
    ))
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
    fn registry(&self) -> Result<ProfileRegistry<'_>, CompileError> {
        match self {
            Self::Owned { registry, .. } => Ok(ProfileRegistry::Borrowed(registry)),
            Self::Profile(registry) => Ok(ProfileRegistry::Borrowed(registry)),
            Self::Overlay { profile, overlay } => overlaid_registry(profile, *overlay)
                .map(ProfileRegistry::Cached)
                .map_err(|miss| overlay_refusal(&miss)),
            Self::Shared { registry, .. } => Ok(ProfileRegistry::Cached(Arc::clone(registry))),
        }
    }

    /// Select the registry axis for an explicit profile compile.
    ///
    /// An owned registry is an embedder's semantic command surface (including
    /// dynamically installed `SpecTcl` hooks), so changing the target profile
    /// must not replace it. A profile-backed service has no such override and
    /// follows the newly requested profile's shared registry generation. An
    /// overlay-backed service follows the requested profile too, at its own
    /// overlay: the workspace's packs are installed for every dialect, and a
    /// profile they are not installed for is a refusal.
    fn registry_for_profile(
        &self,
        profile: &'static DialectProfile,
    ) -> Result<ProfileRegistry<'_>, CompileError> {
        match self {
            Self::Owned {
                registry,
                profile_views,
            } => {
                let mut views = profile_views.lock().expect("profile registry view mutex");
                let view = views
                    .entry(profile.cache_key())
                    .or_insert_with(|| Arc::new(registry.project_for_profile(profile)));
                Ok(ProfileRegistry::Cached(Arc::clone(view)))
            }
            Self::Profile(_) => Ok(ProfileRegistry::Borrowed(
                tcl_registry::model::ingress::static_context_for_profile(profile).commands(),
            )),
            Self::Overlay { overlay, .. } => overlaid_registry(profile, *overlay)
                .map(ProfileRegistry::Cached)
                .map_err(|miss| overlay_refusal(&miss)),
            Self::Shared {
                profile: fixed,
                registry,
            } => {
                if fixed.cache_key() == profile.cache_key() {
                    Ok(ProfileRegistry::Cached(Arc::clone(registry)))
                } else {
                    Err(CompileError::Unsupported(
                        "the shared command store is fixed to its own profile".into(),
                    ))
                }
            }
        }
    }
}

/// A complete compiler-backed [`CompileService`] for the Tcl bytecode VM.
///
/// Construct profile-less/default-registry consumers with [`Self::new`].
/// Release- or dialect-aware consumers use [`Self::for_profile`], which obtains
/// the registry and lexer grammar through the resolved-profile ingress seam.
/// Both forms support optimised and plain-dispatch compilation. Typed script
/// and procedure targets emit only their entered source. The legacy `compile`
/// and `compile_for_profile` entries produce explicit whole-module AOT artifacts.
pub struct BytecodeCompileService {
    registry: RegistryTarget,
    config: LexerConfig,
    profile: Option<&'static DialectProfile>,
}

#[derive(Clone, Copy)]
struct CompileSourcePolicy<'a> {
    entry: Option<&'a tcl_runtime_api::NativeCompilationEntry>,
    scope: ModuleEmissionScope,
}

impl CompileSourcePolicy<'_> {
    const fn whole_module() -> Self {
        Self {
            entry: None,
            scope: ModuleEmissionScope::WholeModule,
        }
    }

    const fn entered(
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    ) -> CompileSourcePolicy<'_> {
        CompileSourcePolicy {
            entry,
            scope: ModuleEmissionScope::EnteredSource,
        }
    }
}

impl BytecodeCompileService {
    /// Build a service for a profile-less registry and the default Tcl grammar.
    #[must_use]
    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            registry: RegistryTarget::Owned {
                registry: Box::new(registry),
                profile_views: Mutex::new(FxHashMap::default()),
            },
            // dialect-drift-ok: the profile-less constructor; a caller that has
            // a document's dialect builds the service with `for_profile`
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

    /// A service over exactly `registry`, fixed to `profile`. Its modules are
    /// compiled against that store itself, never a projection of it.
    #[must_use]
    pub fn for_shared_registry(
        profile: &'static DialectProfile,
        registry: Arc<CommandRegistry>,
    ) -> Self {
        Self {
            registry: RegistryTarget::Shared { profile, registry },
            config: LexerConfig::from_grammar(profile.grammar),
            profile: Some(profile),
        }
    }

    /// Build a service for one resolved dialect profile whose registry
    /// carries the workspace pack overlay `overlay` — the key the packs'
    /// owner installed them under, which the editor's queries thread as
    /// `spec_pack_key`. `0` is no overlay and is [`Self::for_profile`].
    ///
    /// The overlay is looked up, never built: its contents come from the
    /// pack loader, which this crate cannot depend on. It is looked up again
    /// for every compile, so a service built while the packs were installed
    /// still declines to compile once their generation has been retired,
    /// rather than compiling against the registry the packs are missing from.
    ///
    /// # Errors
    ///
    /// [`OverlayMiss`] when `overlay` is non-zero and its packs are not
    /// installed for `profile`.
    pub fn for_profile_with_overlay(
        profile: &'static DialectProfile,
        overlay: u64,
    ) -> Result<Self, OverlayMiss> {
        if overlay == 0 {
            return Ok(Self::for_profile(profile));
        }
        overlaid_registry(profile, overlay)?;
        Ok(Self {
            registry: RegistryTarget::Overlay { profile, overlay },
            config: LexerConfig::from_grammar(profile.grammar),
            profile: Some(profile),
        })
    }

    fn compile_target(
        &self,
        source: &str,
        plain_command_dispatch: bool,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        // A profile-less service compiles for the engine its modules run
        // under, over that engine's view of its owned registry.
        let registry = match self.profile {
            Some(_) => self.registry.registry()?,
            None => self.registry.registry_for_profile(profile_less_engine())?,
        };
        Self::compile_target_with(
            ScriptCompileTarget {
                source,
                namespace: "",
                evaluation: CommandEvaluation::Compiled,
            },
            plain_command_dispatch,
            registry.as_ref(),
            self.config,
            self.profile,
            CompileSourcePolicy::whole_module(),
        )
    }

    fn compile_target_for_profile(
        &self,
        source: &str,
        plain_command_dispatch: bool,
        profile: &'static DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_target_with(
            ScriptCompileTarget {
                source,
                namespace: "",
                evaluation: CommandEvaluation::Compiled,
            },
            plain_command_dispatch,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
            CompileSourcePolicy::whole_module(),
        )
    }

    fn compile_target_with(
        target: ScriptCompileTarget<'_>,
        plain_command_dispatch: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: Option<&'static DialectProfile>,
        policy: CompileSourcePolicy<'_>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let native_entry = policy.entry;
        let config = Self::native_entry_config(config, native_entry);
        if let Some(cut) = tcl_lexer::first_parse_cut_image_checked(
            &tcl_runtime_api::SourceImage::document(target.source),
            config,
        )
        .map_err(|error| CompileError::Unsupported(error.to_string()))?
        {
            return Err(CompileError::Message(cut.message.into()));
        }
        // A whole module compiled with no explicit entry is a document a
        // fresh interpreter of its profile runs, as `tclsh file.tcl` does
        // (user decision 2026-10-19T14:20Z): it takes the supplied-source
        // entry whole-document analysis takes, over the profile's complete
        // input; a profile-less module is compiled for the engine the VM
        // runs it under. A supplied runtime
        // entry keeps its own facts; without a corresponding complete input
        // the module keeps refusing.
        let fresh_profile = profile.unwrap_or_else(profile_less_engine);
        let fresh_input = (native_entry.is_none()
            && policy.scope == ModuleEmissionScope::WholeModule)
            .then(|| fresh_document_input(registry, config, fresh_profile))
            .flatten();
        let compiled =
            |ir| Self::finish_module_compile(ir, target, registry, config, profile, policy);
        match fresh_input {
            Some(input) => {
                let fresh = |compiles: bool| {
                    Self::lower_fresh_document(
                        target,
                        registry,
                        config,
                        fresh_profile,
                        plain_command_dispatch,
                        input.clone(),
                        compiles,
                    )
                };
                // Where the profile's catalogue cannot close a compiler the
                // fresh interpreter would run (a command with no authored
                // compiler descriptor), the document is evaluated command by
                // command instead, as before: the same fresh world, without
                // native compiler selection.
                match compiled(fresh(true)) {
                    Err(CompileError::NativeCompilationAdmission(_)) => compiled(fresh(false)),
                    result => result,
                }
            }
            None => compiled(lower_script_module_for_bytecode_with_options(
                target.source,
                target.namespace,
                registry,
                config,
                profile,
                plain_command_dispatch,
                Self::source_analysis_options(policy, registry, config, profile),
            )),
        }
    }

    /// Prepare, emit and admit a lowered module.
    fn finish_module_compile(
        ir: tcl_compiler::ir::Module,
        target: ScriptCompileTarget<'_>,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: Option<&'static DialectProfile>,
        policy: CompileSourcePolicy<'_>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let mut ir = crate::inlining::inline_reference_bodies(ir, registry, config, profile);
        ir.top_level_evaluation = target.evaluation;
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let command_mutations =
            tcl_compiler::command_binding::scan_module_command_mutations_with_bindings(
                &ir,
                registry,
                prepared.command_bindings(),
            );
        let mut module = codegen_module_with_emission_scope(
            &cfg,
            &ir,
            registry,
            &command_mutations,
            policy.scope,
        )?;
        // A profile-less module is the permissive fallback profile's, whose
        // engine it was compiled for.
        let carried = profile.unwrap_or_else(DialectProfile::plain_tcl);
        if !std::ptr::eq(module.profile, carried) {
            // The module states the world of the profile it carries.
            module.profile = carried;
            module.manifest = Some(Arc::new(crate::codegen::emitter::module_manifest(&module)));
        }
        validate_native_function(&module.top_level)?;
        Ok(module)
    }

    /// Lower a whole document for a fresh interpreter of `profile`: the
    /// supplied-source entry over the complete `input`, retained as the
    /// lowering's own metadata so the procedures the document defines are
    /// published as it runs them. `compiles` selects the script-object
    /// compilers of `tclsh file.tcl`; without it each command is evaluated
    /// directly.
    fn lower_fresh_document(
        target: ScriptCompileTarget<'_>,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: &'static DialectProfile,
        plain_command_dispatch: bool,
        input: tcl_compiler::analyser::ResolvedAnalysisInput,
        compiles: bool,
    ) -> tcl_compiler::ir::Module {
        let entry = tcl_compiler::command_binding::SourceAnalysisEntry::for_supplied_source(
            registry,
            &input,
            config,
            Some(profile),
        );
        let mut options = entry.options();
        options.compilation_scope = ModuleEmissionScope::WholeModule;
        // The VM's embedder can register commands the profile's catalogue
        // does not describe (`Vm::enable_threads` installs `thread::id`).
        options.host_extensible_command_table = true;
        // `tclsh file.tcl` compiles the file as one script object
        // (`Tcl_EvalObjEx` without `TCL_EVAL_DIRECT`): its commands take
        // their release's compilers rather than direct evaluation. A hosted
        // (F5) rule keeps the fresh rule loader's own unmeasured protocol.
        if compiles && options.vendor_source_input.is_none() {
            options.native_compilation.mode =
                tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject;
        }
        let mut lowerer = tcl_compiler::lowering::Lowerer::with_config(registry, config)
            .with_dialect(Some(profile))
            .for_bytecode_backend();
        if plain_command_dispatch {
            lowerer = lowerer.trace_visible();
        }
        let mut lowerer = lowerer.with_resolved_analysis_input(input);
        lowerer.set_source_analysis_options(options);
        lowerer.lower_script_target(target.source, target.namespace);
        lowerer.finish_module(target.source)
    }

    /// Translate a live runtime entry into the shared source-analysis contract.
    /// Custom compiler pipelines use this same adapter before applying passes.
    #[must_use]
    pub fn native_entry_options<'a>(
        entry: &'a tcl_runtime_api::NativeCompilationEntry,
        _profile: Option<&'static DialectProfile>,
    ) -> tcl_compiler::command_binding::SourceAnalysisOptions<'a> {
        tcl_compiler::command_binding::native_entry_options(entry)
    }

    fn source_analysis_options<'a>(
        policy: CompileSourcePolicy<'a>,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: Option<&'static DialectProfile>,
    ) -> Option<tcl_compiler::command_binding::SourceAnalysisOptions<'a>> {
        if policy.scope == ModuleEmissionScope::WholeModule && policy.entry.is_none() {
            return None;
        }
        let mut options = policy.entry.map_or_else(
            || tcl_compiler::command_binding::SourceAnalysisOptions {
                invocation_realm: tcl_dialect::model::InvocationRealm::RuleLoader,
                invocation_dialect: Some(
                    tcl_compiler::environment_ingress::authoring_invocation_dialect(
                        registry, profile, config,
                    ),
                ),
                native_compilation: tcl_compiler::environment_ingress::authoring_native_compilation(
                ),
                ..tcl_compiler::command_binding::SourceAnalysisOptions::default()
            },
            |entry| Self::native_entry_options(entry, profile),
        );
        options.compilation_scope = policy.scope;
        Some(options)
    }

    /// Select a measured interpreter lexical grammar for runtime compilation.
    /// The catalogue profile remains independent; absent native evidence retains
    /// the explicitly supplied configuration, including custom grammar axes.
    #[must_use]
    pub fn native_entry_config(
        config: LexerConfig,
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    ) -> LexerConfig {
        tcl_compiler::command_binding::native_entry_config(config, entry)
    }

    fn compile_procedure_target_with(
        target: ProcedureCompileTarget<'_>,
        plain_command_dispatch: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: &'static DialectProfile,
        native_entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let config = Self::native_entry_config(config, native_entry);
        if let Some(cut) = tcl_lexer::first_parse_cut_image_checked(
            &tcl_runtime_api::SourceImage::document(target.source),
            config,
        )
        .map_err(|error| CompileError::Unsupported(error.to_string()))?
        {
            return Err(CompileError::Message(cut.message.into()));
        }
        let ir = lower_procedure_target_module_for_bytecode_with_options(
            target,
            registry,
            config,
            Some(profile),
            plain_command_dispatch,
            Self::source_analysis_options(
                CompileSourcePolicy::entered(native_entry),
                registry,
                config,
                Some(profile),
            ),
        );
        let ir = crate::inlining::inline_reference_bodies(ir, registry, config, Some(profile));
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let command_mutations =
            tcl_compiler::command_binding::scan_module_command_mutations_with_bindings(
                &ir,
                registry,
                prepared.command_bindings(),
            );
        let params: Vec<_> = target
            .parameters
            .iter()
            .map(tcl_runtime_api::NameBytes::from)
            .collect();
        let mut module = codegen_procedure_module_with_emission_scope(
            &cfg,
            &ir,
            &params,
            registry,
            &command_mutations,
            ModuleEmissionScope::EnteredSource,
        )?;
        module.profile = profile;
        validate_native_function(&module.top_level_body)?;
        Ok(module)
    }
    fn compile_script_bytes_target_with(
        target: ScriptCompileTargetBytes<'_>,
        plain: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: &'static DialectProfile,
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let config = Self::native_entry_config(config, entry);
        reject_malformed_byte_script(target.source, config)?;
        let mut ir = lower_script_bytes_module_for_bytecode_with_options(
            target,
            registry,
            config,
            Some(profile),
            plain,
            Self::source_analysis_options(
                CompileSourcePolicy::entered(entry),
                registry,
                config,
                Some(profile),
            ),
        )
        .map_err(native_word_compile_error)?;
        // A script the release evaluates directly (C8.4's `eval`, `uplevel`
        // and `source`) invokes its own commands, which log as they fail.
        if entry.is_some_and(|entry| entry.direct_evaluation) {
            ir.top_level_evaluation = tcl_runtime_api::CommandEvaluation::Direct;
        }
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let mutations = tcl_compiler::command_binding::scan_module_command_mutations_with_bindings(
            &ir,
            registry,
            prepared.command_bindings(),
        );
        let mut module = codegen_module_with_emission_scope(
            &cfg,
            &ir,
            registry,
            &mutations,
            ModuleEmissionScope::EnteredSource,
        )?;
        module.profile = profile;
        validate_native_function(&module.top_level)?;
        Ok(module)
    }

    fn compile_procedure_bytes_target_with(
        target: ProcedureCompileTargetBytes<'_>,
        plain: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: &'static DialectProfile,
        entry: Option<&tcl_runtime_api::NativeCompilationEntry>,
        original: Option<&tcl_runtime_api::OriginalProcedureCompileRequest>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let config = Self::native_entry_config(config, entry);
        reject_malformed_byte_script(target.source, config)?;
        let options = Self::source_analysis_options(
            CompileSourcePolicy::entered(entry),
            registry,
            config,
            Some(profile),
        );
        let ir = match original {
            Some(request) => lower_original_procedure_bytes_module_for_bytecode_with_options(
                request,
                registry,
                config,
                Some(profile),
                plain,
                options,
            ),
            None => lower_procedure_bytes_module_for_bytecode_with_options(
                target,
                registry,
                config,
                Some(profile),
                plain,
                options,
            ),
        }
        .map_err(native_word_compile_error)?;
        let emit = |ir: &tcl_compiler::ir::Module| {
            let prepared = prepare_cfg_context_bundle(ir, registry);
            let cfg =
                build_cfg_codegen_with_registry_and_context(ir, false, registry, &prepared, config);
            let mutations =
                tcl_compiler::command_binding::scan_module_command_mutations_with_bindings(
                    ir,
                    registry,
                    prepared.command_bindings(),
                );
            let mut module = codegen_procedure_module_with_emission_scope(
                &cfg,
                ir,
                target.parameters,
                registry,
                &mutations,
                ModuleEmissionScope::EnteredSource,
            )?;
            module.profile = profile;
            validate_native_function(&module.top_level_body)?;
            Ok(module)
        };
        // A pack's definition is spliced only where the whole function is
        // then admitted: otherwise the caller compiles as it would without it.
        if LIVE_REFERENCE_INLINING
            && entry.is_some()
            && let Some(options) = options
            && let Some(inlined) = crate::inlining::inline_reference_bodies_live(
                &ir,
                registry,
                config,
                profile,
                options,
                || fresh_document_input(registry, config, profile),
            )
            && let Ok(module) = emit(&inlined)
        {
            return Ok(module);
        }
        emit(&ir)
    }
}

impl Default for BytecodeCompileService {
    fn default() -> Self {
        Self::new(CommandRegistry::build_default())
    }
}

impl CompileService for BytecodeCompileService {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile)?;
        crate::codegen::native_substitution::compile(target, profile, entry, registry.as_ref())
    }

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
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_target_with(
            target,
            false,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
            CompileSourcePolicy::entered(None),
        )
    }

    fn compile_script_with_entry(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        if entry.profile != profile.cache_key() {
            return Err(CompileError::Unsupported(
                "native compilation entry profile does not match target".to_owned(),
            ));
        }
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_target_with(
            target,
            false,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
            CompileSourcePolicy::entered(Some(entry)),
        )
    }

    fn compile_original_procedure_bytes_with_entry(
        &self,
        request: &tcl_runtime_api::OriginalProcedureCompileRequest,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if !request.matches_entry(entry) {
            return Err(CompileError::Unsupported(
                "original procedure compilation owner is unavailable".to_owned(),
            ));
        }
        require_entry_profile(entry, profile)?;
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_procedure_bytes_target_with(
            request.target(),
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            Some(entry),
            Some(request),
        )
    }

    fn compile_procedure_with_entry(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if entry.profile != profile.cache_key() {
            return Err(CompileError::Unsupported(
                "native compilation entry profile does not match target".to_owned(),
            ));
        }
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_procedure_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            Some(entry),
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
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_target_with(
            target,
            true,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
            CompileSourcePolicy::entered(None),
        )
    }

    fn compile_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_script_bytes_target_with(
            target,
            false,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            None,
        )
    }

    fn compile_plain_script_bytes_for_profile(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_script_bytes_target_with(
            target,
            true,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            None,
        )
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        require_entry_profile(entry, profile)?;
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_script_bytes_target_with(
            target,
            false,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            Some(entry),
        )
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        require_entry_profile(entry, profile)?;
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_script_bytes_target_with(
            target,
            true,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            Some(entry),
        )
    }

    fn compile_procedure_bytes_for_profile(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_procedure_bytes_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            None,
            None,
        )
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        require_entry_profile(entry, profile)?;
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_procedure_bytes_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            Some(entry),
            None,
        )
    }

    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        byte_script_command_plan(source, LexerConfig::from_grammar(profile.grammar))
    }

    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        require_entry_profile(entry, profile)?;
        byte_script_command_plan(
            source,
            Self::native_entry_config(LexerConfig::from_grammar(profile.grammar), Some(entry)),
        )
    }

    fn script_command_plan_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
    ) -> Result<ScriptCommandPlan, CompileError> {
        let segmented = tcl_compiler::lowering::command_at_time_script_with_config(
            source,
            LexerConfig::from_grammar(profile.grammar),
        )
        .map_err(|error| CompileError::Unsupported(error.to_string()))?;
        Ok(match segmented.fatal_tail {
            Some(tail) => ScriptCommandPlan {
                complete_prefix_len: tail.command.span.start() as usize,
                // `command_at_time_script_with_config` truncates the command
                // list at the cut, so this is exactly the prefix's command
                // count — zero when the *first* command is the malformed one,
                // however much leading whitespace or comment `start` spans.
                complete_prefix_commands: segmented.commands.len(),
                fatal_tail: Some(fatal_tail_frame_bytes(
                    source.as_bytes(),
                    tail.command.span.start() as usize,
                    tail.cut,
                )),
            },
            None => ScriptCommandPlan::complete(source.len()),
        })
    }

    fn compile_failure_for_profile(
        &self,
        source: &str,
        profile: &'static DialectProfile,
        unit: tcl_runtime_api::CompileUnit,
    ) -> Option<tcl_runtime_api::CompileFailure> {
        let registry = self.registry.registry_for_profile(profile).ok()?;
        tcl_compiler::compile_84::compile_failure(source, registry.as_ref(), profile, unit)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let registry = self.registry.registry_for_profile(profile)?;
        Self::compile_procedure_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            None,
        )
    }
}

fn require_entry_profile(
    entry: &tcl_runtime_api::NativeCompilationEntry,
    profile: &'static DialectProfile,
) -> Result<(), CompileError> {
    if entry.profile == profile.cache_key() {
        Ok(())
    } else {
        Err(CompileError::Unsupported(
            "native compilation entry profile does not match target".into(),
        ))
    }
}

fn native_word_compile_error(error: tcl_lexer::NativeWordError) -> CompileError {
    match error {
        tcl_lexer::NativeWordError::Parse(message) => CompileError::Message(message.into()),
        error => CompileError::Unsupported(format!(
            "original byte word provenance is unavailable: {error:?}"
        )),
    }
}

fn reject_malformed_byte_script(
    source: &tcl_lexer::SourceImage,
    config: LexerConfig,
) -> Result<(), CompileError> {
    if let Some(tail) = byte_script_command_plan(source, config)?.fatal_tail {
        Err(CompileError::Message(tail.message))
    } else {
        Ok(())
    }
}

/// Build the malformed tail's error with the context C's `while executing`
/// frame quotes.
///
/// C reports a parse failure through
/// `Tcl_LogCommandInfo(interp, script, parsePtr->commandStart,
/// parsePtr->term + 1 - parsePtr->commandStart)`, so the quoted text runs from
/// the command's first byte **through the character that opened the
/// unterminated construct**, inclusive — not to the end of the source. The two
/// differ whenever anything follows that character:
///
/// | source | C quotes |
/// |---|---|
/// | `set x "` | `set x "` |
/// | `set x "abc\ndef` | `set x "` |
/// | `set x [foo bar` | `set x [` |
///
/// The cut owner retains the actual nested failure's term separately from its
/// enclosing reporting offset. Validation checks that retained term against
/// the original source and issue kind, independently of the guest message.
///
/// C's term for an unterminated construct is the character that opened it, so
/// the pair is self-checking. An issue C reports *in place* — the
/// `extra characters after …` family — constrains nothing, and is accepted.
fn term_opens_the_named_construct(
    source: &[u8],
    term: usize,
    kind: tcl_lexer::ParseIssueKind,
) -> bool {
    let opener = match kind {
        tcl_lexer::ParseIssueKind::MissingQuote => b'"',
        tcl_lexer::ParseIssueKind::MissingBrace => b'{',
        tcl_lexer::ParseIssueKind::MissingBracket => b'[',
        _ => return true,
    };
    source.get(term) == Some(&opener)
}

fn fatal_tail_frame_bytes(source: &[u8], start: usize, cut: tcl_lexer::ParseCut) -> FatalTail {
    let message = cut.message.to_owned();
    let end = Some(cut.term)
        .map(|offset| offset as usize)
        .filter(|offset| *offset >= start && *offset < source.len())
        .filter(|offset| term_opens_the_named_construct(source, *offset, cut.kind))
        .map(|offset| offset + 1);
    let Some(end) = end else {
        return FatalTail::message_only(message);
    };
    let command_text = source.get(start..end).unwrap_or_default().to_vec();
    let line = u32::try_from(
        source
            .get(..start)
            .unwrap_or_default()
            .split(|&byte| byte == b'\n')
            .count(),
    )
    .unwrap_or(u32::MAX);
    FatalTail {
        message,
        command_text,
        compilation_command_text: tcl_syntax::native_parse_context::c84_compilation_command_extent(
            source,
            start,
            end - 1,
        )
        .map(<[u8]>::to_vec),
        line,
        compile_info: None,
        error_code: None,
    }
}

fn byte_script_command_plan(
    source: &tcl_runtime_api::SourceImage,
    config: LexerConfig,
) -> Result<ScriptCommandPlan, CompileError> {
    // Tolerant lexical tokens let the shared cut owner retain the first
    // malformed command, including when strict native compilation rejects it.
    let parse_config = LexerConfig {
        strict_quoting: false,
        ..config
    };
    let tokens = tcl_lexer::Lexer::with_source_image(source, parse_config)
        .tokenise_all()
        .map_err(|error| CompileError::Message(error.to_string()))?;
    let commands = tcl_lexer::group_commands_bytes(&tokens, source.bytes(), parse_config);
    let cut = tcl_lexer::first_parse_cut_image_in_checked(&commands, &tokens, source, parse_config)
        .map_err(|error| CompileError::Unsupported(error.to_string()))?;
    match cut {
        Some(cut) => {
            let start = commands
                .get(cut.command)
                .map_or(0, |command| command.span.start() as usize);
            Ok(ScriptCommandPlan {
                complete_prefix_len: start,
                complete_prefix_commands: cut.command,
                fatal_tail: Some(fatal_tail_frame_bytes(source.bytes(), start, cut)),
            })
        }
        None => Ok(ScriptCommandPlan {
            complete_prefix_len: source.len(),
            complete_prefix_commands: commands.len(),
            fatal_tail: None,
        }),
    }
}

#[cfg(test)]
mod tests;
