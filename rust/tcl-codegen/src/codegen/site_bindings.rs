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

//! The command bindings, pack claims and native-compilation dependencies a specialised site relies on.

use super::{
    CodegenCtx, EmissionNamespace, NativeOperationOrigin, SiteBinding,
    expression_script_dependency, namespace_path_for_binding, namespace_path_from_constructed_key,
};

impl<'r> CodegenCtx<'r> {
    /// Set the rooted constructed command-resolution namespace for direct
    /// codegen specialisations in this function.
    /// Set an exact native namespace without reparsing a written name.
    pub(crate) fn set_resolution_namespace_path(
        &mut self,
        namespace: tcl_runtime_api::ByteNamespacePath,
    ) {
        self.resolution_namespace = EmissionNamespace::native(namespace);
        self.current_command_namespace = self.resolution_namespace.path.clone();
        self.current_command_namespace_context = self.resolution_namespace_context();
    }

    pub(super) fn resolution_namespace_context(
        &self,
    ) -> Option<tcl_runtime_api::CompiledNamespaceContext> {
        let context = tcl_compiler::command_binding::SourceNamespaceKey::from_native_entry(
            self.native_entry?,
        )
        .ok()?
        .to_compiled_context()?;
        (context.path() == &self.resolution_namespace.path).then_some(context)
    }

    /// Checked constructed analysis key for this exact native namespace.
    pub(crate) fn resolution_namespace(&self) -> Option<&str> {
        self.resolution_namespace.key.as_deref()
    }

