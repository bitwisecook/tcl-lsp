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

//! Compiling scripts and substitutions through the compile service: the script cache, plain-dispatch modules, module profile and namespace validation, and service errors.

use super::super::commands::CommandSidecarKey;
use super::super::{NamespacePath, NativeCachedModule, Vm};
use crate::compiled::CompiledUnit;
use crate::error::TclError;
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, ModuleAsm};
use tcl_runtime_api::{ArtefactIdentityManifest, Rung, ScriptCompileTarget};

impl Vm {
    pub(crate) fn prepare_original_substitution(
        &mut self,
        original: &Value,
        flags: tcl_runtime_api::native_substitution::NativeSubstitutionFlags,
    ) -> Result<Option<CompiledUnit>, TclError> {
        use tcl_runtime_api::native_substitution::NativeSelectedSubstitutionHandler;
        let Some(version) = self
            .actual_native_invocation_dialect()
            .execution_point()
            .and_then(tcl_dialect::model::DialectPoint::tcl_version)
        else {
            return Ok(None);
        };
        if !matches!(
            version,
            tcl_dialect::TclVersion::V8_6
                | tcl_dialect::TclVersion::V9_0
                | tcl_dialect::TclVersion::V9_1
        ) {
            return Ok(None);
        }
        let unavailable = Self::unavailable_original_substitution_handler;
        let origin = self
            .native_invocation
            .sidecar
            .as_ref()
            .map(|key| self.ultimate_import_origin(key))
            .ok_or_else(unavailable)?;
        let CommandSidecarKey::Visible(key) = origin else {
            return Err(unavailable());
        };
        let selected = self
            .visible_command_at_key(&key)
            .and_then(|command| {
                self.native_compilation_binding(&key, &command, self.native_interpreter_identity())
            })
            .ok_or_else(unavailable)?;
        let current = self.current_native_compilation_namespace()?;
        let namespace = self.ns_path(current);
        let snapshot = self.native_compilation_entry_for_namespace_token(
            self.current_native_compilation_namespace()?,
            false,
        );
        let selected = NativeSelectedSubstitutionHandler::capture(&snapshot, &selected)
            .map_err(|_| unavailable())?;
        if let Some(cache) = original.native_substitution_cache() {
            let unit = &cache.unit;
            if cache.version == version
                && cache.context == crate::value::NativeBytecodeContext::Substitution(flags)
                && cache.substitution_layout == snapshot.compiled_local_layout
                && cache.substitution_table.as_ref().map(Rc::as_ptr)
                    == self
                        .frames
                        .last()
                        .and_then(|frame| frame.native_local_names.as_ref())
                        .map(Rc::as_ptr)
                && unit.interpreter == self.native_interpreter_identity()
                && unit.compiler.is_current_service(self.compiler_generation)
                && unit.profile_generation == self.profile_generation
                && unit.native_cache == Some(self.native_cache_stamp(self.current_ns_id()))
                && unit.source_namespace.compiled_path() == Some(&namespace)
                && self.function_command_bindings_match_with_manifest(
                    &unit.asm,
                    unit.manifest.as_deref(),
                )
                && self.compiled_local_layout_matches(&unit.asm)
            {
                return Ok(Some(unit.clone()));
            }
        }
        self.compile_original_substitution(original, flags, version, selected)
            .map(Some)
    }

