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
use crate::codegen::emitter::{
    ModuleEmissionScope, codegen_module_with_emission_scope,
    codegen_procedure_module_with_emission_scope,
};
use crate::lowering::{
    lower_procedure_bytes_module_for_bytecode_with_options,
    lower_procedure_target_module_for_bytecode_with_options,
    lower_script_bytes_module_for_bytecode_with_options,
    lower_script_module_for_bytecode_with_options,
};
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};
use tcl_dialect::{DialectProfile, DialectProfileKey};
use tcl_lexer::LexerConfig;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::{
    CompileError, CompileService, FatalTail, ProcedureCompileTarget, ProcedureCompileTargetBytes,
    ProcedureDispatch, ScriptCommandPlan, ScriptCompileTarget, ScriptCompileTargetBytes,
};

fn validate_native_function(function: &tcl_bytecode::FunctionAsm) -> Result<(), CompileError> {
    function
        .validate_native_compilation_entry()
        .map_err(CompileError::NativeCompilationAdmission)
}

enum RegistryTarget {
    Owned {
        registry: CommandRegistry,
        profile_views: Mutex<FxHashMap<DialectProfileKey, Arc<CommandRegistry>>>,
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
                    .entry(profile.cache_key())
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
            CompileSourcePolicy::whole_module(),
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
            CompileSourcePolicy::whole_module(),
        )
    }

    fn compile_target_with(
        source: &str,
        namespace: &str,
        plain_command_dispatch: bool,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: Option<&'static DialectProfile>,
        policy: CompileSourcePolicy<'_>,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let native_entry = policy.entry;
        let config = Self::native_entry_config(config, native_entry);
        if let Some(cut) = tcl_lexer::first_parse_cut_image_checked(
            &tcl_runtime_api::SourceImage::document(source),
            config,
        )
        .map_err(|error| CompileError::Unsupported(error.to_string()))?
        {
            return Err(CompileError::Message(cut.message.into()));
        }
        let ir = lower_script_module_for_bytecode_with_options(
            source,
            namespace,
            registry,
            config,
            profile,
            plain_command_dispatch,
            Self::source_analysis_options(policy, registry, config, profile),
        );
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let command_mutations = crate::command_binding::scan_module_command_mutations_with_bindings(
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
        );
        if let Some(profile) = profile {
            module.profile = profile;
        }
        validate_native_function(&module.top_level)?;
        Ok(module)
    }

    /// Translate a live runtime entry into the shared source-analysis contract.
    /// Custom compiler pipelines use this same adapter before applying passes.
    #[must_use]
    pub fn native_entry_options<'a>(
        entry: &'a tcl_runtime_api::NativeCompilationEntry,
        _profile: Option<&'static DialectProfile>,
    ) -> crate::command_binding::SourceAnalysisOptions<'a> {
        use tcl_registry::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
        };
        let mut options = crate::command_binding::SourceAnalysisOptions {
            compilation_scope: ModuleEmissionScope::EnteredSource,
            native_entry: Some(entry),
            native_compilation: NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            ..crate::command_binding::SourceAnalysisOptions::default()
        };
        options.invocation_dialect = options.logical_invocation_dialect();
        options
    }

    fn source_analysis_options<'a>(
        policy: CompileSourcePolicy<'a>,
        registry: &CommandRegistry,
        config: LexerConfig,
        profile: Option<&'static DialectProfile>,
    ) -> Option<crate::command_binding::SourceAnalysisOptions<'a>> {
        if policy.scope == ModuleEmissionScope::WholeModule && policy.entry.is_none() {
            return None;
        }
        let mut options = policy.entry.map_or_else(
            || crate::command_binding::SourceAnalysisOptions {
                invocation_realm: tcl_dialect::model::InvocationRealm::RuleLoader,
                invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                    registry, profile, config,
                )),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..crate::command_binding::SourceAnalysisOptions::default()
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
        crate::command_binding::SourceAnalysisOptions {
            native_entry: entry,
            ..crate::command_binding::SourceAnalysisOptions::default()
        }
        .native_lexer_config(config)
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
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let command_mutations = crate::command_binding::scan_module_command_mutations_with_bindings(
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
        );
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
        let ir = lower_script_bytes_module_for_bytecode_with_options(
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
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let mutations = crate::command_binding::scan_module_command_mutations_with_bindings(
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
        );
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
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let config = Self::native_entry_config(config, entry);
        reject_malformed_byte_script(target.source, config)?;
        let ir = lower_procedure_bytes_module_for_bytecode_with_options(
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
        let prepared = prepare_cfg_context_bundle(&ir, registry);
        let cfg =
            build_cfg_codegen_with_registry_and_context(&ir, false, registry, &prepared, config);
        let mutations = crate::command_binding::scan_module_command_mutations_with_bindings(
            &ir,
            registry,
            prepared.command_bindings(),
        );
        let mut module = codegen_procedure_module_with_emission_scope(
            &cfg,
            &ir,
            target.parameters,
            registry,
            &mutations,
            ModuleEmissionScope::EnteredSource,
        );
        module.profile = profile;
        validate_native_function(&module.top_level_body)?;
        Ok(module)
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
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_target_with(
            target.source,
            target.namespace,
            false,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
            CompileSourcePolicy::entered(Some(entry)),
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
        let registry = self.registry.registry_for_profile(profile);
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
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_target_with(
            target.source,
            target.namespace,
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
        let registry = self.registry.registry_for_profile(profile);
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
        let registry = self.registry.registry_for_profile(profile);
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
        let registry = self.registry.registry_for_profile(profile);
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
        let registry = self.registry.registry_for_profile(profile);
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
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_procedure_bytes_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
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
        let registry = self.registry.registry_for_profile(profile);
        Self::compile_procedure_bytes_target_with(
            target,
            dispatch == ProcedureDispatch::Plain,
            registry.as_ref(),
            LexerConfig::from_grammar(profile.grammar),
            profile,
            Some(entry),
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
        let segmented = crate::lowering::command_at_time_script_with_config(
            source,
            LexerConfig::from_grammar(profile.grammar),
        )
        .map_err(|error| CompileError::Unsupported(error.to_string()))?;
        Ok(match segmented.fatal_tail {
            Some((start, message, delimiter_offset)) => ScriptCommandPlan {
                complete_prefix_len: start,
                // `command_at_time_script_with_config` truncates the command
                // list at the cut, so this is exactly the prefix's command
                // count — zero when the *first* command is the malformed one,
                // however much leading whitespace or comment `start` spans.
                complete_prefix_commands: segmented.commands.len(),
                fatal_tail: Some(fatal_tail_frame(source, start, message, delimiter_offset)),
            },
            None => ScriptCommandPlan::complete(source.len()),
        })
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
/// the original source. It cannot manufacture a missing term from a message.
/// Whether the byte at `term` opens the construct `message` names.
///
/// C's term for an unterminated construct is the character that opened it, so
/// the pair is self-checking. A message C reports *in place* — the
/// `extra characters after …` family — constrains nothing, and is accepted.
fn term_opens_the_named_construct(source: &[u8], term: usize, message: &str) -> bool {
    let opener = match message {
        tcl_lexer::word_parts::MISSING_QUOTE => b'"',
        tcl_lexer::word_parts::MISSING_CLOSE_BRACE => b'{',
        tcl_lexer::word_parts::MISSING_CLOSE_BRACKET => b'[',
        _ => return true,
    };
    source.get(term) == Some(&opener)
}

fn fatal_tail_frame(
    source: &str,
    start: usize,
    message: String,
    delimiter_offset: Option<u32>,
) -> FatalTail {
    fatal_tail_frame_bytes(source.as_bytes(), start, message, delimiter_offset)
}

fn fatal_tail_frame_bytes(
    source: &[u8],
    start: usize,
    message: String,
    delimiter_offset: Option<u32>,
) -> FatalTail {
    let end = delimiter_offset
        .map(|offset| offset as usize)
        .filter(|offset| *offset >= start && *offset < source.len())
        .filter(|offset| term_opens_the_named_construct(source, *offset, &message))
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
                fatal_tail: Some(fatal_tail_frame_bytes(
                    source.bytes(),
                    start,
                    cut.message.to_owned(),
                    Some(cut.term),
                )),
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
mod tests {
    use super::*;
    use crate::lowering::{
        lower_proc_body_module_for_bytecode, lower_to_ir_for_bytecode_with_dialect,
    };

    #[test]
    fn fatal_tail_preserves_distinct_native_runtime_and_compilation_extents() {
        let decode = |input: &str| {
            input
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let mut compared = 0;
        for row in include_str!("../../tcl-registry/tests/data/native_c84_parse_context/8.4.20.tsv")
            .lines()
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            if fields[1] == "0" {
                continue;
            }
            let source = decode(fields[6]);
            let start = fields[2].parse::<usize>().unwrap();
            let term = fields[4].parse::<usize>().unwrap();
            let message = String::from_utf8(decode(fields[7])).unwrap();
            let tail =
                fatal_tail_frame_bytes(&source, start, message, Some(u32::try_from(term).unwrap()));
            assert_eq!(tail.command_text, source[start..=term], "{row}");
            let end = if term + 1 == source.len() {
                term
            } else {
                source.len()
            };
            assert_eq!(
                tail.compilation_command_text.as_deref(),
                Some(&source[start..end]),
                "{row}"
            );
            assert_eq!(
                tail.line,
                u32::try_from(
                    source[..start]
                        .iter()
                        .filter(|&&byte| byte == b'\n')
                        .count()
                        + 1
                )
                .unwrap(),
                "{row}"
            );
            compared += 1;
        }
        assert_eq!(compared, 26);
        let unavailable = fatal_tail_frame_bytes(b"set x [bad", 0, "missing \"".into(), Some(6));
        assert!(unavailable.compilation_command_text.is_none());
        assert!(unavailable.command_text.is_empty());
    }

    #[test]
    fn original_byte_command_plan_keeps_c84_compilation_tail_and_runtime_term() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let service = BytecodeCompileService::for_profile(profile);
        let cases: &[(&[u8], &[u8], &[u8], u32)] = &[
            (b"set x \"", b"set x \"", b"set x ", 1),
            (b"set x \"abc", b"set x \"", b"set x \"abc", 1),
            (b"set x {abc", b"set x {", b"set x {abc", 1),
            (b"set x [bad", b"set x [", b"set x [bad", 1),
            (
                b"set x [bad x \"abc",
                b"set x [bad x \"",
                b"set x [bad x \"abc",
                1,
            ),
            (b"set x [bad {", b"set x [bad {", b"set x [bad ", 1),
            (
                b"set before OK\nset x \"abc",
                b"set x \"",
                b"set x \"abc",
                2,
            ),
            (
                b"set x \"\xc3\xa9abc",
                b"set x \"",
                b"set x \"\xc3\xa9abc",
                1,
            ),
            (
                b"set x \"\xf0\x9f\x98\x80abc",
                b"set x \"",
                b"set x \"\xf0\x9f\x98\x80abc",
                1,
            ),
            (b"set x \"\xffabc", b"set x \"", b"set x \"\xffabc", 1),
            (b"set x \"\0abc", b"set x \"", b"set x \"\0abc", 1),
            (b"\"", b"\"", b"", 1),
            (b"{", b"{", b"", 1),
        ];
        for &(source, runtime, compilation, line) in cases {
            let source = tcl_runtime_api::SourceImage::native(source);
            let tail = service
                .script_command_plan_bytes_for_profile(&source, profile)
                .unwrap()
                .fatal_tail
                .unwrap();
            assert_eq!(tail.command_text, runtime, "{source:?}");
            assert_eq!(
                tail.compilation_command_text.as_deref(),
                Some(compilation),
                "{source:?}"
            );
            assert_eq!(tail.line, line, "{source:?}");
        }
    }

    #[test]
    fn immutable_byte_entry_lookup_keeps_names_incarnations_and_residuals() {
        use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCommandLookupUnavailable, NativeCompilationBinding,
            NativeCompilationNamespace, NativeCompilerHookPresence,
        };
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let point = tcl_registry::InvocationDialect::of_profile(profile)
            .core_point
            .unwrap();
        let mut entry = policy_entry(profile, point);
        let local = ByteNamespacePath::from_segments([b"N".as_slice()]);
        entry.closed = true;
        entry.current_namespace = 2;
        entry.namespaces = vec![
            NativeCompilationNamespace {
                path: ByteNamespacePath::root(),
                jim_namespace_object: None,
                token: 1,
                visible: true,
                exports: vec![],
                command_path: vec![],
                unknown_handler: None,
            },
            NativeCompilationNamespace {
                path: local.clone(),
                jim_namespace_object: None,
                token: 2,
                visible: false,
                exports: vec![],
                command_path: vec![],
                unknown_handler: None,
            },
            NativeCompilationNamespace {
                path: local.clone(),
                jim_namespace_object: None,
                token: 9,
                visible: true,
                exports: vec![],
                command_path: vec![],
                unknown_handler: None,
            },
        ];
        let binding = |namespace_token, token| NativeCompilationBinding {
            slot: NativeByteCommandSlot::new(local.clone(), NameBytes::from(b"k\xff".as_slice())),
            namespace_token,
            token,
            implementation_generation: token,
            implementation: NativeCommandImplementation::Opaque,
            compiler_hook: NativeCompilerHookPresence::Absent,
            compiler: None,
            procedure_header: None,
            has_execution_trace: false,
        };
        entry.commands = vec![binding(2, 12), binding(9, 19)];
        assert_eq!(
            entry
                .lookup_command_bytes(2, b"k\xff\0tail")
                .unwrap()
                .unwrap()
                .token,
            12
        );
        assert_eq!(
            entry
                .lookup_command_bytes(2, b"::N::k\xff")
                .unwrap()
                .unwrap()
                .token,
            19
        );
        assert!(entry.lookup_command_bytes(2, b"absent").unwrap().is_none());
        assert_eq!(
            entry.lookup_command_bytes(2, b"child::k\xff"),
            Err(NativeCommandLookupUnavailable::RetainedDescendant)
        );
        entry.commands.push(binding(2, 22));
        assert_eq!(
            entry.lookup_command_bytes(2, b"k\xff"),
            Err(NativeCommandLookupUnavailable::ConflictingBinding)
        );
        entry.closed = false;
        assert_eq!(
            entry.lookup_command_bytes(2, b"k\xff"),
            Err(NativeCommandLookupUnavailable::OpenTable)
        );
    }

    #[test]
    fn byte_procedure_target_seeds_original_nonunicode_formal_keys() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let source = tcl_runtime_api::SourceImage::native(b"return OK".as_slice());
        let namespace = tcl_core_types::ByteNamespacePath::root();
        let parameters = [
            tcl_core_types::NameBytes::from(b"p\xff".as_slice()),
            tcl_core_types::NameBytes::from(b"a(k)".as_slice()),
        ];
        let module = service
            .compile_procedure_bytes_for_profile(
                tcl_runtime_api::ProcedureCompileTargetBytes {
                    source: &source,
                    parameters: &parameters,
                    namespace: &namespace,
                },
                profile,
                tcl_runtime_api::ProcedureDispatch::Optimised,
            )
            .unwrap();
        assert_eq!(module.source, source);
        assert_eq!(module.source_namespace, namespace);
        assert_eq!(&module.top_level_body.lvt.entries()[..2], &parameters);
    }

    #[test]
    fn original_byte_command_plan_keeps_opaque_prefix_and_exact_bad_tail() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let service = BytecodeCompileService::for_profile(profile);
        let source = tcl_runtime_api::SourceImage::native(b"set \xff V; set \xfe {a}b".as_slice());
        let plan = service
            .script_command_plan_bytes_for_profile(&source, profile)
            .unwrap();
        assert_eq!(plan.complete_prefix_commands, 1);
        assert_eq!(&source.bytes()[..plan.complete_prefix_len], b"set \xff V; ");
        let tail = plan.fatal_tail.unwrap();
        assert_eq!(tail.message, "extra characters after close-brace");
        assert_eq!(tail.command_text, b"set \xfe {a}b");
        assert_eq!(tail.line, 1);
    }

    #[test]
    fn byte_command_plan_preserves_native_and_document_channel_rules() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let service = BytecodeCompileService::for_profile(profile);
        let text = "set a A\\\r\nset b B";
        let native = tcl_runtime_api::SourceImage::native(text.as_bytes());
        let document = tcl_runtime_api::SourceImage::document(text);
        assert_eq!(
            service
                .script_command_plan_bytes_for_profile(&native, profile)
                .unwrap()
                .complete_prefix_commands,
            2
        );
        assert_eq!(
            service
                .script_command_plan_bytes_for_profile(&document, profile)
                .unwrap()
                .complete_prefix_commands,
            1
        );
    }

    fn policy_entry(
        profile: &'static DialectProfile,
        point: tcl_dialect::model::DialectPoint,
    ) -> tcl_runtime_api::NativeCompilationEntry {
        use tcl_runtime_api::native_compilation::{
            NativeCompilationEntry, NativeCompilationFrame, NativeInterpreterIdentity,
        };
        NativeCompilationEntry {
            interpreter: NativeInterpreterIdentity {
                owner: 31,
                interpreter: 0,
            },
            epoch: 4,
            profile: profile.cache_key(),
            invocation_policy: Some(profile.cache_key()),
            execution_point: Some(point),
            name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(point),
            compiled_variable_protocol:
                tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point),
            compiled_local_layout: None,
            ensemble_target_objects: None,
            source_string_protocol: tcl_registry::InvocationDialect::of_profile(profile)
                .native_source_string_protocol(),
            lexer_grammar: None,
            inline_compilation_disabled: false,
            authored_tmm_static: None,
            namespace_variable_tables: None,
            variable_observers:
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
            math_functions: None,
            closed: false,
            commands: Vec::new(),
            namespaces: Vec::new(),
            current_namespace: 0,
            frame: NativeCompilationFrame::Unknown,
        }
    }

    #[test]
    fn native_entry_keeps_logical_engine_and_lexical_evidence_independent() {
        use tcl_dialect::model::{DialectPoint, Release};
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let actual = DialectPoint::canonical(Release::JIM_0_84);
        let mut entry = policy_entry(profile, actual);
        entry.lexer_grammar = Some(tcl_registry::InvocationDialect::of_point(actual).lexer_grammar);
        entry.invocation_policy = Some(
            tcl_registry::model::ingress::resolve_environment("jim")
                .analyser_profile()
                .cache_key(),
        );
        assert_eq!(
            BytecodeCompileService::native_entry_options(&entry, Some(profile))
                .invocation_dialect
                .expect("actual runtime")
                .core_point,
            Some(actual),
        );
        assert_eq!(
            BytecodeCompileService::native_entry_options(&entry, None)
                .invocation_dialect
                .expect("runtime without catalogue")
                .core_point,
            Some(actual),
        );
        let catalogue_config = LexerConfig::from_grammar(profile.grammar);
        assert_eq!(
            BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
            LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::of_point(actual).lexer_grammar
            ),
        );
        let mut grammar = entry.lexer_grammar.expect("measured grammar");
        grammar.brace_backslash_newline = tcl_dialect::BraceBackslashNewline::Folds;
        entry.lexer_grammar = Some(grammar);
        let selected = BytecodeCompileService::native_entry_options(&entry, Some(profile))
            .invocation_dialect
            .expect("independent actual axes");
        assert_eq!(selected.core_point, Some(actual));
        assert_eq!(selected.lexer_grammar, grammar);
        assert_eq!(
            selected.word_values,
            tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar)
        );
        assert_eq!(
            BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
            LexerConfig::from_grammar(grammar)
        );
        entry.lexer_grammar = None;
        assert_eq!(
            BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
            catalogue_config
        );
        entry.execution_point = None;
        entry.invocation_policy = Some(profile.cache_key());
        entry.lexer_grammar = None;
        assert_eq!(
            BytecodeCompileService::native_entry_config(catalogue_config, Some(&entry)),
            catalogue_config
        );
        assert_eq!(
            BytecodeCompileService::native_entry_options(&entry, Some(profile))
                .invocation_dialect
                .expect("explicit profile compatibility")
                .core_point,
            tcl_registry::InvocationDialect::of_profile(profile).core_point,
        );
        assert!(
            BytecodeCompileService::native_entry_options(&entry, Some(profile))
                .native_compiler_dialect()
                .is_none()
        );
        entry.execution_point = Some(actual);
        entry.invocation_policy = None;
        let unknown_policy = BytecodeCompileService::native_entry_options(&entry, Some(profile));
        assert!(unknown_policy.invocation_dialect.is_none());
        assert_eq!(
            unknown_policy.native_compiler_dialect().unwrap().core_point,
            Some(actual)
        );
    }

    #[test]
    fn physical_c9_compiler_cannot_replace_f5_logical_value_or_frame_policy() {
        use tcl_dialect::model::{DialectPoint, Release};
        use tcl_registry::InvocationDialect;
        let logical_profile = DialectProfile::find("f5-irules").unwrap();
        let assistance = DialectProfile::find("tcl9.1").unwrap();
        let physical_point = DialectPoint::canonical(Release::TCL_9_1);
        let mut entry = policy_entry(logical_profile, physical_point);
        let logical = InvocationDialect::of_profile(logical_profile);
        let physical = InvocationDialect::of_point(physical_point);
        assert_ne!(logical.numbers, physical.numbers);
        let options = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
        assert_eq!(options.invocation_dialect, Some(logical));
        assert_eq!(options.native_compiler_dialect(), Some(physical));
        assert_eq!(entry.profile, logical_profile.cache_key());

        // A host can independently select a logical compatibility policy.
        let compatibility = DialectProfile::find("tcl8.4").unwrap();
        entry.invocation_policy = Some(compatibility.cache_key());
        let compatible = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
        assert_eq!(
            compatible.invocation_dialect,
            Some(InvocationDialect::of_profile(compatibility))
        );
        assert_eq!(compatible.native_compiler_dialect(), Some(physical));
        assert_eq!(entry.profile, logical_profile.cache_key());
        let compatible_policy = compatible.invocation_dialect;

        entry.execution_point = None;
        let unknown_engine = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
        assert_eq!(unknown_engine.invocation_dialect, compatible_policy);
        assert!(unknown_engine.native_compiler_dialect().is_none());
        entry.execution_point = Some(physical_point);
        entry.invocation_policy = None;
        let unknown_policy = BytecodeCompileService::native_entry_options(&entry, Some(assistance));
        assert!(unknown_policy.invocation_dialect.is_none());
        assert_eq!(unknown_policy.native_compiler_dialect(), Some(physical));
    }

    #[test]
    fn direct_source_options_cannot_fill_missing_live_policy_from_authoring() {
        use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
        use tcl_dialect::model::{DialectPoint, Release};
        use tcl_registry::InvocationDialect;
        let logical = DialectProfile::find("f5-irules").unwrap();
        let physical = DialectPoint::canonical(Release::TCL_9_1);
        let mut entry = policy_entry(logical, physical);
        let config = LexerConfig {
            strict_quoting: true,
            base_line: 12,
            base_col: 4,
            leading_bom: tcl_lexer::LeadingBom::Skip,
            ..LexerConfig::from_grammar(DialectProfile::find("tcl8.6").unwrap().grammar)
        };
        let registry = CommandRegistry::build_default();
        let retained_policy = |entry: &tcl_runtime_api::NativeCompilationEntry| {
            let options = SourceAnalysisOptions {
                native_entry: Some(entry),
                invocation_dialect: Some(InvocationDialect::of_point(physical)),
                ..SourceAnalysisOptions::default()
            };
            assert_eq!(options.native_lexer_config(config), config);
            assert_eq!(
                options.native_compiler_dialect().unwrap().core_point,
                Some(physical)
            );
            SourceCommandBindings::analyse_with_options("set x 1", config, &registry, options)
                .invocation_at_source("set", 0)
                .variable_context
                .invocation_dialect
        };
        let policy = retained_policy(&entry).expect("retained logical policy");
        let expected = InvocationDialect::of_profile(logical);
        assert_eq!(policy.numbers, expected.numbers);
        assert_eq!(policy.characters, expected.characters);
        assert_eq!(policy.upvar_level_presence, expected.upvar_level_presence);
        assert_eq!(
            policy.lexer_grammar,
            config.grammar_over(expected.lexer_grammar)
        );
        entry.invocation_policy = None;
        assert!(retained_policy(&entry).is_none());
        let mut measured = config.grammar_over(expected.lexer_grammar);
        measured.var_syntax =
            InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84))
                .lexer_grammar
                .var_syntax;
        entry.lexer_grammar = Some(measured);
        let measured_options = SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(expected),
            ..SourceAnalysisOptions::default()
        };
        assert!(measured_options.logical_invocation_dialect().is_none());
        assert_eq!(
            measured_options.native_lexer_config(config),
            config.with_grammar(measured)
        );
        let positioned = LexerConfig {
            base_offset: 37,
            ..config
        };
        assert_eq!(
            BytecodeCompileService::native_entry_config(positioned, Some(&entry)),
            positioned.with_grammar(measured)
        );
    }

    #[test]
    fn runtime_entry_selects_actual_namespace_and_keeps_custom_shadow_generic() {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCompilationBinding, NativeCompilationEntry,
            NativeCompilationFrame, NativeCompilationNamespace, NativeInterpreterIdentity,
        };
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let namespace = tcl_core_types::ByteNamespacePath::from_segments(["N"]);
        let entry = NativeCompilationEntry {
            interpreter: NativeInterpreterIdentity {
                owner: 31,
                interpreter: 0,
            },
            epoch: 4,
            profile: profile.cache_key(),
            invocation_policy: Some(profile.cache_key()),
            execution_point: tcl_registry::InvocationDialect::of_profile(profile).core_point,
            name_protocol: tcl_registry::InvocationDialect::of_profile(profile)
                .core_point
                .and_then(tcl_syntax::naming::NamePolicyProtocol::for_native_point),
            compiled_variable_protocol: tcl_registry::InvocationDialect::of_profile(profile)
                .native_compiled_variable_protocol(),
            compiled_local_layout: None,
            ensemble_target_objects: None,
            source_string_protocol: tcl_registry::InvocationDialect::of_profile(profile)
                .native_source_string_protocol(),
            lexer_grammar: None,
            inline_compilation_disabled: false,
            authored_tmm_static: None,
            namespace_variable_tables: None,
            variable_observers:
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
            math_functions: None,
            closed: true,
            commands: vec![NativeCompilationBinding {
                slot: tcl_core_types::NativeByteCommandSlot::new(namespace.clone(), "set".into()),
                namespace_token: 1,
                token: 19,
                implementation_generation: 19,
                implementation: NativeCommandImplementation::Opaque,
                compiler_hook:
                    tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                has_execution_trace: false,
                compiler: None,
                procedure_header: None,
            }],
            namespaces: vec![NativeCompilationNamespace {
                path: namespace,
                jim_namespace_object: None,
                token: 1,
                visible: true,
                exports: Vec::new(),
                command_path: Vec::new(),
                unknown_handler: None,
            }],
            current_namespace: 1,
            frame: NativeCompilationFrame::Namespace,
        };
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let ir = lower_script_module_for_bytecode_with_options(
            "set x 1",
            "",
            registry,
            LexerConfig::from_grammar(profile.grammar),
            Some(profile),
            false,
            Some(BytecodeCompileService::native_entry_options(
                &entry,
                Some(profile),
            )),
        );
        assert_eq!(ir.top_level_namespace, "::N");
        assert_eq!(ir.source_entry.native_entry.as_deref(), Some(&entry));
        let service = BytecodeCompileService::for_profile(profile);
        let compiled = service
            .compile_script_with_entry(
                ScriptCompileTarget {
                    source: "set x 1",
                    namespace: "",
                },
                profile,
                &entry,
            )
            .unwrap();
        assert!(
            compiled
                .top_level
                .instructions
                .iter()
                .any(|instruction| matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                ))
        );
        assert_eq!(
            compiled.top_level.command_bindings,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );
    }

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
        assert_eq!(
            plain.top_level.command_bindings,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );
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
    fn runtime_targets_leave_unentered_procedure_bodies_in_original_source() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
            let service = BytecodeCompileService::for_profile(profile);
            let source = tcl_runtime_api::SourceImage::native(
                &b"proc child {} {\"}; set marker ENTERED"[..],
            );
            let namespace = tcl_runtime_api::ByteNamespacePath::root();
            for plain in [false, true] {
                let target = ScriptCompileTargetBytes {
                    source: &source,
                    namespace: &namespace,
                };
                let script = if plain {
                    service.compile_plain_script_bytes_for_profile(target, profile)
                } else {
                    service.compile_script_bytes_for_profile(target, profile)
                }
                .expect("unentered malformed child does not invalidate entered script");
                assert!(script.procedures.is_empty(), "{environment}");
                assert!(script.procedure_provenance.is_empty());
                assert_eq!(script.source, source, "{environment} original image");
                assert!(
                    script.top_level.instructions.iter().any(|instruction| {
                        instruction.source_cmd_text.bytes() == b"proc child {} {\"}"
                    }),
                    "{environment} original declaration invocation"
                );
                let procedure = service
                    .compile_procedure_bytes_for_profile(
                        ProcedureCompileTargetBytes {
                            source: &source,
                            parameters: &[],
                            namespace: &namespace,
                        },
                        profile,
                        if plain {
                            ProcedureDispatch::Plain
                        } else {
                            ProcedureDispatch::Optimised
                        },
                    )
                    .expect("entered procedure leaves nested body lazy");
                assert!(procedure.procedures.is_empty(), "{environment}");
                assert!(procedure.procedure_provenance.is_empty());
                assert_eq!(procedure.source, source);
            }
            let unicode = service
                .compile_script_for_profile(
                    ScriptCompileTarget {
                        source: "proc child {} {\"}; set marker ENTERED",
                        namespace: "",
                    },
                    profile,
                )
                .expect("Unicode target uses the same entered-source policy");
            assert!(unicode.procedures.is_empty());
        }
    }

    #[test]
    fn explicit_whole_module_artifact_keeps_procedure_inventory() {
        let source = "proc child {} {return KEPT}; set marker ENTERED";
        let module = BytecodeCompileService::default().compile(source).unwrap();
        assert!(module.procedures.contains_key("::child"));
        assert_eq!(module.procedure_provenance["::child"].body, "return KEPT");
    }

    #[test]
    fn entered_source_scope_excludes_deferred_bodies_before_cfg_preparation() {
        let source = "proc child {} {proc grand {} {puts NEVER}; return KEPT}; set marker ENTERED";
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        for scope in [
            ModuleEmissionScope::WholeModule,
            ModuleEmissionScope::EnteredSource,
        ] {
            let options = BytecodeCompileService::source_analysis_options(
                CompileSourcePolicy { entry: None, scope },
                registry,
                config,
                Some(profile),
            );
            let ir = lower_script_module_for_bytecode_with_options(
                source,
                "",
                registry,
                config,
                Some(profile),
                false,
                options,
            );
            assert_eq!(ir.source_entry.compilation_scope, scope);
            let entered = scope == ModuleEmissionScope::EnteredSource;
            assert_eq!(ir.procedures.is_empty(), entered);
            assert!(ir.top_level.statements.iter().any(|statement| {
                statement.tokens().is_some_and(|tokens| {
                    tokens.argv_texts.first().is_some_and(|head| head == "proc")
                        && tokens
                            .argv_texts
                            .last()
                            .is_some_and(|body| body.contains("puts NEVER"))
                })
            }));
            if entered {
                assert!(ir.future_call_sites.is_empty());
                assert!(ir.body_units.is_empty());
                assert!(ir.methods.is_empty());
            }
            let prepared = prepare_cfg_context_bundle(&ir, registry);
            let cfg = build_cfg_codegen_with_registry_and_context(
                &ir, false, registry, &prepared, config,
            );
            assert_eq!(cfg.procedures.is_empty(), entered);
        }
    }

    #[test]
    fn entered_source_defers_generic_namespace_children_before_cfg_preparation() {
        let source = "namespace eval ::outer {namespace eval ::inner {proc lazy {} {\"}; set child UNENTERED}}; set marker ENTERED";
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        for scope in [
            ModuleEmissionScope::WholeModule,
            ModuleEmissionScope::EnteredSource,
        ] {
            let options = BytecodeCompileService::source_analysis_options(
                CompileSourcePolicy { entry: None, scope },
                registry,
                config,
                Some(profile),
            );
            let ir = lower_script_module_for_bytecode_with_options(
                source,
                "",
                registry,
                config,
                Some(profile),
                false,
                options,
            );
            let entered = scope == ModuleEmissionScope::EnteredSource;
            assert_eq!(ir.procedures.is_empty(), entered);
            assert_eq!(ir.body_units.is_empty(), entered);
            let namespace_call = ir.top_level.statements.iter().find_map(|statement| {
                statement.tokens().filter(|tokens| {
                    tokens
                        .argv_texts
                        .first()
                        .is_some_and(|head| head == "namespace")
                })
            });
            let tokens = namespace_call.expect("the original namespace invocation is retained");
            assert_eq!(
                tokens.argv_texts[3],
                "namespace eval ::inner {proc lazy {} {\"}; set child UNENTERED}"
            );
            if entered {
                assert!(tokens.evaluated_body().is_none());
                assert!(ir.future_call_sites.is_empty());
                assert!(ir.methods.is_empty());
            }
            let prepared = prepare_cfg_context_bundle(&ir, registry);
            let cfg = build_cfg_codegen_with_registry_and_context(
                &ir, false, registry, &prepared, config,
            );
            assert_eq!(cfg.procedures.is_empty(), entered);
        }
    }

    #[test]
    fn entered_source_keeps_native_structured_body_preflight() {
        let source = tcl_lexer::SourceImage::native(b"if {1} {puts \"}".as_slice());
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
            let entry = crate::environment_ingress::captured_native_entry(profile);
            assert!(entry.closed, "{environment}: actual compilation entry");
            let selected = entry
                .lookup_command_bytes(entry.current_namespace, b"if")
                .expect("actual original command lookup")
                .expect("registered native if command");
            assert_eq!(
                selected.compiler_hook,
                tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present,
                "{environment}: actual if compiler hook",
            );
            assert!(
                selected.compiler.is_some(),
                "{environment}: selected compiler"
            );
            let script = BytecodeCompileService::for_profile(profile)
                .compile_script_bytes_with_entry(
                    ScriptCompileTargetBytes {
                        source: &source,
                        namespace: &tcl_runtime_api::ByteNamespacePath::root(),
                    },
                    profile,
                    &entry,
                )
                .expect("native syntax failure remains a presentable guest failure");
            let failure = script
                .top_level
                .native_compilation_failure
                .as_ref()
                .expect("the selected compiler visits the literal if body");
            assert_eq!(failure.message, "missing \"", "{environment}");
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
        assert_eq!(
            plain.top_level.command_bindings,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );

        let static_proc = service
            .compile("proc p {} {mutate; if {1} {return yes}}")
            .unwrap();
        assert_eq!(
            static_proc.procedure_provenance["::p"],
            tcl_bytecode::ProcedureProvenance {
                name: "::p".to_owned(),
                namespace_context: None,
                parameters: String::new(),
                body: "mutate; if {1} {return yes}".to_owned(),
            }
        );
    }

    #[test]
    fn same_name_projected_profiles_keep_cache_policies_and_artifact_handles() {
        use tcl_dialect::model::{DialectPoint, Release};
        let early = Box::leak(Box::new(DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            DialectPoint::canonical(Release::JIM_0_80),
        )));
        let late = Box::leak(Box::new(DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            DialectPoint::canonical(Release::JIM_0_84),
        )));
        for service in [
            BytecodeCompileService::default(),
            BytecodeCompileService::for_profile(early),
        ] {
            for profile in [early as &'static DialectProfile, late] {
                let view = service.registry.registry_for_profile(profile);
                assert_eq!(
                    view.as_ref().profile().unwrap().cache_key(),
                    profile.cache_key()
                );
                let fast = service.compile_for_profile("set x 1", profile).unwrap();
                let plain = service
                    .compile_plain_script_for_profile(
                        ScriptCompileTarget {
                            source: "set x 1",
                            namespace: "",
                        },
                        profile,
                    )
                    .unwrap();
                assert!(std::ptr::eq(fast.profile, profile));
                assert!(std::ptr::eq(plain.profile, profile));
                let parameters = Vec::new();
                let target = ProcedureCompileTarget {
                    source: "return 1",
                    namespace: "",
                    parameters: &parameters,
                };
                let procedure = service
                    .compile_procedure_for_profile(target, profile, ProcedureDispatch::Optimised)
                    .unwrap();
                assert!(std::ptr::eq(procedure.profile, profile));
            }
        }
    }

    fn instructions_without_source_spans(
        instructions: &[tcl_bytecode::Instruction],
    ) -> Vec<tcl_bytecode::Instruction> {
        // Direct procedure targets start at zero; enclosing modules use source
        // coordinates. All other codegen facts must remain equivalent.
        let mut instructions = instructions.to_vec();
        for instruction in &mut instructions {
            instruction.source_span = None;
            for operation in &mut instruction.native_operation_selections {
                operation.span = tcl_lexer::Span::new(0, operation.span.len());
            }
        }
        instructions
    }

    #[test]
    fn procedure_target_matches_static_proc_across_profiles_and_dispatches() {
        // Keep the cases together: each exercises module state a
        // procedure-target lowering must assemble the same way a static
        // `proc` body does (procedure frame/LVT, namespace resolution, nested and
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
                    let static_instructions =
                        instructions_without_source_spans(&static_proc.instructions);
                    let direct_instructions =
                        instructions_without_source_spans(&direct_proc.instructions);
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
                        direct_module.procedures.is_empty(),
                        "{environment:?} runtime target compiled an unentered nested body"
                    );
                    assert!(direct_module.procedure_provenance.is_empty());
                } else {
                    assert!(direct_proc.plain_command_dispatch);
                    assert_eq!(
                        direct_proc.command_bindings,
                        [] as [tcl_runtime_api::CommandBindingIdentity; 0]
                    );
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
        assert!(direct_module.procedures.is_empty());
        assert!(direct_module.procedure_provenance.is_empty());

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
        let namespace_proofs: Vec<_> = static_ir
            .procedures
            .get(":::::p")
            .into_iter()
            .flat_map(|procedure| &procedure.body.statements)
            .filter_map(|statement| statement.tokens())
            .map(|tokens| {
                let binding = tokens.source_binding.as_ref();
                (
                    &tokens.argv_texts,
                    binding.map(|binding| &binding.lookup_namespace),
                    binding.and_then(|binding| binding.proved_handler_target()),
                    binding.and_then(|binding| binding.proved_execution_target()),
                    binding.map(|binding| &binding.evaluated_argument_values),
                    crate::registry_invocation::namespace_directive_footprint(
                        registry.as_ref(),
                        None,
                        tokens,
                    ),
                )
            })
            .collect();
        assert_eq!(
            static_ir.namespace_exports,
            direct_ir.namespace_exports,
            "literal-colon procedures: {:?}; body units: {:?}; namespace proofs: {namespace_proofs:?}",
            static_ir.procedures.keys().collect::<Vec<_>>(),
            static_ir.body_units.keys().collect::<Vec<_>>()
        );
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
        assert_eq!(
            plain.top_level.command_bindings,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );
        assert!(
            plain
                .top_level
                .instructions
                .iter()
                .any(|instruction| instruction.op == tcl_bytecode::Op::INVOKE_STK1)
        );
    }

    #[test]
    fn retained_string_selection_captures_a_private_name_without_a_public_token() {
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let service = BytecodeCompileService::for_profile(profile);
        let module = service
            .compile("string equal -nocase a A")
            .expect("known native ensemble");
        assert!(module.top_level.literals.entries().iter().any(|literal| {
            literal
                .unicode()
                .is_ok_and(|text| text.strip_prefix("::").unwrap_or(text) == "tcl::string::equal")
        }));
        assert!(module.top_level.instructions.iter().any(|instruction| {
            instruction.op == tcl_bytecode::Op::INVOKE_REPLACE
                && instruction.operands
                    == [tcl_bytecode::Operand::Imm(5), tcl_bytecode::Operand::Imm(2)]
        }));
        assert!(
            module
                .top_level
                .instructions
                .iter()
                .all(|instruction| instruction.entered_command.is_none())
        );
    }

    #[test]
    fn escaped_heads_retain_decoded_bindings_at_the_native_release_boundary() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile =
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
            let service = BytecodeCompileService::for_profile(profile);
            let direct = service.compile_for_profile("set x OLD", profile).unwrap();
            assert!(
                direct
                    .top_level
                    .command_bindings
                    .iter()
                    .any(|binding| { binding.name == "set" && binding.identity == "set" }),
                "{dialect} direct binding"
            );
            let escaped = service
                .compile_for_profile(r"se\x74 x OLD", profile)
                .unwrap();
            if matches!(dialect, "tcl8.4" | "tcl8.5") {
                assert!(escaped.top_level.command_bindings.is_empty(), "{dialect}");
                assert!(
                    escaped.top_level.instructions.iter().any(|instruction| {
                        matches!(
                            instruction.op,
                            tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                        )
                    }),
                    "{dialect}"
                );
            } else {
                assert!(
                    escaped
                        .top_level
                        .command_bindings
                        .iter()
                        .any(|binding| { binding.name == "set" && binding.identity == "set" }),
                    "{dialect}: {:?}",
                    escaped.top_level.command_bindings
                );
                assert!(
                    escaped
                        .top_level
                        .command_bindings
                        .iter()
                        .all(|binding| binding.name != r"se\x74")
                );
            }
        }
    }

    #[test]
    fn lowering_fallback_keeps_the_native_compiler_selection() {
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
            fallback
                .top_level
                .command_bindings
                .iter()
                .any(|binding| binding.name == "return"),
            "a backend surrogate must retain the native compiled return token: {:?}",
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

    #[test]
    fn runtime_script_command_plan_uses_the_requested_release_grammar() {
        let tcl84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
        let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let service = BytecodeCompileService::for_profile(tcl90);
        let source = "set side 1; {*}{set x 2}";

        let old = service
            .script_command_plan_for_profile(source, tcl84)
            .unwrap();
        assert_eq!(&source[..old.complete_prefix_len], "set side 1; ");
        assert_eq!(
            old.fatal_tail
                .expect("8.4 rejects expansion syntax")
                .message,
            "extra characters after close-brace"
        );

        let current = service
            .script_command_plan_for_profile(source, tcl90)
            .unwrap();
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
    fn native_operation_metadata_is_specialisation_scoped() {
        let service = BytecodeCompileService::default();
        let eligible = service.compile("set result [llength [pid]]").unwrap();
        let generic = service.compile("set result [pid]").unwrap();
        let wrong_arity = service.compile("set result [llength [pid] extra]").unwrap();
        let traced = service
            .compile_traced("set result [llength [pid]]")
            .unwrap();

        let selected_lengths = |module: &tcl_bytecode::ModuleAsm| {
            module
                .top_level
                .instructions
                .iter()
                .flat_map(|instruction| &instruction.native_operation_selections)
                .filter(|operation| {
                    operation
                        .requirements
                        .iter()
                        .any(|binding| binding.name == "llength" && binding.identity == "llength")
                })
                .count()
        };
        assert_eq!(selected_lengths(&eligible), 1);
        assert_eq!(selected_lengths(&generic), 0);
        assert_eq!(selected_lengths(&wrong_arity), 0);
        assert_eq!(selected_lengths(&traced), 0);
    }

    #[test]
    fn every_applicable_typed_codegen_hook_retains_operation_selection() {
        let cases = [
            ("lassign", "lassign {a b} a b"),
            // Keep const-foldable hooks dynamic: this test exercises the
            // entered-token codegen path, while fold provenance is covered by
            // the constant-substitution tests.
            ("llength", "llength $value"),
            ("lset", "lset value 0 x"),
            ("dict", "dict set value key x"),
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
            let operation = module.procedures["::p"]
                .instructions
                .iter()
                .flat_map(|instruction| &instruction.native_operation_selections)
                .find(|operation| {
                    operation
                        .requirements
                        .iter()
                        .any(|binding| binding.name == name && binding.identity == name)
                });
            let operation = operation.unwrap_or_else(|| {
                panic!(
                    "{name}: proved inline hook retains its operation range; requirements={:?}",
                    module.procedures["::p"].command_bindings
                )
            });
            assert_eq!(operation.source, command);
            assert!(module.procedures["::p"].labels.contains_key(&operation.end));
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
        let source = "proc continue {} {return -code break}\n\
                      if {[incr i] > 3} {set seen 1}\ncontinue";
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
        assert_eq!(edge.source_cmd_text, "");
        assert!(module.top_level.instructions.iter().any(|instruction| {
            instruction.source_cmd_text == "continue"
                && matches!(
                    instruction.op,
                    tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
                )
        }));
    }

    #[test]
    fn generic_eval_keeps_its_body_for_runtime_compilation() {
        let module = BytecodeCompileService::default()
            .compile("eval {while {0} {}}")
            .unwrap();
        assert_eq!(
            module.top_level.command_bindings,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );
        assert!(module.top_level.instructions.iter().any(|instruction| {
            matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            ) && instruction.source_cmd_text == "eval {while {0} {}}"
        }));
    }

    #[test]
    fn foreach_line_binding_dependency_follows_the_resolved_release_profile() {
        let tcl90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let tcl86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let source = "foreachLine line file.txt {set seen $line}";

        let current = BytecodeCompileService::for_profile(tcl90)
            .compile(source)
            .unwrap();
        assert_eq!(
            current.top_level.command_bindings,
            [] as [tcl_runtime_api::CommandBindingIdentity; 0]
        );
        assert!(std::ptr::eq(current.profile, tcl90));
        assert!(current.top_level.instructions.iter().any(|instruction| {
            matches!(
                instruction.op,
                tcl_bytecode::Op::INVOKE_STK1 | tcl_bytecode::Op::INVOKE_STK4
            ) && instruction.source_cmd_text == source
        }));

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
            .map(|instruction| {
                instruction
                    .source_cmd_text
                    .try_text()
                    .expect("Unicode fixture source")
            })
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
            .compile("pid; if {1} {set ::body_ran 1}")
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
        let body = "pid; if {1} {set ::body_ran 1}";
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
