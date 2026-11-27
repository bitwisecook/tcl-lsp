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

//! Compiler traversal precedes execution, including unreachable inline bodies.

pub(super) mod expression_syntax;
#[doc(hidden)]
pub mod ordered_locals;
mod original_preparation;

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, NativeCompilationSnapshot,
    SourceCommandBindings, SourceExecutionContext,
};
use crate::ir::CommandTokens;
use tcl_registry::native_compilation::{
    NativeCompilationFailureScope, NativeCompilationMode, NativeCompilationSelection,
    NativeCompilationStep, NativeCompilationSteps, NativeExpressionCompilerStep,
    NativeMathFunctionResolution,
};
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;

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
    pub expression_syntax_children: std::collections::BTreeMap<
        CommandAllocationSite,
        Vec<expression_syntax::SourceExpressionSyntaxChild>,
    >,
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
    if context.compilation().mode != NativeCompilationMode::BytecodeObject {
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
        chunk: chunk.clone(),
        possible_error: false,
        compiler_dependencies: Vec::new(),
        math_table_prerequisite: None,
        before_next_context: Vec::new(),
        compiler_invocations: std::collections::BTreeMap::new(),
        compiled_children: std::collections::BTreeMap::new(),
        expression_syntax_children: std::collections::BTreeMap::new(),
        locals: ordered_locals::SourceCompilerLocalInventory::at_entry(
            chunk.clone(),
            image.len(),
            state,
            context,
        ),
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
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
                eprintln!(
                    "ORIGINAL_NAMESPACE_CHILD_PROVIDER base={base} reason=math_table_prerequisite_mismatch"
                );
            }
            scanner.possible_error = true;
        } else {
            failure.math_table_prerequisite = Some(required.clone());
        }
    }
    // An entry-less compile is a fresh interpreter of its profile (user
    // decision 2026-10-19T14:20Z): its compiler table is the profile's own,
    // which the module's command bindings guard at run time. Only an
    // explicit unknown entry leaves the chunk's compiler provider unresolved.
    let native_entry_unavailable =
        state.baseline.native_entry.is_none() && state.baseline.unknown_entry;
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
    scanner.snapshot.source_locals = scanner
        .locals
        .take()
        .filter(|_| failure.is_none() && !scanner.possible_error)
        .map(Arc::new);
    for invocations in scanner.compiler_invocations.values_mut() {
        for invocation in invocations {
            invocation
                .source_locals
                .clone_from(&scanner.snapshot.source_locals);
        }
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
        expression_syntax_children: scanner.expression_syntax_children,
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

/// One segmented command's tokens, built on the heap.
#[inline(never)]
fn boxed_command_tokens(
    map: &tcl_lexer::SourceMap<'_>,
    config: tcl_lexer::LexerConfig,
    segment: &tcl_source::segment::SegmentedCommand,
) -> Box<CommandTokens> {
    Box::new(CommandTokens::from_segmented(map, config, segment))
}

/// What [`CompilerTraversal::command`] visits once the command is resolved.
enum CommandVisit {
    /// Only the command's own word substitutions.
    Substitutions,
    /// Resolution decided the outcome; nothing more is visited.
    Finished(Option<SourceNativeCompilationFailure>),
    /// An actual ensemble's selected operation.
    Ensemble(Box<tcl_registry::native_instruction_plan::NativeInstructionPlan>),
    /// A selected compiler's recipe and visits.
    Compiled(Box<CompiledCommandVisit>),
}

/// The resolved command a selected compiler visits, kept off the stack of
/// the traversal's descent.
struct CompiledCommandVisit {
    target: super::SourceCommandTarget,
    head: String,
    preparation: Option<crate::registry_invocation::OriginalNativeCompilerPreparation>,
    selection: NativeCompilationSelection,
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    steps: NativeCompilationSteps,
}

struct CompilerTraversal<'a> {
    state: &'a ModuleCommandBindings,
    snapshot: &'a mut NativeCompilationSnapshot,
    chunk: CommandAllocationSite,
    possible_error: bool,
    locals: Option<ordered_locals::SourceCompilerLocalInventory>,
    compiler_dependencies: Vec<SourceNativeCompilationDependency>,
    math_table_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    compiled_children: std::collections::BTreeMap<CommandAllocationSite, Vec<SourceCompiledChild>>,
    expression_syntax_children: std::collections::BTreeMap<
        CommandAllocationSite,
        Vec<expression_syntax::SourceExpressionSyntaxChild>,
    >,
    before_next_context: Vec<String>,
    compiler_invocations: std::collections::BTreeMap<
        CommandAllocationSite,
        Vec<super::compiler_inventory::SourceCompilerInvocation>,
    >,
}

