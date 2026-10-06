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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compiler traversal precedes execution, including unreachable inline bodies.

mod original_preparation;

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, NativeCompilationSnapshot,
    SourceCommandBindings, SourceExecutionContext,
};
use crate::ir::{CommandTokens, WordExpr, WordPart};
use tcl_registry::native_compilation::{
    NativeCompilationFailureScope, NativeCompilationMode, NativeCompilationSelection,
    NativeCompilationStep, NativeCompilationSteps, NativeExpressionCompilerStep,
    NativeMathFunctionResolution,
};

/// A rejection of one compilation chunk before any of its script effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeCompilationFailure {
    /// Actual source instance and beginning of the rejected chunk.
    pub chunk: CommandAllocationSite,
    /// Native invocation whose compilation rejected that chunk.
    pub invocation: CommandAllocationSite,
    /// Registry-owned completion presentation; unresolved error information
    /// remains explicit rather than being replaced by the message.
    pub failure: tcl_registry::native_compilation::NativeCompilationFailure,
    /// Original compiler bindings reached through the rejected prefix, including
    /// the invocation and enclosing compiler selections which reach its error.
    pub dependencies: Vec<SourceNativeCompilationDependency>,
    /// Actual native fixed-function table required by this compiler traversal.
    pub math_table_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    /// Original inner-to-outer compilation contexts, without runtime procedure names.
    pub contexts: Vec<SourceNativeCompilationContext>,
}

/// One immutable native binding required by a selected compile-time failure.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeCompilationDependency {
    /// Raw compiler registration, independent of the current runtime handler.
    pub compiler_prerequisite:
        Option<Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>>,
    /// Exact native command object and implementation generation.
    pub target: super::SourceCommandTarget,
    /// Actual command lookup namespace at compilation.
    pub namespace: String,
    pub(crate) namespace_key: super::SourceNamespaceKey,
    /// Effective literal command head selected by the compiler.
    pub head: String,
    /// Table-validation boundary, separate from later runtime dispatch.
    pub guard: tcl_registry::native_compilation::NativeCompilationGuard,
}

/// Source presentation for one native compilation context.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeCompilationContext {
    /// Invocation in its actual authored or materialised source instance.
    pub invocation: CommandAllocationSite,
    /// Full written command, including original quoting and delimiters.
    pub command: Option<String>,
    /// One-based native source line measured from the rejected chunk's beginning.
    pub line_in_chunk: Option<u32>,
    /// Proved compiler annotations preceding this command context.
    pub before_context: Vec<String>,
    /// Native compiler annotations emitted after this command's context frame.
    pub after_context: Vec<String>,
}

impl SourceCommandBindings {
    /// Compiler failures in reached compilation boundaries, including procedure bodies.
    #[must_use]
    pub fn native_compilation_failures(&self) -> &[SourceNativeCompilationFailure] {
        &self.compilation_failures
    }

    /// Select a rejected authored chunk at its actual compilation boundary.
    #[must_use]
    pub fn native_compilation_failure_at(
        &self,
        chunk_offset: u32,
    ) -> Option<&SourceNativeCompilationFailure> {
        self.compilation_boundaries
            .get(&CommandAllocationSite {
                source: Arc::clone(self.root_origin.as_ref()?),
                offset: chunk_offset,
            })?
            .as_ref()
    }

    /// Whether a native compiler must resolve a remaining possible entry error.
    /// This obligation never asserts that compilation definitely fails.
    #[must_use]
    pub fn native_compilation_provider_required_at(&self, chunk_offset: u32) -> bool {
        self.root_origin.as_ref().is_some_and(|origin| {
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: chunk_offset,
            };
            let covered = self.compiled_children.get(&site).is_some_and(|children| {
                    children.iter().any(|child| {
                        child.enclosing.source == site.source
                            && child.enclosing.offset != site.offset
                            && child.compilation.mode == NativeCompilationMode::BytecodeObject
                            && matches!(child.parent.as_ref(), super::compiled_invocation::SourceNativeCompilerAdmission::Inline(parent)
                                if parent.compilation_site.source == site.source)
                            && self.compilation_sources.get(&site).and_then(Option::as_ref)
                                == Some(&child.script)
                    })
                });
            self.compilation_provider_required.contains(&site) && !covered
        })
    }

    /// Exact bytes and source identity presented at this compilation entry.
    #[must_use]
    pub fn native_compilation_source_at(
        &self,
        chunk_offset: u32,
    ) -> Option<&super::ExecutedScriptSource> {
        self.compilation_sources
            .get(&CommandAllocationSite {
                source: Arc::clone(self.root_origin.as_ref()?),
                offset: chunk_offset,
            })?
            .as_ref()
    }

    pub(super) fn record_compilation_source(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &ModuleCommandBindings,
    ) {
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let original = origin.source_image().bytes();
        let carrier = (original.get(base as usize..(base as usize).saturating_add(image.len()))
            == Some(image.bytes()))
        .then(|| super::ExecutedScriptSource {
            text: image.clone(),
            origin: Arc::clone(origin),
            mapping: super::ExecutedScriptMapping::Contiguous { base },
        });
        self.compilation_sources
            .entry(CommandAllocationSite {
                source: Arc::clone(origin),
                offset: base,
            })
            .and_modify(|previous| {
                if *previous != carrier {
                    *previous = None;
                }
            })
            .or_insert(carrier);
    }

    pub(super) fn record_compilation_boundary(
        &mut self,
        base: u32,
        state: &ModuleCommandBindings,
        failure: Option<&SourceNativeCompilationFailure>,
    ) {
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: base,
        };
        self.compilation_boundaries
            .entry(site)
            .and_modify(|proved| {
                if proved.as_ref() != failure {
                    *proved = None;
                }
            })
            .or_insert_with(|| failure.cloned());
    }
}

/// A literal child compiled by its enclosing native compiler traversal.
/// Its immutable entry is independent of prospective runtime recompilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SourceCompiledChild {
    pub enclosing: CommandAllocationSite,
    pub table: Arc<super::SourceLookupSnapshot>,
    pub parent: Arc<super::compiled_invocation::SourceNativeCompilerAdmission>,
    pub script: super::ExecutedScriptSource,
    pub compilation: tcl_registry::native_compilation::NativeCompilationContext,
    pub parent_namespace_key: super::SourceNamespaceKey,
    pub namespace_key: super::SourceNamespaceKey,
    pub compiler_visits: Arc<super::compiler_inventory::SourceCompilerVisits>,
}

#[derive(Default)]
pub(super) struct PreflightOutcome {
    pub failure: Option<SourceNativeCompilationFailure>,
    /// Missing host evidence requires admission without inventing a guest error.
    pub native_entry_unavailable: bool,
    pub possible_error: bool,
    pub compiled_children:
        std::collections::BTreeMap<CommandAllocationSite, Vec<SourceCompiledChild>>,
    pub compiler_invocations: std::collections::BTreeMap<
        CommandAllocationSite,
        Vec<super::compiler_inventory::SourceCompilerInvocation>,
    >,
}

pub(super) fn preflight(
    source: &str,
    base: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    snapshot: &mut NativeCompilationSnapshot,
) -> PreflightOutcome {
    let channel = state
        .current_source_origin
        .as_ref()
        .map_or(tcl_lexer::SourceChannel::Document, |origin| {
            origin.source_image().channel()
        });
    let image = tcl_lexer::SourceImage::from_bytes(source.as_bytes(), channel);
    preflight_image(&image, base, state, context, snapshot)
}