    fn unavailable_original_substitution_handler() -> TclError {
        TclError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original substitution handler registration",
        ))
    }

    fn compile_original_substitution(
        &mut self,
        original: &Value,
        flags: tcl_runtime_api::native_substitution::NativeSubstitutionFlags,
        version: tcl_dialect::TclVersion,
        selected: tcl_runtime_api::native_substitution::NativeSelectedSubstitutionHandler,
    ) -> Result<CompiledUnit, TclError> {
        use tcl_runtime_api::native_substitution::{
            NativeSubstitutionCompilationEntry, NativeSubstitutionTarget,
        };

        let unavailable = Self::unavailable_original_substitution_handler;
        original.retire_native_substitution_cache();
        // Source updater callbacks happen before the actual compilation snapshot.
        let source = self.native_script_source_image(original)?;
        let current = self.current_native_compilation_namespace()?;
        let namespace = self.ns_path(current);
        let snapshot = self.native_compilation_entry_for_namespace_token(
            self.current_native_compilation_namespace()?,
            false,
        );
        let entry =
            NativeSubstitutionCompilationEntry::from_selected_handler(&snapshot, selected, flags)
                .map_err(|error| {
                self.compile_service_error_bytes(
                    error,
                    &source,
                    &namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                )
            })?;
        let compiler = self.compiler.clone().ok_or_else(|| {
            self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "substitution compilation requires a CompileService".into(),
                ),
                &source,
                &namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            )
        })?;
        let module = compiler
            .compile_substitution_with_entry(
                NativeSubstitutionTarget {
                    source: &source,
                    namespace: &namespace,
                },
                self.source_profile(),
                entry,
            )
            .map_err(|error| {
                self.compile_service_error_bytes(
                    error,
                    &source,
                    &namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                )
            })?;
        if module.source != source
            || module.source_namespace != namespace
            || module.profile.cache_key() != self.source_profile().cache_key()
            || module
                .top_level
                .validate_native_compilation_entry()
                .is_err()
            || !module.procedures.is_empty()
        {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "CompileService returned another substitution target".into(),
                ),
                &source,
                &namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        }
        let strings = snapshot.source_string_protocol.ok_or_else(unavailable)?;
        let unit = self
            .compiled_unit(Rc::new(module.top_level), namespace)
            .finalize_original_source_pool(original, strings)?;
        let table = self
            .frames
            .last()
            .and_then(|frame| frame.native_local_names.clone());
        original.install_native_substitution_cache(
            version,
            flags,
            snapshot.compiled_local_layout,
            table,
            unit.clone(),
        )?;
        Ok(unit)
    }
}

impl Vm {
    /// Compile through the explicit plain-dispatch capability and verify the
    /// returned artifact actually carries no specialised binding assumptions.
    /// A missing or dishonest capability fails closed.
    pub(crate) fn compile_plain_module(
        &mut self,
        src: &str,
        namespace: &str,
    ) -> Result<ModuleAsm, TclError> {
        self.compile_plain_module_as(src, namespace, tcl_runtime_api::CommandEvaluation::Compiled)
    }