impl CompilerTraversal<'_> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn require_provider(&mut self) {
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_NAMESPACE_CHILD_PROVIDER base={} previous_possible_error={} caller={}",
                self.chunk.offset,
                self.possible_error,
                std::panic::Location::caller(),
            );
        }
        self.possible_error = true;
        self.locals = None;
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
        if tcl_source::limits::MAX_SOURCE_NEST_DEPTH.exceeded(context.depth) {
            self.require_provider();
            return None;
        }
        let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.len()).ok()?),
            context.config,
            context.source_purpose(),
        ) else {
            self.require_provider();
            return None;
        };
        // The checked native plan owns the syntax cut. The structural view may
        // retain the failed command, but it must not select its incomplete argv.
        let mut config = context.source_purpose().config(context.config);
        config.strict_quoting = false;
        let Some(segments) =
            tcl_source::segment::segment_commands_image_with_offset_and_config(image, base, config)
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
            // Boxed: the tokens stay off this frame while the command's
            // bodies are traversed beneath it.
            let tokens = boxed_command_tokens(&map, context.config, &segment);
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
            .and_then(tcl_native_model::InvocationDialect::native_bytecode_storage_protocol)
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
        if selected
            .admission
            .is_none_or(|selection| selection == NativeCompilationSelection::Unknown)
            || selected.original_words.is_none()
        {
            self.locals = None;
        }
        let invocation = super::compiler_inventory::SourceCompilerInvocation {
            source_locals: None,
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

    /// Visit one command's compilation. Resolving it ([`Self::command_visit`])
    /// keeps large locals — its target, facts, word shapes and recipe — so it
    /// returns a boxed plan before the traversal descends into the command's
    /// words and bodies: each nesting level then costs only this frame and
    /// [`Self::compiled_command`]'s, which the source depth guard bounds.
    fn command(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        match self.command_visit(words, offset, context) {
            CommandVisit::Substitutions => self.substitutions(words, context),
            CommandVisit::Finished(failure) => failure,
            CommandVisit::Ensemble(recipe) => {
                self.structured_compilation(words, offset, &recipe, context)
            }
            CommandVisit::Compiled(visit) => self.compiled_command(words, offset, context, visit),
        }
    }

    /// Resolve what [`Self::command`] visits, without descending.
    #[inline(never)]
    fn command_visit(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        context: SourceExecutionContext<'_>,
    ) -> CommandVisit {
        if !self.original_head_has_compilation(words) {
            return CommandVisit::Substitutions;
        }
        let written = super::source_effective_words(words, self.state.baseline.dialect, None);
        let Some(head) = written
            .first()
            .and_then(|word| word.as_registry_word().literal())
        else {
            return CommandVisit::Substitutions;
        };
        let Some(target) = self.command_target(head, &context.namespace_identity()) else {
            return CommandVisit::Substitutions;
        };
        self.retain_command_head_dependency(&target, head, context);
        let arguments = written
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word())
            .collect::<Vec<_>>();
        if let Some(visit) = self.command_without_body(&target, words, head, offset, context) {
            return visit;
        }
        let Some((invocation, mut facts)) = self.command_facts(&target, &arguments, context) else {
            // No retained compiler descriptor represents attempted local
            // allocations for this target. Its ordinary words still compile.
            self.locals = None;
            return CommandVisit::Substitutions;
        };
        let Some(spec) = self.original_compilation_spec(words, head, offset, &mut facts, context)
        else {
            return CommandVisit::Substitutions;
        };
        let (shapes, selection, preparation) =
            compilation_selection(spec, invocation, &facts, words, offset, self.state, context);
        // A rejected hook can reserve locals before generic fallback. Only an
        // explicit attempted preparation represents that prefix completely.
        if self.incomplete_compiler_preparation(selection, spec, preparation.as_ref()) {
            self.locals = None;
        }
        let structured = preparation
            .as_ref()
            .is_some_and(|recipe| recipe.structured().is_some());
        if !structured
            && let Some(failure) =
                self.selected_failure(spec, selection, invocation, &facts, words, head)
        {
            return CommandVisit::Finished(Some(self.failure_at(
                offset,
                failure,
                vec![self.command_dependency(&target, head, context)],
            )));
        }
        // The compiler's visits are a function of its selection alone; they
        // are read here, ahead of the visits that precede them, so that the
        // descent keeps none of what selected them.
        let steps = spec.compilation_steps(
            selection,
            invocation,
            &shapes,
            &facts,
            self.state.baseline.compilation_dialect(),
        );
        CommandVisit::Compiled(Box::new(CompiledCommandVisit {
            target,
            head: head.to_owned(),
            preparation,
            selection,
            spec,
            steps,
        }))
    }

    /// Visit a resolved compiler's recipe in its original order. Every value
    /// it keeps stays in `visit`'s box; only references live in this frame.
    #[inline(never)]
    fn compiled_command(
        &mut self,
        words: &[WordExpr],
        offset: u32,
        context: SourceExecutionContext<'_>,
        mut visit: Box<CompiledCommandVisit>,
    ) -> Option<SourceNativeCompilationFailure> {
        let preparation = visit.preparation.as_ref();
        if let Some(recipe) = preparation.and_then(|recipe| recipe.structured()) {
            let failure = self.structured_compilation(words, offset, recipe, context);
            return self.with_command_dependency(failure, &visit.target, &visit.head, context);
        }
        if let Some(recipe) = preparation.and_then(|recipe| recipe.switch()) {
            let failure = self.switch_compilation(words, offset, recipe, context);
            return self.with_command_dependency(failure, &visit.target, &visit.head, context);
        }
        let namespace_bindings = preparation.and_then(|recipe| recipe.namespace_bindings());
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
        let NativeCompilationSteps::Known(steps) =
            std::mem::replace(&mut visit.steps, NativeCompilationSteps::Unknown)
        else {
            self.require_provider();
            return None;
        };
        let failure = self.visit_compilation_steps(
            words,
            offset,
            steps,
            context,
            (visit.selection, visit.spec),
        )?;
        if matches!(visit.selection, NativeCompilationSelection::Inline { .. }) {
            return self.with_command_dependency(
                Some(failure),
                &visit.target,
                &visit.head,
                context,
            );
        }
        Some(failure)
    }

    #[inline(never)]
    fn with_command_dependency(
        &self,
        mut failure: Option<SourceNativeCompilationFailure>,
        target: &super::SourceCommandTarget,
        head: &str,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        if let Some(failure) = &mut failure {
            failure
                .dependencies
                .push(self.command_dependency(target, head, context));
        }
        failure
    }

    fn retain_command_head_dependency(
        &mut self,
        target: &super::SourceCommandTarget,
        head: &str,
        context: SourceExecutionContext<'_>,
    ) {
        let dependency = self.command_dependency(target, head, context);
        if dependency.compiler_prerequisite.is_some()
            || (target.registry_backed && target.prepended.is_empty())
        {
            self.retain_prefix_dependency(dependency);
        }
    }

    fn incomplete_compiler_preparation(
        &self,
        selection: NativeCompilationSelection,
        spec: tcl_registry::native_compilation::NativeCompilationSpec,
        preparation: Option<&crate::registry_invocation::OriginalNativeCompilerPreparation>,
    ) -> bool {
        selection == NativeCompilationSelection::Generic
            && self
                .state
                .baseline
                .compilation_dialect()
                .is_none_or(|dialect| spec.compiler_hook_presence(dialect) != Some(false))
            && preparation.is_none_or(|preparation| {
                preparation.structured().is_none() && preparation.namespace_bindings().is_none()
            })
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
            context.realm(),
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
    ) -> Option<CommandVisit> {
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
            return Some(CommandVisit::Substitutions);
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
                    context: context.compilation(),
                    operand_from: 2,
                },
                context.registry,
            )),
        )?;
        match plan {
            super::ensemble_compilation::ActualEnsemblePlan::Operation { recipe, .. } => {
                Some(CommandVisit::Ensemble(recipe))
            }
            super::ensemble_compilation::ActualEnsemblePlan::Generic => {
                // A public generic fallback does not retain whether a worker
                // hook attempted local allocations before declining.
                self.locals = None;
                Some(CommandVisit::Substitutions)
            }
            super::ensemble_compilation::ActualEnsemblePlan::Named { preparations, .. } => {
                Some(CommandVisit::Ensemble(Box::new(
                    tcl_registry::native_instruction_plan::NativeInstructionPlan::GenericPreparation(
                        preparations,
                    ),
                )))
            }
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
                (!target.registry_backed).then_some(CommandVisit::Substitutions)
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
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
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
                        compilation: context.compilation(),
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
                NativeNamespaceBindingVisit::DeclareLocal(name) => {
                    self.declare_source_local(Some(name));
                }
                NativeNamespaceBindingVisit::Literal(_)
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
                    self.locals = None;
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
            .map(|word| original.get(word.original_written_span(original)?.as_range()))
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
        segment: &tcl_source::segment::SegmentedCommand,
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
            let end = c84_command_size_end(source.as_bytes(), end as usize);
            source.get(start as usize..end).map(str::to_owned)
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
        dialect: tcl_native_model::InvocationDialect,
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
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
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
                context.compilation(),
                selection,
                super::compiled_invocation::word_shape(&words[body.argument + 1]),
            );
            let Some(compilation) = body.entered_context(compilation) else {
                continue;
            };
            // A script the release evaluates directly (C8.4's `eval`) is not
            // compiled: its commands fail as they run.
            if compilation.mode != NativeCompilationMode::BytecodeObject {
                continue;
            }
            let failure = self.script(
                &tcl_lexer::SourceImage::native(text.as_bytes()),
                source.span.start().saturating_add(1),
                SourceExecutionContext {
                    control: (context)
                        .control
                        .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript)
                        .with_compilation(compilation),
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
                    WordPart::Variable { source, .. } => {
                        self.source_local_substitution(word, source, context);
                        self.variable_indices(word, source, context)
                    }
                    _ => None,
                }),
                WordExpr::Expand { word, .. } => {
                    self.substitutions(std::slice::from_ref(word), context)
                }
                WordExpr::Variable { source, .. } => {
                    self.source_local_substitution(word, source, context);
                    self.variable_indices(word, source, context)
                }
                _ => None,
            };
            if failure.is_some() {
                return failure;
            }
        }
        None
    }

    fn source_local_substitution(
        &mut self,
        word: &WordExpr,
        site: &tcl_source::words::SourceSite,
        context: SourceExecutionContext<'_>,
    ) {
        if self.locals.is_none() {
            return;
        }
        let root = (|| {
            let original =
                super::original_name_value::original_native_word(word, self.state, context.config)?;
            let policy = self
                .state
                .source_variables
                .execution_name_policy?
                .native_recipe()?;
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
                &original,
                site.span,
                tcl_syntax::word_rules::WordValueRules::from_config(&context.config),
                policy,
            )
        })();
        let Some(root) = root else {
            self.locals = None;
            return;
        };
        if self.locals.as_mut().is_some_and(|locals| {
            !locals.substitution_name(root.bytes(), root.is_separate_array_root())
        }) {
            self.locals = None;
        }
    }

    fn declare_source_local(&mut self, name: Option<&[u8]>) {
        if self
            .locals
            .as_mut()
            .is_some_and(|locals| !locals.declare(name))
        {
            self.locals = None;
        }
    }

    fn source_local_command(&mut self, name: &[u8]) {
        if self
            .locals
            .as_mut()
            .is_some_and(|locals| !locals.command_name(name))
        {
            self.locals = None;
        }
    }

    fn variable_indices(
        &mut self,
        word: &WordExpr,
        site: &tcl_source::words::SourceSite,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let Some(original) =
            super::original_name_value::original_native_word(word, self.state, context.config)
        else {
            self.require_provider();
            return None;
        };
        let arena = original.executable_parts();
        let Some(variable) = arena.all_parts().find(|part| {
            matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
                && (part.span == site.span || arena.source_span(part) == Some(site.span))
        }) else {
            self.require_provider();
            return None;
        };
        let tcl_lexer::ExecutablePart::Variable {
            index: Some(index), ..
        } = variable.part
        else {
            return None;
        };
        let mut pending = vec![(index, 0, context.depth)];
        while let Some((list, next, depth)) = pending.pop() {
            if tcl_source::limits::MAX_SOURCE_NEST_DEPTH.exceeded(depth) {
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
                    if let Some(failure) = self.script(
                        &tcl_lexer::SourceImage::from_bytes(script, arena.image().channel()),
                        body.start(),
                        SourceExecutionContext {
                            control: (context).control.with_source_purpose(
                                tcl_lexer::SourceSegmentationPurpose::NestedScript,
                            ),
                            depth: depth + 1,
                            ..context
                        },
                    ) {
                        return Some(failure);
                    }
                }
                tcl_lexer::ExecutablePart::Variable { index, .. } => {
                    let root = self.state.source_variables.execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        .and_then(|policy| crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                            arena, arena.image(), context.config, component.span,
                            tcl_syntax::word_rules::WordValueRules::from_config(&context.config), policy,
                        ));
                    if let Some(root) = root {
                        if self.locals.as_mut().is_some_and(|locals| {
                            !locals.substitution_name(root.bytes(), root.is_separate_array_root())
                        }) {
                            self.locals = None;
                        }
                    } else {
                        self.locals = None;
                    }
                    if let Some(index) = index {
                        pending.push((index, 0, depth + 1));
                    }
                }
                tcl_lexer::ExecutablePart::Expression { .. }
                | tcl_lexer::ExecutablePart::ParseError(_) => self.require_provider(),
                tcl_lexer::ExecutablePart::Text(_) => {}
            }
        }
        None
    }

    fn brackets(
        &mut self,
        spelling: &str,
        source: &tcl_source::words::SourceSite,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceNativeCompilationFailure> {
        let script = spelling.strip_prefix('[')?.strip_suffix(']')?;
        self.script(
            &tcl_lexer::SourceImage::native(script.as_bytes()),
            source.span.start().saturating_add(1),
            SourceExecutionContext {
                control: (context)
                    .control
                    .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
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
                    context: context.compilation(),
                    operand_from: facts.argument_offset + 1,
                },
            )
        });
    // As the registered compiler's own fallback: a named ensemble compiler
    // selects from the retained original words once their selection closes.
    let (selected, recipe) = original.unwrap_or_else(|| {
        let selection = if super::compiled_invocation::original_named_selection_is_closed(
            words, offset, facts, state, context,
        ) {
            spec.select_for_original_facts(
                invocation,
                &shapes,
                facts,
                invocation.arguments().dialect(),
                context.compilation(),
            )
        } else {
            spec.select_for_facts(
                invocation,
                &shapes,
                facts,
                invocation.arguments().dialect(),
                context.compilation(),
            )
        };
        (selection, None)
    });
    (shapes, selected, recipe)
}

/// Eligibility of the original head syntax, before any runtime name lookup.
fn head_compilation_capability(
    words: &[WordExpr],
    dialect: Option<tcl_native_model::InvocationDialect>,
) -> Option<bool> {
    words
        .first()
        .map(super::compiled_invocation::word_shape)?
        .compiler_head(dialect)
}

/// Where C8.4's `Tcl_ParseCommand` ends a command's size after its last word:
/// past the white space (backslash-newlines included) that runs to its
/// terminator, which `TclCompileScript` then leaves out, or to the script end.
fn c84_command_size_end(source: &[u8], mut end: usize) -> usize {
    loop {
        match source.get(end..) {
            Some([b' ' | b'\t' | b'\r' | 0x0b | 0x0c, ..]) => end += 1,
            Some([b'\\', b'\n', ..]) => end += 2,
            _ => return end,
        }
    }
}

#[cfg(test)]
mod tests;