pub(super) fn preflight_image(
    image: &tcl_lexer::SourceImage,
    base: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    snapshot: &mut NativeCompilationSnapshot,
) -> PreflightOutcome {
    if context.compilation.mode != NativeCompilationMode::BytecodeObject {
        return PreflightOutcome::default();
    }
    let Some(origin) = &state.current_source_origin else {
        return PreflightOutcome {
            possible_error: true,
            ..PreflightOutcome::default()
        };
    };
    let chunk = CommandAllocationSite {
        source: Arc::clone(origin),
        offset: base,
    };
    let mut scanner = CompilerTraversal {
        state,
        snapshot,
        chunk,
        possible_error: false,
        compiler_dependencies: Vec::new(),
        math_table_prerequisite: None,
        before_next_context: Vec::new(),
        compiler_invocations: std::collections::BTreeMap::new(),
        compiled_children: std::collections::BTreeMap::new(),
    };
    let mut failure = scanner.script(image, base, context);
    if let Some(failure) = &mut failure {
        for dependency in &scanner.compiler_dependencies {
            if !failure.dependencies.contains(dependency) {
                failure.dependencies.push(dependency.clone());
            }
        }
    }
    if let Some(failure) = &mut failure
        && let Some(required) = &scanner.math_table_prerequisite
    {
        if failure
            .math_table_prerequisite
            .as_ref()
            .is_some_and(|own| own != required)
        {
            scanner.possible_error = true;
        } else {
            failure.math_table_prerequisite = Some(required.clone());
        }
    }
    let native_entry_unavailable = state.baseline.native_entry.is_none();
    if (scanner.possible_error || native_entry_unavailable)
        && let Some(failure) = &mut failure
    {
        // An unsupported visit or missing entry leaves presentation unproved.
        // Retain the rejection dependency without inventing its guest result.
        failure.failure.message = None;
        failure.failure.error_code = None;
        failure.failure.error_info = None;
        failure.contexts.clear();
    }
    if !scanner.compiled_children.is_empty() {
        let visits = Arc::new(scanner.compiler_invocations.clone());
        for children in scanner.compiled_children.values_mut() {
            for child in children {
                child.compiler_visits = Arc::clone(&visits);
            }
        }
    }
    PreflightOutcome {
        failure,
        native_entry_unavailable,
        possible_error: scanner.possible_error,
        compiler_invocations: scanner.compiler_invocations,
        compiled_children: scanner.compiled_children,
    }
}

pub(super) fn materialise_error_storage(
    state: &mut ModuleCommandBindings,
    registry: &tcl_registry::CommandRegistry,
) {
    let Some(storage) = state
        .baseline
        .dialect
        .and_then(tcl_registry::special_vars::native_error_storage)
    else {
        state.mark_opaque_binding_mutation();
        return;
    };
    let observed = storage.iter().any(|variable| {
        crate::var_resolve::resolve_literal_access(
            variable.global_name(),
            &state.source_variables,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        )
        .observed
    });
    if observed {
        // Error presentation invokes observers before a surrounding catch
        // resumes; their command mutations are part of this abrupt edge.
        state.mark_opaque_binding_mutation();
    } else {
        let variables = Arc::make_mut(&mut state.source_variables);
        for variable in storage {
            variables.define_unknown_contents(variable.global_name(), registry);
        }
    }
}

struct CompilerTraversal<'a> {
    state: &'a ModuleCommandBindings,
    snapshot: &'a mut NativeCompilationSnapshot,
    chunk: CommandAllocationSite,
    possible_error: bool,
    compiler_dependencies: Vec<SourceNativeCompilationDependency>,
    math_table_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    compiled_children: std::collections::BTreeMap<CommandAllocationSite, Vec<SourceCompiledChild>>,
    before_next_context: Vec<String>,
    compiler_invocations: std::collections::BTreeMap<
        CommandAllocationSite,
        Vec<super::compiler_inventory::SourceCompilerInvocation>,
    >,
}