    /// [`Self::compile_plain_module`], the script's own commands evaluated as
    /// `evaluation` says ([`tcl_runtime_api::CommandEvaluation`]).
    pub(crate) fn compile_plain_module_as(
        &mut self,
        src: &str,
        namespace: &str,
        evaluation: tcl_runtime_api::CommandEvaluation,
    ) -> Result<ModuleAsm, TclError> {
        let Some(compiler) = self.compiler.as_ref() else {
            return Err(self.compile_service_error(
                tcl_runtime_api::CompileError::Unsupported(
                    "command mutation recovery requires a plain-dispatch CompileService".into(),
                ),
                src,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        };
        let module = compiler
            .compile_plain_script_for_profile(
                ScriptCompileTarget {
                    source: src,
                    namespace,
                    evaluation,
                },
                self.source_profile(),
            )
            .map_err(|error| {
                self.compile_service_error(
                    error,
                    src,
                    namespace,
                    tcl_runtime_api::NativeCompilationAdmissionScope::Script,
                )
            })?;
        self.validate_module_profile(&module)?;
        Self::validate_module_namespace(&module, namespace)?;
        if !module.plain_command_dispatch
            || !module.top_level.plain_command_dispatch
            || !module.top_level.command_bindings.is_empty()
            || !module.top_level.procedure_bindings.is_empty()
            || module.procedures.values().any(|asm| {
                !asm.plain_command_dispatch
                    || !asm.command_bindings.is_empty()
                    || !asm.procedure_bindings.is_empty()
            })
        {
            return Err(TclError::new(
                "CompileService plain-dispatch capability returned optimised bytecode",
            ));
        }
        Ok(module)
    }

    pub(in super::super) fn compile_plain_cached_module(
        &mut self,
        target: ScriptCompileTarget<'_>,
    ) -> Result<Rc<ModuleAsm>, TclError> {
        let key = (
            Self::legacy_source_namespace(target.namespace),
            tcl_lexer::SourceImage::document(target.source),
        );
        let entry = std::sync::Arc::new(
            self.native_compilation_entry_for_namespace(target.namespace, false)?,
        );
        if let Some(cached) = self.eval_cache_plain.get(&key)
            && cached.entry.same_compilation_world(&entry)
        {
            Self::validate_module_namespace(&cached.module, target.namespace)?;
            return Ok(Rc::clone(&cached.module));
        }
        let module = Rc::new(self.compile_plain_module(target.source, target.namespace)?);
        self.eval_cache_plain.insert(
            key,
            NativeCachedModule {
                module: Rc::clone(&module),
                entry,
            },
        );
        Ok(module)
    }

    pub(in super::super) fn selected_source_identity(
        &self,
    ) -> Result<&ArtefactIdentityManifest, TclError> {
        let Some(active) = self.active_native_profile else {
            return Ok(self.pin.identity());
        };
        if std::ptr::eq(active, self.pin.profile) {
            return Ok(self.pin.identity());
        }
        let unavailable = || {
            TclError::from(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original native core source context",
            ))
        };
        let engine = self.actual_engine_profile.ok_or_else(unavailable)?;
        let core = self.native_core_pin.as_ref().ok_or_else(unavailable)?;
        if !std::ptr::eq(active, engine) || !std::ptr::eq(core.profile, engine) {
            return Err(unavailable());
        }
        Ok(core.identity())
    }

    /// Enforce the bytecode artifact's exact profile at every consumption
    /// boundary.  The permissive fallback profile is still a real compile
    /// target: bytecode lowered with its all-Tcl registry can contain an
    /// intrinsic unavailable in a named release, so it is only executable by
    /// a fallback-profile VM.
    pub(crate) fn validate_module_profile(&self, module: &ModuleAsm) -> Result<(), TclError> {
        if !std::ptr::eq(module.profile, self.source_profile()) {
            return Err(TclError::new(format!(
                "bytecode compiled for dialect profile {} cannot run under {}",
                module.profile.name,
                self.source_profile().name
            )));
        }
        // What the whole unit rests on — the ABI and the world it was lexed
        // and specialised for — is checked here, once, and refuses it outright.
        // The fields that only some rungs rest on are checked per function, in
        // `function_command_bindings_match`.
        let Some(manifest) = &module.manifest else {
            return Ok(());
        };
        let held = self.selected_source_identity()?;
        match manifest
            .disagreements(held)
            .into_iter()
            .find(|field| field.rests_on().contains(Rung::Generic))
        {
            None => Ok(()),
            Some(field) => Err(TclError::new(format!(
                "bytecode manifest disagrees with the runtime on {}: compiled for {}, runtime holds {}",
                field.name(),
                manifest.describe(field),
                held.describe(field)
            ))),
        }
    }

    /// Enforce the command-resolution namespace captured by the compiler.
    /// Namespace is an executable specialisation axis just like dialect and
    /// command bindings: an unqualified command in identical source can name
    /// a different implementation in another namespace.
    pub(crate) fn validate_module_namespace(
        module: &ModuleAsm,
        expected: &str,
    ) -> Result<(), TclError> {
        Self::validate_module_namespace_bytes(module, &Self::legacy_source_namespace(expected))
    }

    pub(crate) fn validate_module_namespace_bytes(
        module: &ModuleAsm,
        expected: &NamespacePath,
    ) -> Result<(), TclError> {
        if &module.source_namespace == expected {
            Ok(())
        } else {
            Err(TclError::new(
                "bytecode compilation namespace does not match the exact execution path",
            ))
        }
    }

    /// Adapt an already constructed legacy Unicode key. Native callers retain
    /// namespace segments directly and never use this compatibility projection.
    pub(super) fn legacy_source_namespace(namespace: &str) -> NamespacePath {
        NamespacePath::from_segments(tcl_syntax::naming::key_segments(namespace))
    }

    /// Compile `src` via the injected [`CompileService`], from the cache when
    /// possible — the shared body of [`eval_source`](Self::eval_source) and
    /// [`compile_script_cached`](Self::compile_script_cached). Picks
    /// [`CompileService::compile_plain_dispatch_for_profile`] and
    /// [`Self::eval_cache_plain`] instead of the fast pair whenever
    /// [`Self::requires_plain_command_dispatch`] is true,
    /// so a step-traced proc's `if`/`while`/`foreach`/`eval`/`uplevel`/
    /// `catch`/`try`/… bodies — every one of which funnels through this
    /// method via its runtime builtin — compile trace-visible too,
    /// without each call site needing to know that.
    pub(in super::super) fn compile_cached(
        &mut self,
        src: &str,
    ) -> Result<Rc<ModuleAsm>, TclError> {
        self.current_native_compilation_namespace()?;
        let namespace = self.current_ns().to_owned();
        self.compile_cached_in_namespace(src, &namespace)
    }

    /// Namespace-explicit core of [`Self::compile_cached`]. Most dynamic Tcl
    /// scripts inherit the active frame; synthetic wrappers such as a
    /// coroutine's already-resolved initial command execute at the global
    /// frame and enter here with that explicit provenance instead.
    pub(in super::super) fn compile_cached_in_namespace(
        &mut self,
        src: &str,
        namespace: &str,
    ) -> Result<Rc<ModuleAsm>, TclError> {
        let plain = self.requires_plain_command_dispatch();
        let target = ScriptCompileTarget {
            source: src,
            namespace,
            evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
        };
        if plain {
            return self.compile_plain_cached_module(target);
        }
        let key = (
            Self::legacy_source_namespace(namespace),
            tcl_lexer::SourceImage::document(src),
        );
        let entry =
            std::sync::Arc::new(self.native_compilation_entry_for_namespace(namespace, false)?);
        if let Some(cached) = self.eval_cache.get(&key)
            && cached.entry.same_compilation_world(&entry)
        {
            let m = Rc::clone(&cached.module);
            Self::validate_module_namespace(&m, namespace)?;
            if !self
                .function_command_bindings_match_with_manifest(&m.top_level, m.manifest.as_deref())
            {
                return self.compile_plain_cached_module(target);
            }
            return Ok(m);
        }
        let Some(c) = self.compiler.as_ref() else {
            return Err(self.compile_service_error(
                tcl_runtime_api::CompileError::Unsupported(
                    "eval / command substitution requires a CompileService".into(),
                ),
                src,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        };
        let compiled = c.compile_script_with_entry(
            ScriptCompileTarget {
                source: src,
                namespace,
                evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
            },
            self.source_profile(),
            &entry,
        );
        let m = Rc::new(compiled.map_err(|error| {
            self.compile_service_error(
                error,
                src,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            )
        })?);
        self.validate_module_profile(&m)?;
        Self::validate_module_namespace(&m, namespace)?;
        if !self.function_command_bindings_match_with_manifest(&m.top_level, m.manifest.as_deref())
        {
            return self.compile_plain_cached_module(target);
        }
        self.eval_cache.insert(
            key,
            NativeCachedModule {
                module: Rc::clone(&m),
                entry,
            },
        );
        Ok(m)
    }

    pub(crate) fn compile_plain_function_cached_bytes(
        &mut self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
    ) -> Result<Rc<FunctionAsm>, TclError> {
        let module = self.compile_module_bytes(target.source, target.namespace, true)?;
        self.merge_procs(&module);
        Ok(Rc::new(module.top_level.clone()))
    }

    pub(crate) fn compile_plain_module_bytes(
        &mut self,
        source: &tcl_lexer::SourceImage,
        namespace: &NamespacePath,
    ) -> Result<ModuleAsm, TclError> {
        self.compile_module_bytes(source, namespace, true)
            .map(|module| (*module).clone())
    }

    fn validate_script_module_source(
        &mut self,
        module: &ModuleAsm,
        source: &tcl_lexer::SourceImage,
        namespace: &NamespacePath,
    ) -> Result<(), TclError> {
        self.validate_module_profile(module)?;
        Self::validate_module_namespace_bytes(module, namespace)?;
        if &module.source != source {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "byte CompileService returned another source image or input channel".into(),
                ),
                source,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        }
        Ok(())
    }

    pub(in super::super) fn compile_module_bytes(
        &mut self,
        source: &tcl_lexer::SourceImage,
        namespace: &NamespacePath,
        plain: bool,
    ) -> Result<Rc<ModuleAsm>, TclError> {
        self.compile_module_bytes_as(source, namespace, plain, false)
    }

    /// [`Self::compile_module_bytes`] for a script the release evaluates
    /// `direct`ly (`TCL_EVAL_DIRECT`), whose commands are invoked uncompiled.
    pub(in super::super) fn compile_module_bytes_as(
        &mut self,
        source: &tcl_lexer::SourceImage,
        namespace: &NamespacePath,
        plain: bool,
        direct: bool,
    ) -> Result<Rc<ModuleAsm>, TclError> {
        let current = self
            .current_namespace_execution_context()
            .resolved_token()
            .filter(|token| self.native_namespace_token_has_owner(*token))
            .filter(|current| namespace == &self.ns_path(*current))
            .or_else(|| self.namespace_token_at_path(namespace, true));
        let current = current
            .filter(|token| self.native_namespace_token_has_owner(*token))
            .ok_or_else(Self::unavailable_native_compilation_namespace)?;
        let mut entry = self.native_compilation_entry_for_namespace_token(current, false);
        entry.direct_evaluation = direct;
        let entry = std::sync::Arc::new(entry);
        let key = (namespace.clone(), source.clone());
        let cached = if plain {
            self.eval_cache_plain.get(&key)
        } else {
            self.eval_cache.get(&key)
        };
        if let Some(cached) = cached
            && cached.entry.same_compilation_world(&entry)
        {
            Self::validate_module_namespace_bytes(&cached.module, namespace)?;
            let module = cached.module.clone();
            if plain
                || self.function_command_bindings_match_with_manifest(
                    &module.top_level,
                    module.manifest.as_deref(),
                )
            {
                return Ok(module);
            }
            return self.compile_module_bytes_as(source, namespace, true, direct);
        }
        let Some(compiler) = self.compiler.clone() else {
            return Err(self.compile_service_error_bytes(
                tcl_runtime_api::CompileError::Unsupported(
                    "eval / command substitution requires a byte CompileService".into(),
                ),
                source,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            ));
        };
        let target = tcl_runtime_api::ScriptCompileTargetBytes { source, namespace };
        let result = if plain {
            compiler.compile_plain_script_bytes_with_entry(target, self.source_profile(), &entry)
        } else {
            compiler.compile_script_bytes_with_entry(target, self.source_profile(), &entry)
        };
        let module = Rc::new(result.map_err(|error| {
            self.compile_service_error_bytes(
                error,
                source,
                namespace,
                tcl_runtime_api::NativeCompilationAdmissionScope::Script,
            )
        })?);
        self.validate_script_module_source(&module, source, namespace)?;
        if plain {
            if !module.plain_command_dispatch
                || !module.top_level.plain_command_dispatch
                || !module.top_level.command_bindings.is_empty()
                || !module.top_level.procedure_bindings.is_empty()
                || module.procedures.values().any(|asm| {
                    !asm.plain_command_dispatch
                        || !asm.command_bindings.is_empty()
                        || !asm.procedure_bindings.is_empty()
                })
            {
                return Err(self.compile_service_error_bytes(tcl_runtime_api::CompileError::Unsupported(
                    "byte CompileService plain capability returned specialized command assumptions".into()), source,
                    namespace, tcl_runtime_api::NativeCompilationAdmissionScope::Script));
            }
        } else if !self.function_command_bindings_match_with_manifest(
            &module.top_level,
            module.manifest.as_deref(),
        ) {
            return self.compile_module_bytes_as(source, namespace, true, direct);
        }
        let cache = if plain {
            &mut self.eval_cache_plain
        } else {
            &mut self.eval_cache
        };
        cache.insert(
            key,
            NativeCachedModule {
                module: module.clone(),
                entry,
            },
        );
        Ok(module)
    }

    /// Compile the original byte image in the actual current namespace.
    pub(crate) fn compile_script_cached_bytes(
        &mut self,
        source: &tcl_lexer::SourceImage,
    ) -> Result<CompiledUnit, TclError> {
        let namespace = self.source_namespace_path();
        self.compile_script_cached_bytes_in_namespace(source, &namespace)
    }

    pub(crate) fn compile_script_cached_bytes_in_namespace(
        &mut self,
        source: &tcl_lexer::SourceImage,
        namespace: &NamespacePath,
    ) -> Result<CompiledUnit, TclError> {
        self.reset_error_state_for_eval();
        let module =
            self.compile_module_bytes(source, namespace, self.requires_plain_command_dispatch())?;
        self.merge_procs(&module);
        Ok(self.compiled_unit(
            Rc::new(module.top_level.clone()),
            module.source_namespace.clone(),
        ))
    }

    /// Keep a compiler provider obligation outside the guest error channel.
    /// The empty Tcl error only unwinds internal callers; the engine-owned
    /// refusal takes priority before catch stores or subsequent guest effects.
    pub(crate) fn compile_service_error(
        &mut self,
        error: tcl_runtime_api::CompileError,
        source: &str,
        namespace: &str,
        scope: tcl_runtime_api::NativeCompilationAdmissionScope,
    ) -> TclError {
        self.compile_service_error_bytes(
            error,
            &tcl_lexer::SourceImage::document(source),
            &Self::legacy_source_namespace(namespace),
            scope,
        )
    }

    pub(crate) fn compile_service_error_bytes(
        &mut self,
        error: tcl_runtime_api::CompileError,
        source: &tcl_lexer::SourceImage,
        compilation_namespace: &NamespacePath,
        scope: tcl_runtime_api::NativeCompilationAdmissionScope,
    ) -> TclError {
        match error {
            tcl_runtime_api::CompileError::Message(message) => TclError::new(message),
            tcl_runtime_api::CompileError::NativeCompilationAdmission(error) => {
                let failure = self.retain_native_execution_refusal(error.into());
                TclError::from_execution_failure(failure)
            }
            tcl_runtime_api::CompileError::Unsupported(reason) => {
                if self.execution_refusal.is_none() {
                    self.retain_native_execution_refusal(
                        tcl_runtime_api::NativeExecutionError::CompileServiceRefusal(Box::new(
                            tcl_runtime_api::NativeCompileServiceRefusal {
                                reason,
                                source: source.clone(),
                                compilation_namespace: compilation_namespace.clone(),
                                scope,
                                source_profile: self.expression_source_profile().cache_key(),
                                native_profile: self.native_execution_profile().cache_key(),
                                interpreter:
                                    tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                                        owner: self.owner_nonce,
                                        interpreter: u64::try_from(self.cur.0)
                                            .expect("interpreter slot fits u64"),
                                    },
                                frame: self.current_level(),
                                namespace: self.ns_path(self.current_ns_id()),
                                namespace_token: u64::from(self.current_ns_id().0),
                            },
                        )),
                    );
                }
                TclError::from_execution_failure(
                    self.execution_refusal
                        .clone()
                        .expect("compile refusal retained"),
                )
            }
        }
    }
}