    /// Construct the complete source-site identity for a direct codegen
    /// specialisation.
    pub(crate) fn command_binding_identity(
        &self,
        name: impl Into<String>,
        identity: impl Into<String>,
    ) -> tcl_runtime_api::CommandBindingIdentity {
        let name = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| {
                tcl_compiler::registry_invocation::static_command_word(
                    tokens,
                    self.escapes,
                    self.word_rules,
                )
            })
            .unwrap_or_else(|| name.into());
        let identity = identity.into();
        let namespace_context = self
            .invocation_tokens
            .as_deref()
            .and_then(tcl_compiler::registry_invocation::compiled_namespace_context);
        let namespace = self.resolution_namespace().unwrap_or_else(|| {
            assert!(
                namespace_context.is_some(),
                "native specialisation requires an exact namespace context"
            );
            ""
        });
        let binding = tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            namespace,
            name,
            tcl_syntax::naming::unroot_rooted_key(&identity).unwrap_or(&identity),
        )
        .with_namespace_context(namespace_context);
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens
                .source_binding
                .as_ref()
                .and_then(|proof| proof.admitted_inline_invocation())
                .is_some_and(|proof| {
                    tcl_registry::command_descriptor_labels_match(
                        &proof.target.command,
                        &binding.identity,
                    )
                })
        {
            return binding.with_guard(tcl_compiler::registry_invocation::command_binding_guard(
                tokens,
            ));
        }
        binding
    }

    #[track_caller]
    pub(super) fn refuse_native_dependency(&mut self) {
        if tcl_dialect::trace_env::var("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC").as_deref()
            == Ok("first-decline-v1")
        {
            eprintln!(
                "CCG_NATIVE_DEPENDENCY_REFUSAL caller={} source_len={} span={:?}",
                std::panic::Location::caller(),
                self.source.len(),
                self.current_span.map(|span| (span.start(), span.end())),
            );
        }
        if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
            eprintln!(
                "NATIVE_CODEGEN_REFUSAL caller={} compilation={:?} invocation={:?} span={:?} source={:?}",
                std::panic::Location::caller(),
                self.native_compilation,
                self.invocation_dialect,
                self.current_span,
                self.exact_command_source
                    .as_ref()
                    .map(tcl_lexer::SourceImage::bytes),
            );
        }
        self.native_dependency_refusal = true;
    }

    /// Scope exact source proof to one emitter call; nested emitters restore it.
    pub(super) fn with_invocation_tokens<T>(
        &mut self,
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
        emit: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous =
            std::mem::replace(&mut self.invocation_tokens, tokens.cloned().map(Box::new));
        let previous_lexical_words = self.original_lexical_words.get_mut().take();
        let previous_layout = self.native_hook_layout.take();
        let operation = if self.plain_command_dispatch {
            None
        } else if let Some(tokens) = self
            .invocation_tokens
            .as_deref()
            .filter(|tokens| tokens.synthetic.is_none())
        {
            if let Ok(plan) = tcl_compiler::registry_invocation::native_operation_selection_plan(
                tokens,
                self.escapes,
                self.word_rules,
            ) {
                plan
            } else {
                self.refuse_native_dependency();
                None
            }
        } else {
            None
        };
        let operation_start = self.instructions.len();
        if !self.plain_command_dispatch
            && let Some(tokens) = self.invocation_tokens.as_deref()
        {
            // A direct evaluation parses and dispatches each command only when
            // it is reached, so one the walk proves never entered needs no
            // compiler admission.
            let direct_unentered =
                |binding: &tcl_compiler::command_binding::SourceInvocationBinding| {
                    self.native_compilation.mode
                        == tcl_registry::native_compilation::NativeCompilationMode::Direct
                        && binding.runtime_reachability()
                            == tcl_compiler::command_binding::SourceRuntimeReachability::NotEntered
                };
            let refused = tokens.source_binding.as_ref().is_some_and(|binding| {
                matches!(
                    binding.native_compilation_admission_selection(),
                    tcl_registry::native_compilation::NativeCompilationSelection::Unknown
                ) && binding.original_named_compiler_admission(tokens).is_none()
                    && !direct_unentered(binding)
            });
            // Auxiliary compiler lookups are guard dependencies, not source
            // commands. Recording one must not switch the enclosing command's
            // replay namespace to a private implementation's namespace.
            self.command_binding_requirements.extend(
                tcl_compiler::registry_invocation::native_implementation_dependencies(tokens),
            );
            if refused {
                if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
                    eprintln!(
                        "NATIVE_CODEGEN_ADMISSION argv={:?} selection={:?} original_named={}",
                        tokens.argv_texts,
                        tokens
                            .source_binding
                            .as_ref()
                            .map(tcl_compiler::command_binding::SourceInvocationBinding::native_compilation_admission_selection),
                        tokens.source_binding.as_ref().is_some_and(|binding| binding
                            .original_named_compiler_admission(tokens)
                            .is_some()),
                    );
                }
                self.refuse_native_dependency();
            }
        }
        let result = emit(self);
        if let Some(operation) = operation {
            self.retain_native_operation_selection(operation_start, operation);
        }
        self.invocation_tokens = previous;
        *self.original_lexical_words.get_mut() = previous_lexical_words;
        self.native_hook_layout = previous_layout;
        result
    }

    /// `required` as the site holds it: a pack command the site specialised
    /// through its `alias_of` is an alias at run time, so the name is held to
    /// the target the site's alias claim records and never to the pack's own
    /// registration.
    fn requirement_through_alias_claim(
        &self,
        required: &tcl_runtime_api::CommandBindingIdentity,
    ) -> tcl_runtime_api::CommandBindingIdentity {
        self.site_claim_requirements
            .iter()
            .find_map(|claim| match claim {
                tcl_runtime_api::SiteClaim::BuiltinAlias { binding, .. }
                    if binding.name == required.name
                        && binding.resolution_namespace == required.resolution_namespace
                        && binding.namespace_context == required.namespace_context
                        && binding.guard == required.guard =>
                {
                    Some(binding.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| required.clone())
    }

    /// Retain selection at the first operation instruction, before its argv.
    /// Nested operations can share that instruction but have distinct ranges.
    fn retain_native_operation_selection(
        &mut self,
        start: usize,
        plan: tcl_compiler::registry_invocation::NativeOperationSelectionPlan,
    ) {
        if start == self.instructions.len() {
            return;
        }
        // The function inventory summarizes every retained operation premise,
        // including constant-result paths that never call a typed opcode hook.
        let requirements: Vec<_> = plan
            .requirements
            .iter()
            .map(|required| self.requirement_through_alias_claim(required))
            .collect();
        self.command_binding_requirements
            .extend(requirements.iter().cloned());
        let end = self.fresh_label("native_operation_end");
        self.place_label(&end);
        let previous = self.native_operation_origins.insert(
            NativeOperationOrigin {
                start,
                source: plan.compilation_site,
                operation: plan.operation,
                guard: plan.guard,
                compiler_prerequisite: plan.compiler_prerequisite.clone(),
                requirements: requirements.clone(),
            },
            end.clone(),
        );
        let site = tcl_bytecode::NativeOperationSelectionSite {
            compiler_prerequisite: plan.compiler_prerequisite,
            requirements,
            guard: plan.guard,
            end,
            source: tcl_lexer::SourceImage::from_bytes(
                plan.source.into_bytes(),
                self.source.channel(),
            ),
            span: plan.span,
            namespace: plan.namespace_context.as_ref().map_or_else(
                || namespace_path_from_constructed_key(&plan.namespace),
                |context| context.path().clone(),
            ),
            namespace_context: plan.namespace_context,
        };
        let first = &mut self.instructions[start];
        // Reentrant bridges can scope the same invocation twice. Keep the
        // enclosing range rather than validating its shorter duplicate.
        if let Some(previous) = previous {
            first
                .native_operation_selections
                .retain(|existing| existing.end != previous);
        }
        first.native_operation_selections.insert(0, site);
        first.no_fold = true;
    }

    /// Native engine snapshot for a reached hook, independent of its catalogue.
    pub(super) fn native_hook_dialect(&self) -> Option<tcl_native_model::InvocationDialect> {
        if let Some(binding) = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
        {
            return binding.native_compiler_dialect();
        }
        self.invocation_dialect.or_else(|| {
            Some(
                tcl_compiler::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    self.dialect,
                    tcl_lexer::LexerConfig::for_profile(self.dialect),
                ),
            )
        })
    }

    /// Select backend metadata under the actual reached native dialect. A
    /// permissive or assisting catalogue cannot replace this engine snapshot.
    pub(super) fn invocation_surface_query(&self) -> Option<tcl_dialect::model::SurfaceQuery<'r>> {
        match self.native_hook_dialect() {
            Some(dialect) => dialect.authoring_query(),
            None => self.registry.own_surface_query(),
        }
    }

    /// A retained uncertain execution cannot acquire a hook from its spelling.
    pub(super) fn invocation_specialisation_proved(&self) -> bool {
        (self.resolution_namespace().is_some()
            || self
                .invocation_tokens
                .as_deref()
                .and_then(tcl_compiler::registry_invocation::compiled_namespace_context)
                .is_some())
            && self.invocation_tokens.as_deref().is_none_or(|tokens| {
                tokens.source_binding.as_ref().is_none_or(|binding| {
                    tcl_compiler::registry_invocation::proved_native_admitted_inline_operation(
                        tokens,
                    )
                    .is_some()
                        && binding.native_inline_rejection
                            == tcl_compiler::command_binding::NativeInlineRejection::None
                })
            })
    }

    /// Whether `name` is free of whole-unit mutation, for transforms that have
    /// no independently replayable command boundary.
    ///
    /// Answers from the whole-module [`Self::command_bindings`] summary — a
    /// flow-**insensitive** scan on purpose: a `rename` buried in a proc body
    /// can fire before a call earlier in the file runs, so "no rename seen so
    /// far" is not a sound answer. Without a module view the answer is
    /// `true`; see [`Self::command_bindings`].
    #[must_use]
    pub fn trusts_builtin(&self, name: &str) -> bool {
        self.command_bindings.is_none_or(|m| m.trusts(name))
    }

    /// Record one source binding relied on by specialised emission.
    pub fn require_command_binding(&mut self, binding: &tcl_runtime_api::CommandBindingIdentity) {
        self.require_invocation_binding(
            binding,
            &tcl_registry::release_compilation::InvocationShape::default(),
        );
    }

    /// [`Self::require_command_binding`] for an invocation of `shape`.
    pub(crate) fn require_invocation_binding(
        &mut self,
        binding: &tcl_runtime_api::CommandBindingIdentity,
        shape: &tcl_registry::release_compilation::InvocationShape,
    ) {
        self.hold_release_binding(&binding.name, shape);
        self.command_binding_requirements.insert(binding.clone());
        self.current_command_namespace = namespace_path_for_binding(binding);
        self.current_command_namespace_context
            .clone_from(&binding.namespace_context);
    }

    /// Unit cache validation can retain only the compiler world present at
    /// this artifact's original entry. Later child worlds keep their own
    /// instruction selection and validation boundary.
    pub(super) fn retain_entry_named_compiler_prerequisite(
        &mut self,
        required: &std::sync::Arc<
            tcl_runtime_api::native_compilation::NativeEnsembleCompilerPrerequisite,
        >,
    ) {
        let Some(entry) = self.native_entry else {
            return;
        };
        if entry.interpreter != required.interpreter {
            return;
        }
        if required.matches_registration_with(|namespace, word| {
            entry.lookup_command_bytes(namespace, word.as_bytes()).map(
                Option::<&tcl_runtime_api::native_compilation::NativeCompilationBinding>::cloned,
            )
        }) != Ok(true)
        {
            return;
        }
        // Cache validity is checked at this same immutable entry; the
        // instruction retains its independent before-arguments selection.
        let mut entry_required = required.as_ref().clone();
        entry_required.guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
        self.retain_native_compilation_dependency(
            tcl_compiler::registry_invocation::NativeCompilationDependency::Compiler(
                tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
                    std::sync::Arc::new(entry_required),
                ),
            ),
        );
    }

    pub(super) fn retain_native_compilation_dependency(
        &mut self,
        dependency: tcl_compiler::registry_invocation::NativeCompilationDependency,
    ) {
        match dependency {
            tcl_compiler::registry_invocation::NativeCompilationDependency::Compiler(required) => {
                if !self.native_compiler_prerequisites.contains(&required) {
                    self.native_compiler_prerequisites.push(required);
                }
            }
            tcl_compiler::registry_invocation::NativeCompilationDependency::Implementation(
                binding,
            ) => {
                self.command_binding_requirements.insert(binding);
            }
        }
    }

    /// Atomically retain the exact dependencies consumed by successful folds.
    /// Implicit lookup guards must not overwrite the enclosing replay namespace.
    pub(crate) fn retain_math_invocations(
        &mut self,
        proofs: &[tcl_compiler::command_binding::SourceMathInvocation],
    ) -> bool {
        use tcl_compiler::math_function_binding::{
            NativeMathFoldDependency, native_fold_dependency,
        };
        let Some(dependencies) = proofs
            .iter()
            .map(native_fold_dependency)
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        let mut table = self.math_table_prerequisite.as_ref();
        for dependency in &dependencies {
            if let NativeMathFoldDependency::Fixed(required) = dependency {
                if table.is_some_and(|existing| existing != required) {
                    return false;
                }
                table = Some(required);
            }
        }
        for dependency in dependencies {
            match dependency {
                NativeMathFoldDependency::Command(binding) => {
                    self.command_binding_requirements.insert(binding);
                }
                NativeMathFoldDependency::Fixed(required) => {
                    self.math_table_prerequisite = Some(required);
                }
            }
        }
        true
    }

    /// Retain actual preparation owners even when no function call was reached.
    pub(crate) fn retain_expression_preparations(
        &mut self,
        proofs: &[tcl_compiler::command_binding::SourceExpressionPreparation],
    ) -> bool {
        if proofs.is_empty() {
            return true;
        }
        let Some(mut context) = self.fold_policy().preparation_context() else {
            return false;
        };
        context.lexer_grammar = self.lexer_config().grammar_over(context.lexer_grammar);
        let mut table = self.math_table_prerequisite.as_ref();
        for proof in proofs {
            if proof.witness.context() != &context
                || proof.witness.source().as_bytes() != proof.source.text.bytes()
                || !proof.has_closed_script_compilation()
            {
                return false;
            }
            if let Some(required) = proof.witness.fixed_functions() {
                if table.is_some_and(|existing| existing != required) {
                    return false;
                }
                table = Some(required);
            }
        }
        let Some(dependencies) = proofs
            .iter()
            .flat_map(tcl_compiler::command_binding::SourceExpressionPreparation::script_compilation_dependencies)
            .map(expression_script_dependency)
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        if let Some(required) = table {
            self.math_table_prerequisite = Some(required.clone());
        }
        for dependency in dependencies {
            self.retain_native_compilation_dependency(dependency);
        }
        true
    }

    /// [`Self::require_command_binding`] for the command whose head `site`
    /// says typed lowering consumed, wherever emission stands.
    pub(crate) fn require_command_site(&mut self, site: &tcl_compiler::ir::CommandBindingSite) {
        self.hold_release_binding_at(site.span, &site.binding.name, &site.shape);
        if let Some(binding) =
            tcl_compiler::registry_invocation::native_site_binding_requirement(site)
        {
            self.command_binding_requirements.insert(binding.clone());
        }
        self.current_command_namespace = namespace_path_for_binding(&site.binding);
        self.current_command_namespace_context
            .clone_from(&site.binding.namespace_context);
    }

    /// Record one spec-pack claim a specialised operation rests on.
    pub fn require_site_claim(&mut self, claim: &tcl_runtime_api::SiteClaim) {
        self.site_claim_requirements.insert(claim.clone());
    }

    /// Record a site binding of an invocation of `shape`: the binding, and its
    /// claim when it has one.
    pub(crate) fn require_site_binding(
        &mut self,
        site: &SiteBinding,
        shape: &tcl_registry::release_compilation::InvocationShape,
    ) {
        self.require_invocation_binding(&site.binding, shape);
        if let Some(claim) = &site.claim {
            self.require_site_claim(claim);
        }
    }

    /// The binding a call of `cmd` specialised on `stamp` relies on: the
    /// identity the registry answers for it
    /// ([`ResolvedCall::stamp_identity`](tcl_registry::registry::ResolvedCall::stamp_identity)),
    /// and the rung-2 claim when that identity reached a builtin through a
    /// pack command's `alias_of`.
    pub(crate) fn stamped_binding(
        &self,
        cmd: &str,
        resolved: &tcl_registry::registry::ResolvedCall<'_>,
        stamp: tcl_registry::codegen_stamp::CodegenStamp,
    ) -> SiteBinding {
        let binding =
            self.command_binding_identity(cmd, resolved.stamp_identity(self.registry, stamp));
        let claim =
            tcl_compiler::site_claims::builtin_alias_claim(self.registry, resolved, &binding);
        SiteBinding { binding, claim }
    }

    /// Resolve a registry-described lowering specialisation for an inline
    /// emitter which operates below the normal IR lowering boundary.
    ///
    /// The returned identity is the dependency the caller must retain if it
    /// consumes the command head. Keeping the proof and its identity together
    /// prevents ad-hoc nested-body emitters from specialising on a raw command
    /// name without participating in runtime command-table revalidation.
    pub(crate) fn inline_lowering_hook(
        &self,
        command: &str,
        args: &[&str],
    ) -> Option<(
        tcl_registry::hooks::LoweringHookId,
        tcl_runtime_api::CommandBindingIdentity,
    )> {
        if self.plain_command_dispatch
            || !self.invocation_specialisation_proved()
            || !self.trusts_builtin(command)
        {
            return None;
        }
        if let Some(tokens) = self.invocation_tokens.as_deref()
            && tokens.source_binding.is_some()
        {
            if !tokens.words_align_with_argv_text()
                || tokens.argv_texts.first().is_none_or(|head| head != command)
                || tokens.argv_texts.len() != args.len().checked_add(1)?
                || tokens
                    .argv_texts
                    .iter()
                    .skip(1)
                    .zip(args)
                    .any(|(original, operand)| original.as_str() != *operand)
            {
                return None;
            }
            let admitted = tcl_compiler::registry_invocation::admitted_native_compiler_invocation(
                self.registry,
                None,
                tokens,
            )?;
            return Some((
                admitted.lowering_hook()?,
                self.command_binding_identity(command, admitted.canonical_registration_name()),
            ));
        }
        let resolved =
            self.registry
                .resolve_call(command, args, self.invocation_surface_query())?;
        if resolved.spec.name != command {
            return None;
        }
        Some((
            resolved.lowering_hook?,
            self.command_binding_identity(command, resolved.spec.name),
        ))
    }
}