impl CompilerTraversal<'_> {
    fn require_provider(&mut self) {
        self.possible_error = true;
    }

    fn script(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let Ok(source) = image.try_text() else {
            self.require_provider();
            return None;
        };
        if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(context.depth) {
            self.require_provider();
            return None;
        }
        let Ok(plan) = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.len()).ok()?),
            context.config,
        ) else {
            self.require_provider();
            return None;
        };
        // The checked native plan owns the syntax cut. The structural view may
        // retain the failed command, but it must not select its incomplete argv.
        let mut config = context.config;
        config.strict_quoting = false;
        let Some(segments) =
            crate::segmenter::segment_commands_image_with_offset_and_config(image, base, config)
        else {
            self.require_provider();
            return None;
        };
        let map = tcl_lexer::SourceMap::from_image(image).with_base(base, 0, 0);
        for (index, segment) in segments.into_iter().enumerate() {
            if let Some(tail) = plan.fatal_tail.as_ref()
                && index == tail.cut.command
            {
                return self.script_parse_failure(image, base, tail);
            }
            let tokens = CommandTokens::from_segmented(&map, context.config, &segment);
            let failure = self.command(tokens.words(), segment.span.start(), context);
            self.record_compiler_invocation(tokens.words(), segment.span.start(), context);
            if let Some(mut failure) = failure {
                let mut presentation = self.presentation(source, base, &segment);
                presentation.before_context = std::mem::take(&mut self.before_next_context);
                failure.contexts.push(presentation);
                return Some(failure);
            }
        }
        None
    }

    fn script_parse_failure(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        tail: &tcl_lexer::NativeScriptWordCut,
    ) -> Option<SourceNativeCompilationFailure> {
        let Some(storage) = self
            .state
            .baseline
            .compilation_dialect()
            .and_then(tcl_registry::InvocationDialect::native_bytecode_storage_protocol)
        else {
            self.require_provider();
            return None;
        };
        // Modern compilers retain a runtime Syntax instruction. C84 instead
        // rejects this compilation before any command in its chunk executes.
        if storage.recipe().retains_parse_failure() {
            return None;
        }
        let offset = base.checked_add(tail.command_start)?;
        let command = tcl_syntax::native_parse_context::c84_compilation_command_extent(
            image.bytes(),
            tail.command_start as usize,
            tail.cut.term as usize,
        )
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned);
        let original = self.chunk.source.source_image();
        let lines = tcl_lexer::LineIndex::from_bytes(original.bytes());
        let mut failure = self.failure_at(
            offset,
            tcl_registry::native_compilation::NativeCompilationFailure {
                message: Some(tail.cut.message.to_owned()),
                error_code: Some("NONE".to_owned()),
                error_info: None,
            },
            Vec::new(),
        );
        failure.contexts.push(SourceNativeCompilationContext {
            invocation: failure.invocation.clone(),
            command,
            before_context: std::mem::take(&mut self.before_next_context),
            after_context: Vec::new(),
            line_in_chunk: lines
                .line_at(offset)
                .checked_sub(lines.line_at(self.chunk.offset))
                .and_then(|line| line.checked_add(1)),
        });
        Some(failure)
    }

    #[inline(never)]
    fn record_compiler_invocation(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) {
        let selected = super::compiled_invocation::select_invocation_boxed(
            words,
            self.state,
            &SourceExecutionContext {
                compilation_snapshot: Some(self.snapshot),
                ..context
            },
            offset,
        );
        let invocation = super::compiler_inventory::SourceCompilerInvocation {
            selection: selected.admission,
            admitted: selected.admitted.clone(),
            operand_layout: selected.operand_layout.clone(),
            namespace_bindings: selected.namespace_bindings.clone(),
            switch: selected.switch.clone(),
            structured: selected.structured.clone(),
            original_words: selected.original_words.clone(),
            policy: selected.policy.clone(),
            table: Arc::clone(&self.snapshot.table),
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            head: super::source_effective_words(words, self.state.baseline.dialect, None)
                .first()
                .and_then(|word| word.as_registry_word().literal())
                .map(str::to_owned),
        };
        if let Some(recipe) = &invocation.structured {
            self.retain_prefix_dependency(recipe.dependency().clone());
        }
        if let Some(recipe) = &invocation.namespace_bindings {
            self.retain_prefix_dependency(recipe.dependency().clone());
        }
        if let Some(recipe) = &invocation.switch {
            self.retain_prefix_dependency(recipe.dependency.clone());
        }
        if let Some(admitted) = &invocation.admitted {
            let dependencies = match admitted.as_ref() {
                super::compiled_invocation::SourceNativeCompilerAdmission::Inline(proof) => {
                    &proof.lookup_dependencies
                }
                super::compiled_invocation::SourceNativeCompilerAdmission::Named(proof) => {
                    &proof.dependencies
                }
            };
            for dependency in dependencies {
                self.retain_prefix_dependency(dependency.clone());
            }
        }
        let site = CommandAllocationSite {
            source: Arc::clone(&self.chunk.source),
            offset,
        };
        let retained = self.compiler_invocations.entry(site).or_default();
        if !retained.contains(&invocation) {
            retained.push(invocation);
        }
    }

    fn retain_prefix_dependency(&mut self, mut dependency: SourceNativeCompilationDependency) {
        dependency.guard = tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry;
        if let Some(required) = &mut dependency.compiler_prerequisite {
            Arc::make_mut(required).guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
        }
        if !self.compiler_dependencies.contains(&dependency) {
            self.compiler_dependencies.push(dependency);
        }
    }

    fn retain_prefix_math_table(
        &mut self,
        required: Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    ) {
        if let Some(required) = required {
            if self
                .math_table_prerequisite
                .as_ref()
                .is_some_and(|previous| previous != &required)
            {
                self.require_provider();
            } else {
                self.math_table_prerequisite = Some(required);
            }
        }
    }

    fn command_target(
        &mut self,
        head: &str,
        namespace: &(impl super::NamespaceKeyQuery + ?Sized),
    ) -> Option<super::SourceCommandTarget> {
        let targets = self.snapshot.compiler_targets_in_namespace(head, namespace);
        if targets.unknown || targets.may_be_generic || targets.targets.len() != 1 {
            if targets.unknown || !targets.targets.is_empty() {
                self.require_provider();
            }
            return None;
        }
        targets.targets.into_iter().next()
    }

    fn command(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        if !self.original_head_has_compilation(words) {
            return self.substitutions(words, context);
        }
        let written = super::source_effective_words(words, self.state.baseline.dialect, None);
        let Some(head) = written
            .first()
            .and_then(|word| word.as_registry_word().literal())
        else {
            return self.substitutions(words, context);
        };
        let Some(target) = self.command_target(head, &context.namespace_identity()) else {
            return self.substitutions(words, context);
        };
        let dependency = self.command_dependency(&target, head, context);
        if dependency.compiler_prerequisite.is_some()
            || (target.registry_backed && target.prepended.is_empty())
        {
            self.retain_prefix_dependency(dependency);
        }
        let arguments = written
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word())
            .collect::<Vec<_>>();
        if let Some((visit_substitutions, failure)) =
            self.command_without_body(&target, words, head, offset, context)
        {
            return failure.or_else(|| {
                if visit_substitutions {
                    self.substitutions(words, context)
                } else {
                    None
                }
            });
        }
        let (invocation, mut facts) = self.command_facts(&target, &arguments, context)?;
        let Some(spec) = self.original_compilation_spec(words, head, offset, &mut facts, context)
        else {
            return self.substitutions(words, context);
        };
        let (shapes, selection, preparation) =
            compilation_selection(spec, invocation, &facts, words, offset, self.state, context);

        if let Some(recipe) = preparation.as_ref().and_then(|recipe| recipe.structured()) {
            let mut failure = self.structured_compilation(words, offset, recipe, context);
            if let Some(failure) = &mut failure {
                failure
                    .dependencies
                    .push(self.command_dependency(&target, head, context));
            }
            return failure;
        }
        if let Some(failure) =
            self.selected_failure(spec, selection, invocation, &facts, words, head)
        {
            return Some(self.failure_at(
                offset,
                failure,
                vec![self.command_dependency(&target, head, context)],
            ));
        }
        if let Some(recipe) = preparation.as_ref().and_then(|recipe| recipe.switch()) {
            let mut failure = self.switch_compilation(words, offset, recipe, context);
            if let Some(failure) = &mut failure {
                failure
                    .dependencies
                    .push(self.command_dependency(&target, head, context));
            }
            return failure;
        }
        let namespace_bindings = preparation
            .as_ref()
            .and_then(|recipe| recipe.namespace_bindings());
        if let Some(recipe) = namespace_bindings
            && let Some(failure) = self.namespace_binding_prefix(words, recipe, context)
        {
            return Some(failure);
        }
        if (namespace_bindings.is_none_or(|recipe| {
            recipe.outcome != tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingOutcome::Inline
        })) && let Some(failure) = self.substitutions(words, context) {
            return Some(failure);
        }
        let NativeCompilationSteps::Known(steps) = spec.compilation_steps(
            selection,
            invocation,
            &shapes,
            &facts,
            self.state.baseline.compilation_dialect(),
        ) else {
            self.require_provider();
            return None;
        };
        let mut failure =
            self.visit_compilation_steps(words, offset, steps, context, (selection, spec))?;
        if matches!(selection, NativeCompilationSelection::Inline { .. }) {
            failure
                .dependencies
                .push(self.command_dependency(&target, head, context));
        }
        Some(failure)
    }

    fn original_compilation_spec(
        &mut self,
        words: &[WordExpr],
        head: &str,
        offset: u32,
        facts: &mut tcl_registry::InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationSpec> {
        let Some((spec, argument_offset)) =
            super::compiled_invocation::original_registration_descriptor(
                words, head, offset, facts, self.state, context,
            )
        else {
            self.require_provider();
            return None;
        };
        // Compiler operand coordinates remain independent of runtime members.
        facts.argument_offset = argument_offset;
        Some(spec)
    }

    fn original_head_has_compilation(&mut self, words: &[WordExpr]) -> bool {
        if let Some(capability) =
            head_compilation_capability(words, self.state.baseline.compilation_dialect())
        {
            capability
        } else {
            self.require_provider();
            false
        }
    }

    fn command_facts<'a>(
        &self,
        target: &'a super::SourceCommandTarget,
        arguments: &'a [tcl_registry::InvocationWord<'a>],
        context: SourceExecutionContext<'_>,
    ) -> Option<(
        tcl_registry::InvocationWords<'a>,
        Box<tcl_registry::InvocationFacts>,
    )> {
        let mut invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&target.command),
            arguments,
        );
        if let Some(dialect) = self.state.baseline.compilation_dialect() {
            invocation = invocation.with_dialect(dialect);
        }
        let facts = super::resolve_source_invocation_facts(
            context.registry,
            context
                .registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile),
            invocation,
            context.realm,
        )?;
        Some((invocation, facts))
    }

    fn command_dependency(
        &self,
        target: &super::SourceCommandTarget,
        head: &str,
        context: SourceExecutionContext<'_>,
    ) -> SourceNativeCompilationDependency {
        SourceNativeCompilationDependency {
            compiler_prerequisite: self
                .snapshot
                .table
                .state
                .runtime_command_compiler_prerequisite(
                    head,
                    &context.namespace_identity(),
                    tcl_runtime_api::CommandBindingGuard::ChunkEntry,
                )
                .map(Arc::new),
            target: target.clone(),
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            head: head.to_owned(),
            guard: tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry,
        }
    }

    fn command_without_body(
        &mut self,
        target: &super::SourceCommandTarget,
        words: &[WordExpr],
        head: &str,
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) -> Option<(bool, Option<SourceNativeCompilationFailure>)> {
        if self
            .state
            .runtime_noop_header(
                self.state
                    .compiler_identity_at_lookup(head, &context.namespace_identity())
                    .or(target.identity.as_ref()),
                target.implementation_generation,
                target.implementation_allocation.as_ref(),
            )
            .is_some()
        {
            return Some((true, None));
        }

        let plan = self.snapshot.table.state.actual_ensemble_plan(
            target,
            head,
            &context.namespace_identity(),
            Some((
                crate::registry_invocation::OriginalNativeCompilerInvocation {
                    image: self.chunk.source.source_image(),
                    words,
                    offset,
                    config: context.config,
                    source_protocol:
                        super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(
                            self.state,
                        ),
                    compiler_dialect: self.state.baseline.compilation_dialect(),
                    context: context.compilation,
                    operand_from: 2,
                },
                context.registry,
            )),
        )?;
        match plan {
            super::ensemble_compilation::ActualEnsemblePlan::Operation { recipe, .. } => Some((
                false,
                self.structured_compilation(words, offset, &recipe, context),
            )),
            super::ensemble_compilation::ActualEnsemblePlan::Generic
            | super::ensemble_compilation::ActualEnsemblePlan::Named { .. } => Some((true, None)),
            super::ensemble_compilation::ActualEnsemblePlan::Unknown => {
                // A registered recipe can describe the delegated compiler
                // only after the original map and every private compiler
                // dependency have been validated by the shared selector.
                // A public registration alone cannot close this obligation.
                let selected = super::compiled_invocation::select_invocation_boxed(
                    words,
                    self.state,
                    &SourceExecutionContext {
                        compilation_snapshot: Some(self.snapshot),
                        ..context
                    },
                    offset,
                );
                if selected
                    .admission
                    .is_none_or(|admission| admission == NativeCompilationSelection::Unknown)
                {
                    self.require_provider();
                }
                (!target.registry_backed).then_some((true, None))
            }
        }
    }

    fn switch_compilation(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        recipe: &tcl_registry::native_switch_compilation::NativeSwitchInstruction,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        if let NativeCompilerWordOperand::Original(index) = &recipe.subject {
            let Some(subject) = words.get(*index) else {
                self.require_provider();
                return None;
            };
            if let Some(failure) = self.substitutions(std::slice::from_ref(subject), context) {
                return Some(failure);
            }
        }
        let image = self.chunk.source.source_image().clone();
        let selected = super::compiled_invocation::select_invocation_boxed(
            words,
            self.state,
            &SourceExecutionContext {
                compilation_snapshot: Some(self.snapshot),
                ..context
            },
            offset,
        );
        for arm in &recipe.arms {
            if !arm.compile_body {
                continue;
            }
            let Some(span) = arm.body else {
                self.require_provider();
                return None;
            };
            let Some(bytes) = image.bytes().get(span.as_range()) else {
                self.require_provider();
                return None;
            };
            let original = tcl_lexer::SourceImage::from_bytes(bytes, image.channel());
            if let Some(failure) = self.script(
                &original,
                span.start(),
                SourceExecutionContext {
                    depth: context.depth + 1,
                    ..context
                },
            ) {
                return Some(failure);
            }
            if !self.possible_error {
                let Some(parent) = selected.admitted.as_ref().filter(|parent| {
                    matches!(
                        parent.as_ref(),
                        super::compiled_invocation::SourceNativeCompilerAdmission::Inline(_)
                    )
                }) else {
                    self.require_provider();
                    continue;
                };
                self.compiled_children
                    .entry(CommandAllocationSite {
                        source: Arc::clone(&self.chunk.source),
                        offset: span.start(),
                    })
                    .or_default()
                    .push(SourceCompiledChild {
                        enclosing: self.chunk.clone(),
                        table: Arc::clone(&self.snapshot.table),
                        parent: Arc::clone(parent),
                        script: super::ExecutedScriptSource {
                            text: original,
                            origin: Arc::clone(&self.chunk.source),
                            mapping: super::ExecutedScriptMapping::Contiguous {
                                base: span.start(),
                            },
                        },
                        compilation: context.compilation,
                        parent_namespace_key: context.namespace_identity(),
                        namespace_key: context.namespace_identity(),
                        compiler_visits: Arc::default(),
                    });
            }
        }
        None
    }

    fn namespace_binding_prefix(
        &mut self,
        words: &[WordExpr],
        recipe: &tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        use tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingVisit;
        for visit in &recipe.visits {
            match visit {
                NativeNamespaceBindingVisit::Word(NativeCompilerWordOperand::Original(index)) => {
                    let Some(word) = words.get(*index) else {
                        self.require_provider();
                        return None;
                    };
                    if let Some(failure) = self.substitutions(std::slice::from_ref(word), context) {
                        return Some(failure);
                    }
                }
                // These compiler visits retain geometry, not a live cell.
                NativeNamespaceBindingVisit::DeclareLocal(_)
                | NativeNamespaceBindingVisit::Literal(_)
                | NativeNamespaceBindingVisit::Word(
                    NativeCompilerWordOperand::LiteralExpansion { .. },
                ) => {}
            }
        }
        None
    }

    fn failure_at(
        &self,
        offset: u32,
        failure: tcl_registry::native_compilation::NativeCompilationFailure,
        dependencies: Vec<SourceNativeCompilationDependency>,
    ) -> SourceNativeCompilationFailure {
        SourceNativeCompilationFailure {
            chunk: self.chunk.clone(),
            invocation: CommandAllocationSite {
                source: Arc::clone(&self.chunk.source),
                offset,
            },
            failure,
            dependencies,
            math_table_prerequisite: None,
            contexts: Vec::new(),
        }
    }

    fn visit_compilation_steps(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        steps: Vec<NativeCompilationStep>,
        context: SourceExecutionContext<'_>,
        native: (
            NativeCompilationSelection,
            tcl_registry::native_compilation::NativeCompilationSpec,
        ),
    ) -> Option<SourceNativeCompilationFailure> {
        for step in steps {
            let failure = match step {
                NativeCompilationStep::Body(body) => self.bodies(
                    words,
                    offset,
                    std::slice::from_ref(&body),
                    context,
                    native.0,
                    native.1,
                ),
                NativeCompilationStep::Expression(expression) => {
                    self.expression(words, offset, expression, context)
                }
            };
            if failure.is_some() {
                return failure;
            }
        }
        None
    }

    fn selected_failure(
        &self,
        spec: tcl_registry::native_compilation::NativeCompilationSpec,
        selection: NativeCompilationSelection,
        invocation: tcl_registry::InvocationWords<'_>,
        facts: &tcl_registry::InvocationFacts,
        words: &[WordExpr],
        head: &str,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationFailure> {
        if selection != NativeCompilationSelection::CompileError {
            return None;
        }
        let original = self.chunk.source.try_text().ok()?;
        let spellings = words
            .iter()
            .map(|word| {
                original.get(tcl_lexer::word_span_at(original, word.source().span).as_range())
            })
            .collect::<Option<Vec<_>>>();
        if let Some(spellings) = spellings
            && let Some((_, arguments)) = spellings.split_first()
        {
            spec.failure_for_selection_with_source(
                selection,
                invocation,
                facts,
                self.state.baseline.compilation_dialect(),
                head,
                arguments,
            )
        } else {
            spec.failure_for_selection(
                selection,
                invocation,
                facts,
                self.state.baseline.compilation_dialect(),
            )
        }
    }

    fn presentation(
        &self,
        source: &str,
        base: u32,
        segment: &crate::segmenter::SegmentedCommand,
    ) -> SourceNativeCompilationContext {
        let command = segment.span.start().checked_sub(base).and_then(|start| {
            let mut end = segment.span.end().checked_sub(base)?;
            if let Some(last) = segment.word_fragments.last()
                && let (Some(first), Some(last)) = (last.first(), last.last())
            {
                let span = tcl_lexer::Span::new(
                    first.token.span.start().checked_sub(base)?,
                    last.token.span.end().checked_sub(base)?,
                );
                end = end.max(tcl_lexer::word_span_at(source, span).end());
            }
            source
                .get(tcl_lexer::Span::new(start, end).as_range())
                .map(str::to_owned)
        });
        let original = self.chunk.source.source_image().bytes();
        let index = tcl_lexer::LineIndex::from_bytes(original);
        SourceNativeCompilationContext {
            invocation: CommandAllocationSite {
                source: Arc::clone(&self.chunk.source),
                offset: segment.span.start(),
            },
            command,
            before_context: Vec::new(),
            after_context: Vec::new(),
            line_in_chunk: index
                .line_at(segment.span.start())
                .checked_sub(index.line_at(self.chunk.offset))
                .and_then(|line| line.checked_add(1)),
        }
    }

    fn expression_steps(
        &self,
        text: &str,
        expression: tcl_registry::native_compilation::NativeCompiledExpressionOperand,
        dialect: tcl_registry::InvocationDialect,
        context: SourceExecutionContext<'_>,
    ) -> (
        Vec<NativeExpressionCompilerStep>,
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    ) {
        let mut parser = dialect.expression_parse_context(context.registry.profile());
        parser.lexer_grammar = context.config.grammar_over(parser.lexer_grammar);
        let table = self
            .state
            .baseline
            .native_entry
            .as_ref()
            .and_then(|entry| entry.math_functions.as_ref());
        let mut used_math_table = false;
        let steps = expression.compiler_steps(text, &parser, |name| {
            used_math_table = true;
            let Some(table) = table else {
                return NativeMathFunctionResolution::Unknown;
            };
            match table.lookup(name) {
                tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row) => {
                    row.arity
                        .map_or(NativeMathFunctionResolution::Unknown, |arity| {
                            NativeMathFunctionResolution::Known { arity }
                        })
                }
                tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Absent => {
                    NativeMathFunctionResolution::Absent
                }
                tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Unknown => {
                    NativeMathFunctionResolution::Unknown
                }
            }
        });
        let math_table_prerequisite = used_math_table
            .then(|| {
                let entry = self.state.baseline.native_entry.as_ref()?;
                Some(
                    tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite {
                        interpreter: entry.interpreter,
                        table: entry.math_functions.as_ref()?.clone(),
                    },
                )
            })
            .flatten();
        (steps, math_table_prerequisite)
    }

    fn expression_source_base(&self, word: &WordExpr, text: &str) -> Option<u32> {
        let base = match word {
            WordExpr::BracedLiteral { source, .. } => source.span.start().checked_add(1),
            WordExpr::Literal { source, .. } => Some(source.span.start()),
            WordExpr::Template { parts, .. } if parts.len() == 1 => match &parts[0] {
                WordPart::Text { source, .. } => Some(source.span.start()),
                _ => None,
            },
            _ => None,
        };
        let original = self.chunk.source.source_image().bytes();
        base.filter(|base| {
            original.get(*base as usize..(*base as usize).saturating_add(text.len()))
                == Some(text.as_bytes())
        })
    }

    fn expression(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        expression: tcl_registry::native_compilation::NativeCompiledExpressionOperand,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let Some(dialect) = self.state.baseline.compilation_dialect() else {
            self.require_provider();
            return None;
        };
        let mut parser = dialect.expression_parse_context(context.registry.profile());
        parser.lexer_grammar = context.config.grammar_over(parser.lexer_grammar);
        if !expression.requires_source_preflight(&parser) {
            return None;
        }
        let effective = super::source_effective_words(words, self.state.baseline.dialect, None);
        let Some(text) = effective
            .get(expression.argument + 1)
            .and_then(|word| word.as_registry_word().literal())
        else {
            self.require_provider();
            return None;
        };
        let (steps, math_table_prerequisite) =
            self.expression_steps(text, expression, dialect, context);
        self.retain_prefix_math_table(math_table_prerequisite.clone());
        let base = self.expression_source_base(&words[expression.argument + 1], text);
        for step in steps {
            let span = match step {
                NativeExpressionCompilerStep::Unknown => {
                    self.require_provider();
                    return None;
                }
                NativeExpressionCompilerStep::Failure(failure) => {
                    let note = expression.error_context.note();
                    if !note.is_empty() {
                        self.before_next_context.push(note.to_owned());
                    }
                    return Some(SourceNativeCompilationFailure {
                        chunk: self.chunk.clone(),
                        invocation: CommandAllocationSite {
                            source: Arc::clone(&self.chunk.source),
                            offset,
                        },
                        failure,
                        dependencies: Vec::new(),
                        math_table_prerequisite: math_table_prerequisite.clone(),
                        contexts: Vec::new(),
                    });
                }
                NativeExpressionCompilerStep::Script(span) => span,
            };
            let Some(base) = base else {
                self.require_provider();
                return None;
            };
            let Some(spelling) = text.get(span.as_range()) else {
                self.require_provider();
                continue;
            };
            let Some(script) = spelling
                .strip_prefix('[')
                .and_then(|text| text.strip_suffix(']'))
            else {
                self.require_provider();
                continue;
            };
            let child_base = base.checked_add(span.start())?.checked_add(1)?;
            if let Some(mut failure) = self.script(
                &tcl_lexer::SourceImage::native(script.as_bytes()),
                child_base,
                SourceExecutionContext {
                    depth: context.depth + 1,
                    ..context
                },
            ) {
                if let Some(prerequisite) = &math_table_prerequisite {
                    if failure
                        .math_table_prerequisite
                        .as_ref()
                        .is_some_and(|old| old != prerequisite)
                    {
                        self.require_provider();
                        failure.failure.message = None;
                        failure.failure.error_code = None;
                        failure.failure.error_info = None;
                    } else {
                        failure.math_table_prerequisite = Some(prerequisite.clone());
                    }
                }
                let note = expression.error_context.note();
                if !note.is_empty()
                    && let Some(inner) = failure.contexts.last_mut()
                {
                    inner.after_context.push(note.to_owned());
                }
                return Some(failure);
            }
        }
        None
    }

    /// Capture original parent admission and literal source outside recursive
    /// compiler frames; a prospective runtime body never creates this receipt.
    #[inline(never)]
    fn compiled_child_receipt(
        &self,
        words: &[WordExpr],
        offset: u32,
        argument: usize,
        compilation: tcl_registry::native_compilation::NativeCompilationContext,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceCompiledChild> {
        let WordExpr::BracedLiteral { text, source } = words.get(argument + 1)? else {
            return None;
        };
        let selected = super::compiled_invocation::select_invocation_boxed(
            words,
            self.state,
            &SourceExecutionContext {
                compilation_snapshot: Some(self.snapshot),
                ..context
            },
            offset,
        );
        let parent = selected.admitted.as_ref()?;
        if !matches!(
            parent.as_ref(),
            super::compiled_invocation::SourceNativeCompilerAdmission::Inline(_)
        ) {
            return None;
        }
        Some(SourceCompiledChild {
            enclosing: self.chunk.clone(),
            table: Arc::clone(&self.snapshot.table),
            parent: Arc::clone(parent),
            script: super::ExecutedScriptSource {
                text: tcl_lexer::SourceImage::native(text.as_bytes()),
                origin: Arc::clone(&self.chunk.source),
                mapping: super::ExecutedScriptMapping::Contiguous {
                    base: source.span.start().checked_add(1)?,
                },
            },
            compilation,
            parent_namespace_key: context.namespace_identity(),
            namespace_key: context.namespace_identity(),
            compiler_visits: Arc::default(),
        })
    }

    fn bodies(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        bodies: &[tcl_registry::native_compilation::NativeCompiledBodyOperand],
        context: SourceExecutionContext<'_>,
        selection: NativeCompilationSelection,
        spec: tcl_registry::native_compilation::NativeCompilationSpec,
    ) -> Option<SourceNativeCompilationFailure> {
        for body in bodies {
            let Some(WordExpr::BracedLiteral { text, source }) = words.get(body.argument + 1)
            else {
                continue;
            };
            let compilation = spec.body_context_for_operand(
                self.state.baseline.compilation_dialect(),
                context.compilation,
                selection,
                super::compiled_invocation::word_shape(&words[body.argument + 1]),
            );
            let Some(compilation) = body.entered_context(compilation) else {
                continue;
            };
            let failure = self.script(
                &tcl_lexer::SourceImage::native(text.as_bytes()),
                source.span.start().saturating_add(1),
                SourceExecutionContext {
                    compilation,
                    depth: context.depth + 1,
                    ..context
                },
            );
            let Some(mut failure) = failure else {
                if !self.possible_error
                    && let Some(child) = self.compiled_child_receipt(
                        words,
                        offset,
                        body.argument,
                        compilation,
                        context,
                    )
                {
                    self.compiled_children
                        .entry(CommandAllocationSite {
                            source: Arc::clone(&self.chunk.source),
                            offset: source.span.start().saturating_add(1),
                        })
                        .or_default()
                        .push(child);
                }
                continue;
            };
            if body.failure_scope == NativeCompilationFailureScope::EnclosingChunk {
                let last = failure.contexts.last_mut()?;
                let child_base = source.span.start().checked_add(1)?;
                let child_line = last
                    .invocation
                    .offset
                    .checked_sub(child_base)
                    .map(|offset| tcl_lexer::LineIndex::new(text).line_at(offset))
                    .and_then(|line| line.checked_add(1));
                last.after_context
                    .push(body.error_context.note(child_line)?);
                return Some(failure);
            }
            self.snapshot
                .generic_fallbacks
                .insert(CommandAllocationSite {
                    source: Arc::clone(&self.chunk.source),
                    offset,
                });
        }
        None
    }

    fn substitutions(
        &mut self,
        words: &[WordExpr],
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        for word in words {
            let failure = match word {
                WordExpr::CommandSubstitution { spelling, source } => {
                    self.brackets(spelling, source, context)
                }
                WordExpr::Template { parts, .. } => parts.iter().find_map(|part| match part {
                    WordPart::CommandSubstitution { spelling, source } => {
                        self.brackets(spelling, source, context)
                    }
                    WordPart::Variable { source, .. } => self.variable_indices(source, context),
                    _ => None,
                }),
                WordExpr::Expand { word, .. } => {
                    self.substitutions(std::slice::from_ref(word), context)
                }
                WordExpr::Variable { source, .. } => self.variable_indices(source, context),
                _ => None,
            };
            if failure.is_some() {
                return failure;
            }
        }
        None
    }

    fn variable_indices(
        &mut self,
        site: &crate::ir::SourceSite,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let image = self.chunk.source.source_image().clone();
        let original = image.try_text().ok()?;
        let (spelling, source) =
            super::original_variable_source(original, 0, site, context.config)?;
        let input = tcl_lexer::SourceImage::native(spelling.as_bytes());
        let Ok(length) = u32::try_from(input.len()) else {
            self.require_provider();
            return None;
        };
        let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
            input,
            tcl_lexer::Span::new(0, length),
            tcl_lexer::SubstFlags::default(),
            context.config,
        ) else {
            self.require_provider();
            return None;
        };
        let mut pending = vec![(arena.root(), 0, context.depth)];
        while let Some((list, next, depth)) = pending.pop() {
            if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth) {
                self.require_provider();
                continue;
            }
            let Some(component) = arena.list(list).get(next) else {
                continue;
            };
            pending.push((list, next + 1, depth));
            match component.part {
                tcl_lexer::ExecutablePart::Command { body } => {
                    let Some(script) = arena.bytes(body) else {
                        self.require_provider();
                        continue;
                    };
                    let Some(offset) = source.span.start().checked_add(body.start()) else {
                        self.require_provider();
                        continue;
                    };
                    if let Some(failure) = self.script(
                        &tcl_lexer::SourceImage::native(script),
                        offset,
                        SourceExecutionContext {
                            depth: depth + 1,
                            ..context
                        },
                    ) {
                        return Some(failure);
                    }
                }
                tcl_lexer::ExecutablePart::Variable {
                    index: Some(index), ..
                } => {
                    pending.push((index, 0, depth + 1));
                }
                tcl_lexer::ExecutablePart::Expression { .. }
                | tcl_lexer::ExecutablePart::ParseError(_) => self.possible_error = true,
                _ => {}
            }
        }
        None
    }

    fn brackets(
        &mut self,
        spelling: &str,
        source: &crate::ir::SourceSite,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let script = spelling.strip_prefix('[')?.strip_suffix(']')?;
        self.script(
            &tcl_lexer::SourceImage::native(script.as_bytes()),
            source.span.start().saturating_add(1),
            SourceExecutionContext {
                depth: context.depth + 1,
                ..context
            },
        )
    }
}

fn compilation_selection(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    invocation: tcl_registry::InvocationWords<'_>,
    facts: &tcl_registry::InvocationFacts,
    words: &[WordExpr],
    offset: u32,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> (
    Vec<tcl_registry::native_compilation::NativeCompilationWordShape>,
    NativeCompilationSelection,
    Option<crate::registry_invocation::OriginalNativeCompilerPreparation>,
) {
    let shapes = words
        .iter()
        .skip(1)
        .map(super::compiled_invocation::word_shape)
        .collect::<Vec<_>>();
    let original =
        state.current_source_origin.as_ref().and_then(|origin| {
            crate::registry_invocation::original_native_compilation(
                spec,
                crate::registry_invocation::OriginalNativeCompilerInvocation {
                    image: origin.source_image(),
                    words,
                    offset,
                    config: context.config,
                    source_protocol:
                        super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(
                            state,
                        ),
                    compiler_dialect: state.baseline.compilation_dialect(),
                    context: context.compilation,
                    operand_from: facts.argument_offset + 1,
                },
            )
        });
    let (selected, recipe) = original.unwrap_or_else(|| {
        (
            spec.select_for_facts(
                invocation,
                &shapes,
                facts,
                invocation.arguments().dialect(),
                context.compilation,
            ),
            None,
        )
    });
    (shapes, selected, recipe)
}

/// Eligibility of the original head syntax, before any runtime name lookup.
fn head_compilation_capability(
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<bool> {
    words
        .first()
        .map(super::compiled_invocation::word_shape)?
        .compiler_head(dialect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceAnalysisOptions;
    use tcl_registry::native_compilation::{NativeCompilationContext, NativeCompilationFrame};

    fn analyse(source: &str) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_4,
                )),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn original_return_preparation_uses_selected_stack_options_before_result() {
        for version in tcl_dialect::TclVersion::ALL {
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let registry =
                tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let mut sources = vec!["return [set result BODY]", "return"];
            if version >= tcl_dialect::TclVersion::V8_5 {
                sources.extend([
                    "return -level 0 -code error [set result BODY]",
                    "return -options [set options $opts] [set result BODY]",
                ]);
            }
            if version >= tcl_dialect::TclVersion::V8_6 {
                sources.push("return -code [set code $code] [set result BODY]");
            }
            for source in sources {
                let inventory = SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                    &registry,
                    SourceAnalysisOptions {
                        native_entry: Some(&entry),
                        invocation_dialect: Some(dialect),
                        native_compilation: NativeCompilationContext {
                            mode: NativeCompilationMode::BytecodeObject,
                            frame: NativeCompilationFrame::ProcedureCode,
                            loop_depth: 0,
                            catch_depth: Some(0),
                        },
                        ..Default::default()
                    },
                );
                assert!(
                    !inventory.native_compilation_provider_required_at(0),
                    "{version:?} {source}"
                );
                assert!(
                    inventory.native_compilation_failure_at(0).is_none(),
                    "{version:?} {source}"
                );
            }
        }
    }

    #[test]
    fn original_set_preparation_visits_only_native_target_and_value_regions() {
        for source in [
            "set x VALUE",
            "set x",
            "set {a([incr bad extra args])} VALUE",
        ] {
            let inventory = analyse(source);
            assert!(
                !inventory.native_compilation_provider_required_at(0),
                "{source}"
            );
            assert!(
                inventory.native_compilation_failure_at(0).is_none(),
                "{source}"
            );
        }
        for source in [
            "set x [incr bad extra args]",
            "set a([incr bad extra args]) VALUE",
            "set $a([incr bad extra args]) VALUE",
        ] {
            let inventory = analyse(source);
            assert!(
                inventory.native_compilation_failure_at(0).is_some(),
                "{source}"
            );
            assert!(
                !inventory.native_compilation_provider_required_at(0),
                "{source}"
            );
        }
    }

    #[test]
    fn original_each_rejection_precedes_legacy_arity_projection() {
        let source = "set before 1; foreach \"{i\" {A} {set never 1}";
        let inventory = analyse(source);
        let failure = inventory
            .native_compilation_failure_at(0)
            .expect("native varlist compilation rejection");
        assert_eq!(
            failure.failure.message.as_deref(),
            Some("unmatched open brace in list")
        );
        assert!(!inventory.has_site(0));
        assert!(!inventory.native_compilation_provider_required_at(0));
        assert!(!failure.dependencies.is_empty());
    }

    #[test]
    fn original_empty_each_declines_without_manufacturing_a_compile_failure() {
        let inventory = analyse("foreach {} {A} {set never 1}");
        assert!(inventory.native_compilation_failure_at(0).is_none());
        assert!(!inventory.native_compilation_provider_required_at(0));
        assert_eq!(
            inventory
                .invocation_at_source("foreach", 0)
                .native_compilation_admission_selection(),
            NativeCompilationSelection::Generic
        );
    }

    #[test]
    fn original_c84_catch_rolls_back_a_rejected_body_without_granting_child_admission() {
        let source = "catch {incr x bad extra} result; set after 1";
        let inventory = analyse(source);
        assert!(inventory.native_compilation_failure_at(0).is_none());
        assert!(!inventory.native_compilation_provider_required_at(0));
        assert_eq!(
            inventory
                .invocation_at_source("catch", 0)
                .native_compilation_admission_selection(),
            NativeCompilationSelection::Generic
        );
        assert!(
            inventory
                .invocation_at_source("incr", 7)
                .admitted_inline_invocation()
                .is_none()
        );
    }

    #[test]
    fn escaped_c84_head_does_not_donate_an_early_set_arity_failure() {
        let plain = analyse("set ::before 1; set");
        assert!(plain.native_compilation_failure_at(0).is_some());
        let escaped = analyse(r"set ::before 1; se\x74");
        assert!(escaped.native_compilation_failure_at(0).is_none());
        assert!(!escaped.native_compilation_provider_required_at(0));
        let binding = escaped.invocation_at_source("set", 16);
        assert_eq!(
            binding.native_compilation_selection(),
            NativeCompilationSelection::Generic
        );
    }

    #[test]
    fn compiler_visits_runtime_dead_expression_substitutions_and_array_indices() {
        for source in [
            "set before 1; expr {0 && [incr bad extra args]}",
            "set before 1; expr {1 ? 1 : [incr bad extra args]}",
            "set before 1; set x $a([incr bad extra args])",
            "set before 1; expr {1 +}",
        ] {
            let analysis = analyse(source);
            assert!(
                analysis.native_compilation_failure_at(0).is_some(),
                "{source}"
            );
            assert!(!analysis.has_site(0), "prefix executed: {source}");
        }
    }

    #[test]
    fn unsupported_expression_compile_success_retains_an_entry_obligation() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let mut entry = crate::environment_ingress::captured_native_entry(profile);
        entry.math_functions.as_mut().unwrap().closed = false;
        let analysis = SourceCommandBindings::analyse_with_options(
            "set before 1; expr {future_function(1)}",
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_4,
                )),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        );
        assert!(analysis.native_compilation_failure_at(0).is_none());
        assert!(analysis.native_compilation_provider_required_at(0));
    }

    #[test]
    fn definite_compile_error_prevents_earlier_effects_and_dead_code_execution() {
        let source = "rename set saved; return; set x extra bad";
        let analysis = analyse(source);
        let failure = analysis.native_compilation_failure_at(0).unwrap();
        assert_eq!(
            failure.failure.message.as_deref(),
            Some("wrong # args: should be \"set varName ?newValue?\"")
        );
        assert!(!analysis.has_site(0));
        assert!(
            super::super::source_binding(&analysis.final_state, "set", "::")
                .proved_target()
                .is_some_and(|target| target.registry_backed)
        );
    }

    #[test]
    fn compiler_prunes_only_native_literal_boolean_bodies() {
        for source in [
            "set x 1; if {0} {set x extra bad}",
            "set x 1; while {0} {set x extra bad}",
        ] {
            assert!(
                analyse(source).native_compilation_failure_at(0).is_none(),
                "{source}"
            );
        }
        for source in [
            "set x 1; if {0+0} {set x extra bad}",
            "set x 1; foreach i {} {set x extra bad}",
        ] {
            assert!(
                analyse(source).native_compilation_failure_at(0).is_some(),
                "{source}"
            );
        }
    }

    #[test]
    fn decoded_escape_values_do_not_acquire_literal_compiler_tokens() {
        for source in [r#"if "\61""#, r"if \61", r"i\146 {1}"] {
            assert!(
                analyse(source).native_compilation_failure_at(0).is_none(),
                "{source}"
            );
        }
        for source in [r#""if" {1}"#, "{if} {1}"] {
            assert!(
                analyse(source).native_compilation_failure_at(0).is_some(),
                "{source}"
            );
        }
    }

    #[test]
    fn conditional_failure_keeps_authored_operand_quoting() {
        for (source, operand) in [("if {1}", "{1}"), ("if \"1\"", "\"1\""), ("if 1", "1")] {
            let analysis = analyse(source);
            let failure = analysis.native_compilation_failure_at(0).expect(source);
            assert_eq!(
                failure.failure.message.as_deref(),
                Some(format!("wrong # args: no script following \"{operand}\" argument").as_str()),
            );
        }
        for source in ["if", "{if}", "\"if\""] {
            let analysis = analyse(source);
            assert_eq!(
                analysis
                    .native_compilation_failure_at(0)
                    .unwrap()
                    .failure
                    .message
                    .as_deref(),
                Some("wrong # args: no expression after \"if\" argument"),
            );
        }
    }

    #[test]
    fn caught_child_compiler_failure_falls_back_without_rejecting_the_parent() {
        let source = "set x 1; catch {set x extra bad}; set y 2";
        let analysis = analyse(source);
        assert!(analysis.native_compilation_failure_at(0).is_none());
        let catch_offset = u32::try_from(source.find("catch").unwrap()).unwrap();
        let catch = analysis.invocation_at_source("catch", catch_offset);
        assert_eq!(
            catch.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        assert!(catch.may_use_live_dispatch);
        let last = u32::try_from(source.rfind("set y").unwrap()).unwrap();
        assert!(
            analysis
                .invocation_at_source("set", last)
                .proved_execution_target()
                .is_some()
        );
    }

    #[test]
    fn failure_retains_native_dependencies_and_original_nested_contexts() {
        let source = "set before 1\nif {1} {\n    set x extra bad\n}";
        let analysis = analyse(source);
        let failure = analysis.native_compilation_failure_at(0).unwrap();
        assert_eq!(failure.failure.error_code.as_deref(), Some("NONE"));
        assert_eq!(failure.contexts.len(), 2);
        assert_eq!(
            failure.contexts[0].command.as_deref(),
            Some("set x extra bad")
        );
        assert_eq!(failure.contexts[0].line_in_chunk, Some(3));
        assert_eq!(
            failure.contexts[1].command.as_deref(),
            Some("if {1} {\n    set x extra bad\n}")
        );
        assert_eq!(failure.contexts[1].line_in_chunk, Some(2));
        assert_eq!(failure.dependencies.len(), 2);
        assert!(failure.dependencies.iter().all(|dependency| {
            dependency.target.registry_backed
                && dependency.guard
                    == tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry
        }));
        assert_eq!(failure.dependencies[0].head, "set");
        assert_eq!(failure.dependencies[1].head, "if");
        assert!(!analysis.has_site(0));
    }

    #[test]
    fn parent_compiled_child_retains_admission_when_runtime_fallback_is_opaque() {
        for body in ["set selected YES", "set ::a(new) NEW"] {
            assert_original_child_compilation(body);
        }
    }

    fn assert_original_child_compilation(body: &str) {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let source = format!("if {{$flag}} {{{body}}}");
        let compilation = tcl_registry::native_compilation::NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
            ..Default::default()
        };
        let analysed = SourceCommandBindings::analyse_with_options(
            &source,
            config,
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: compilation,
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find(body).unwrap()).unwrap();
        let original = analysed
            .invocation_at_source("set", offset)
            .native_compilation_admission_selection();
        assert_ne!(
            original,
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        let namespace = super::super::SourceNamespaceKey::from_native_entry(&entry).unwrap();
        let frame = crate::var_resolve::VariableExecutionFrame::Global
            .with_namespace_identity(namespace.clone());
        let context = original_child_context(compilation, config, &registry, offset, &frame);

        assert_authored_child_context_is_unknown(&analysed, body, context);
        for same_source in [true, false] {
            let mut inventory = analysed.clone();
            let mut late = analysed.final_state.as_ref().clone();
            if !same_source {
                late.current_source_origin = Some(Arc::new(
                    super::super::SourceOriginId::authored(&Arc::from(format!("{source}\n"))),
                ));
                inventory
                    .root_origin
                    .clone_from(&late.current_source_origin);
            }
            late.mark_opaque_binding_mutation();
            inventory.walk_source(body, offset, &mut late, &context);
            let site = CommandAllocationSite {
                source: Arc::clone(inventory.root_origin.as_ref().unwrap()),
                offset,
            };
            assert!(inventory.compilation_provider_required.contains(&site));
            assert_eq!(
                inventory.native_compilation_provider_required_at(offset),
                !same_source
            );
            assert!(late.has_opaque_domain());
            let binding = inventory.invocation_at_source("set", offset);
            assert_eq!(
                binding.native_compilation_admission_selection(),
                if same_source {
                    original
                } else {
                    tcl_registry::native_compilation::NativeCompilationSelection::Unknown
                }
            );
            assert!(binding.proved_execution_target().is_none());
            if same_source {
                // An actual new compilation of the enclosing command with a
                // missing parent slot cannot borrow the old child's recipe.
                let mut reentry = analysed.final_state.as_ref().clone();
                reentry.remove(super::super::SourceCommandKey::slot(
                    namespace.clone(),
                    "if".into(),
                ));
                inventory.walk_source(
                    &source,
                    0,
                    &mut reentry,
                    &SourceExecutionContext {
                        invocation_offset: 0,
                        ..context
                    },
                );
                assert_eq!(
                    inventory
                        .invocation_at_source("set", offset)
                        .native_compilation_admission_selection(),
                    tcl_registry::native_compilation::NativeCompilationSelection::Unknown
                );
            }
        }
    }

    fn assert_authored_child_context_is_unknown(
        analysed: &SourceCommandBindings,
        body: &str,
        context: SourceExecutionContext<'_>,
    ) {
        // An authored presentation of the same namespace cannot stand in for
        // the original native entry, even when the source image is unchanged.
        let mut inventory = analysed.clone();
        let mut state = analysed.final_state.as_ref().clone();
        state.mark_opaque_binding_mutation();
        inventory.walk_source(
            body,
            context.invocation_offset,
            &mut state,
            &SourceExecutionContext {
                frame: &crate::var_resolve::VariableExecutionFrame::Global,
                namespace_key: None,
                ..context
            },
        );
        let binding = inventory.invocation_at_source("set", context.invocation_offset);
        assert_eq!(
            binding.native_compilation_admission_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
        assert!(binding.proved_execution_target().is_none());
    }

    fn original_child_context<'a>(
        compilation: tcl_registry::native_compilation::NativeCompilationContext,
        config: tcl_lexer::LexerConfig,
        registry: &'a tcl_registry::CommandRegistry,
        offset: u32,
        frame: &'a crate::var_resolve::VariableExecutionFrame,
    ) -> SourceExecutionContext<'a> {
        SourceExecutionContext {
            realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            compilation,
            compilation_snapshot: None,
            selected_compilation: None,
            namespace: "::",
            config,
            registry,
            depth: 0,
            frame,
            invocation_offset: offset,
            variable_read_owner: None,
            written_arguments: None,
            written_values: None,
            written_representations: None,
            written_objects: None,
            written_method_prefixes: None,
            written_variable_reads: None,
            namespace_key: frame.namespace_identity(),
            expression_source: None,
        }
    }

    #[test]
    fn compilation_error_observers_mutate_before_catch_continuation() {
        for (subject, mode) in [
            ("::errorCode", "w"),
            ("::errorInfo", "w"),
            ("::unrelated", "w"),
            ("::errorCode", "r"),
            ("::errorInfo", "r"),
        ] {
            let source = format!(
                "proc observer {{args}} {{proc set {{args}} {{return CUSTOM}}}}\n\
                 trace variable {subject} {mode} observer\n\
                 proc p {{}} {{set ::before 1; set x extra bad}}\n\
                 catch {{p}}\nset x 1"
            );
            let analysis = SourceCommandBindings::analyse_with_options(
                &source,
                tcl_lexer::LexerConfig::default(),
                &tcl_registry::CommandRegistry::build_default(),
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                        tcl_dialect::TclVersion::V8_4,
                    )),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..SourceAnalysisOptions::default()
                },
            );
            let offset = u32::try_from(source.rfind("set x 1").unwrap()).unwrap();
            let binding = analysis.invocation_at_source("set", offset);
            let states = analysis
                .points
                .iter()
                .filter(|point| point.dispatch)
                .map(|point| {
                    (
                        point.head.as_deref(),
                        point.offset,
                        point.state.opaque_domain,
                        point.state.source_variables.dynamic_traces,
                        point.state.source_variables.traced.len(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(
                binding.proved_execution_target().is_some(),
                subject == "::unrelated" || mode == "r",
                "{subject} {mode}: lookup unknown={}, compile unknown={}, states={states:?}",
                binding.unknown,
                binding.compiled_execution_unknown(),
            );
        }
    }
}
