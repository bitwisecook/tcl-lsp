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

//! Native script-object admission and pre-argv operation selection.

#[cfg(test)]
use super::drop_fresh;
use super::{
    obj, obj_bytes, parse, BuiltinFn, Cell, Code, Command, CommandGenerationLookup, Interp, NsId,
    Rc, RefCell, TclObj,
};
use std::collections::HashMap;
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationSelection as Selection, NativeCompilationSpec,
    NativeCompilationWordShape as Shape, NativeCompiledBodies,
};
use tcl_registry::{ArgRole, InvocationFacts, InvocationWord, InvocationWords};
use tcl_runtime_api::{NativeCompilationAdmissionError, NativeCompilationError};

#[cfg(test)]
#[path = "native_compilation/jim_frame_tests.rs"]
mod jim_frame_tests;

mod procedure_frames;
mod refusals;
mod script_compilation;

pub(super) use procedure_frames::{
    formal_parameters, native_parameter_plan, NativeProcedureAdmission,
};
#[cfg(debug_assertions)]
use refusals::trace_retained_native_refusal;

#[derive(Clone)]
pub(super) struct SelectedInvocation {
    identity: Vec<u8>,
    handler: Option<BuiltinFn>,
    facts: InvocationFacts,
    spec: NativeCompilationSpec,
    selection: Selection,
    shapes: Vec<Shape>,
    context: Context,
    body_contexts: HashMap<usize, Context>,
}

#[derive(Default)]
pub(super) struct CompilationState {
    /// Only engine installation, before host registration, seals these tokens.
    pub(super) stock: HashMap<u64, Vec<u8>>,
    pub(super) stock_registrations: HashMap<u64, super::stock_ensembles::StockRegistrationSlot>,
    fixed_math: Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable>,
    execution: CompilationExecution,
    admission_error: Option<NativeCompilationAdmissionError>,
    execution_refusal: Option<tcl_runtime_api::NativeExecutionError>,
    native_access_refusal: Option<tcl_syntax::raw_string::NativeValueAccessRefusal>,
    host_command_refusal: Option<Box<tcl_runtime_api::NativeHostCommandRefusal>>,
    /// Pure selection results, shared across every dispatch of the same words.
    selection_cores: HashMap<SelectionKey, Option<Rc<SelectionCore>>>,
}

const MAX_SELECTION_CORES: usize = 4096;

/// Everything a native selection's registry facts, spec choice and body
/// contexts depend on. Command lookup, traces and worker replacement are
/// decided live by each dispatch; `generic` records their verdict.
#[derive(Clone, PartialEq, Eq, Hash)]
struct SelectionKey {
    dialect: tcl_native_model::InvocationDialect,
    registry: usize,
    spec: NativeCompilationSpec,
    identity: Box<str>,
    arguments: Box<[SelectionWord]>,
    shapes: Box<[Shape]>,
    context: Context,
    generic: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum SelectionWord {
    Expanded,
    Literal(Box<[u8]>),
    Substituted,
}

impl SelectionWord {
    fn of(word: &parse::Word<'_>) -> Self {
        if word.expand {
            return Self::Expanded;
        }
        match &word.body {
            parse::WordBody::Literal(value) => Self::Literal(value.as_ref().into()),
            parse::WordBody::Parts(_) => Self::Substituted,
        }
    }
}

struct SelectionCore {
    facts: InvocationFacts,
    selection: Selection,
    body_contexts: HashMap<usize, Context>,
}

fn select_core(
    registry: &tcl_registry::CommandRegistry,
    key: &SelectionKey,
    arguments: &[InvocationWord<'_>],
    shapes: &[Shape],
) -> Option<SelectionCore> {
    let SelectionKey {
        dialect,
        spec,
        identity,
        context,
        generic,
        ..
    } = key;
    let words = InvocationWords::structured(InvocationWord::Literal(identity), arguments)
        .with_dialect(*dialect);
    let facts = registry
        .resolve_structured_invocation(words, None)
        .resolved()?
        .facts();
    let selection = if *generic {
        Selection::Generic
    } else {
        spec.select(words, shapes, Some(*dialect), *context)
    };
    let mut body_contexts = HashMap::new();
    if let NativeCompiledBodies::Known(bodies) =
        spec.compiled_bodies(selection, words, shapes, &facts, Some(*dialect))
    {
        for body in bodies {
            let enclosing = spec.body_context_for_operand(
                Some(*dialect),
                *context,
                selection,
                shapes[body.argument],
            );
            if let Some(entered) = body.entered_context(enclosing) {
                body_contexts.insert(body.argument, entered);
            }
        }
    }
    Some(SelectionCore {
        facts,
        selection,
        body_contexts,
    })
}

#[derive(Default)]
pub(super) struct CompilationExecution {
    scripts: Vec<ScriptActivation>,
    invocations: Vec<ActiveInvocation>,
    builtin_activations: Vec<NativeBuiltinActivation>,
    evaluating_arguments: usize,
    pending: Vec<Option<ScriptActivation>>,
}

impl std::ops::Deref for CompilationState {
    type Target = CompilationExecution;
    fn deref(&self) -> &Self::Target {
        &self.execution
    }
}
impl std::ops::DerefMut for CompilationState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.execution
    }
}
/// Compilation state whose every mutable borrow republishes, on release, the
/// shared refusal flag that operation currencies read without borrowing.
pub(super) struct CompilationCell {
    state: RefCell<CompilationState>,
    refused: Rc<Cell<bool>>,
}

impl Default for CompilationCell {
    fn default() -> Self {
        Self {
            state: RefCell::new(CompilationState::default()),
            refused: Rc::new(Cell::new(false)),
        }
    }
}

impl CompilationCell {
    pub(super) fn borrow(&self) -> std::cell::Ref<'_, CompilationState> {
        self.state.borrow()
    }

    #[cfg(test)]
    pub(super) fn try_borrow(
        &self,
    ) -> Result<std::cell::Ref<'_, CompilationState>, std::cell::BorrowError> {
        self.state.try_borrow()
    }

    pub(super) fn borrow_mut(&self) -> CompilationGuard<'_> {
        CompilationGuard {
            state: self.state.borrow_mut(),
            refused: &self.refused,
        }
    }

    /// The retained-refusal flag, current whenever no mutable borrow is live.
    pub(super) fn refused_flag(&self) -> &Rc<Cell<bool>> {
        &self.refused
    }
}

pub(super) struct CompilationGuard<'a> {
    state: std::cell::RefMut<'a, CompilationState>,
    refused: &'a Cell<bool>,
}

impl std::ops::Deref for CompilationGuard<'_> {
    type Target = CompilationState;
    fn deref(&self) -> &CompilationState {
        &self.state
    }
}

impl std::ops::DerefMut for CompilationGuard<'_> {
    fn deref_mut(&mut self) -> &mut CompilationState {
        &mut self.state
    }
}

impl Drop for CompilationGuard<'_> {
    fn drop(&mut self) {
        self.refused.set(self.state.refused());
    }
}

impl CompilationState {
    /// Whether any execution refusal cause is retained.
    pub(super) fn refused(&self) -> bool {
        self.execution_refusal.is_some()
            || self.admission_error.is_some()
            || self.native_access_refusal.is_some()
            || self.host_command_refusal.is_some()
    }

    pub(super) fn swap_execution(&mut self, other: &mut CompilationExecution) {
        std::mem::swap(&mut self.execution, other);
    }
}

struct NativeBuiltinActivation {
    #[cfg_attr(not(have_tommath), allow(dead_code))]
    identity: Option<Vec<u8>>,
    // Actual dispatch selection only; a direct compiled helper has no token.
    generation: Option<u64>,
}

#[derive(Clone)]
struct ScriptActivation {
    context: Context,
    chunk: Rc<CompiledChunk>,
}

#[derive(Default)]
struct CompiledChunk {
    scripts: HashMap<(Vec<u8>, Context), ScriptCommandSelections>,
    unpresented_failure: bool,
}

pub(super) enum NativeCompilerAdmissionFailure {
    Presented(Box<NativeCompilationError>),
    Unpresented,
}

type ScriptCommandSelections = Vec<Option<Box<SelectedInvocation>>>;

#[derive(Clone, Copy)]
struct CompilerEntry {
    namespace: NsId,
    context: Context,
}

struct ActiveInvocation {
    selected: Box<SelectedInvocation>,
    // Borrowed from the live invocation argv; the handler and any suspended
    // coroutine retain argv until this activation leaves.
    arguments: Vec<*mut TclObj>,
}

impl Interp {
    pub(crate) fn reset_native_compilation_admission(&self) {
        let mut state = self.native_compilation.borrow_mut();
        state.execution_refusal = None;
        state.admission_error = None;
        state.native_access_refusal = None;
        state.host_command_refusal = None;
    }

    pub(super) fn seal_native_compiler_tokens(&self) {
        for (generation, identity) in self.registry_builtin_names.borrow().clone() {
            self.record_stock_compiler_registration(generation, &identity);
        }
        self.seal_native_compiler_attachments();
        self.refresh_native_math_function_table();
    }

    pub(super) fn seal_native_compiler_attachments(&self) {
        let dialect = self.native_invocation_dialect();
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        let stock = self.native_compilation.borrow().stock.clone();
        for (generation, identity) in stock {
            let spec = core::str::from_utf8(&identity).ok().and_then(|identity| {
                registry.native_compilation_for_registration(identity, dialect)
            });
            let recipe = match spec.and_then(|spec| spec.compiler_hook_presence(dialect)) {
                Some(true) => crate::namespace::NativeCompilerRecipe::Registered {
                    registration: identity,
                    spec: spec.expect("selected compiler descriptor"),
                },
                Some(false) => crate::namespace::NativeCompilerRecipe::Absent,
                None => crate::namespace::NativeCompilerRecipe::Unknown,
            };
            let handler = match self.raw_command_by_generation(generation) {
                Some(Command::Builtin(handler)) => Some(handler),
                Some(Command::Ensemble(_)) => self.stock_ensemble_compiler(generation),
                _ => None,
            };
            let mut namespaces = self.namespaces.borrow_mut();
            namespaces.set_native_compiler_recipe(generation, recipe);
            namespaces.set_native_compiler_handler(generation, handler);
        }
    }

    /// Actual independent compiler and resolver epochs for this namespace token.
    /// A logical platform profile cannot supply an unavailable native C stamp.
    pub fn native_compiler_cache_epochs(&self, namespace: NsId) -> Option<(u64, u64)> {
        let dialect = self.native_invocation_dialect();
        if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect.tcl_version.is_none()
        {
            return None;
        }
        self.namespaces
            .borrow()
            .native_compiler_cache_epochs(namespace)
    }

    pub(super) fn refresh_native_math_function_table(&self) {
        let surface =
            tcl_registry::expr_surface::RuntimeExprSurface::for_profile(self.dialect_profile());
        // This engine implements the fixed-function dispatcher from the shared
        // implementation table. Host command registration cannot replace it.
        let fixed_math = (!surface.has_math_function_command_table()).then(|| {
            let functions = surface
                .builtin_math_function_names()
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    let arity = tcl_syntax::expr::mathfunc::spec(name).and_then(|implementation| {
                        (implementation.arity.max == Some(implementation.arity.min))
                            .then_some(usize::from(implementation.arity.min))
                    });
                    tcl_runtime_api::native_compilation::NativeMathFunctionBinding {
                        name: name.into(),
                        token: index as u64 + 1,
                        implementation_generation: 1,
                        registry_identity: arity
                            .map(|_| tcl_registry::mathfunc::qualified_name(name)),
                        arity,
                    }
                })
                .collect();
            tcl_runtime_api::native_compilation::NativeMathFunctionTable {
                closed: true,
                generation: 1,
                functions,
            }
        });
        self.native_compilation.borrow_mut().fixed_math = fixed_math;
    }

    /// Retain the actual immutable fixed-function registrations implemented by
    /// this interpreter. This is independent of mutable Tcl command bindings.
    pub fn native_math_function_table(
        &self,
    ) -> Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable> {
        self.native_compilation.borrow().fixed_math.clone()
    }

    /// Complete the admitted native configuration setter transaction on the
    /// original ensemble token. Imports identify the origin's configuration,
    /// while compiler invalidation uses that origin token's raw attachment.
    pub(crate) fn note_native_ensemble_configuration_changed(
        &self,
        token: &Rc<crate::ensemble::EnsembleToken>,
    ) {
        self.namespaces
            .borrow_mut()
            .advance_ensemble_export_epoch(token.config().ns);
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        let Some(count) = tcl_native_model::native_ensemble::configuration_compiler_mutations(
            self.native_invocation_dialect(),
        ) else {
            return;
        };
        let generations = self.namespaces.borrow().native_command_generations();
        let hook = generations
            .into_iter()
            .find_map(
                |generation| match self.raw_command_by_generation(generation) {
                    Some(Command::Ensemble(original)) if Rc::ptr_eq(&original, token) => {
                        self.namespaces.borrow().native_compiler_hook(generation)
                    }
                    _ => None,
                },
            )
            .unwrap_or(Hook::Unknown);
        let mut namespaces = self.namespaces.borrow_mut();
        for _ in 0..count {
            namespaces.note_native_compiler_mutation(None,
                tcl_registry::native_procedure::NativeCompilerCacheMutation::EnsembleConfiguration { hook });
        }
    }

    pub(super) fn native_compiler_target(
        &self,
        namespace: NsId,
        head: &[u8],
    ) -> Option<(Vec<u8>, BuiltinFn, u64, u64)> {
        let mut generation = self
            .namespaces
            .borrow()
            .resolve_generation(namespace, head)?;
        let lookup_generation = generation;
        // Imports retain a command token. Aliases retain an argv trampoline and
        // deliberately have no compiler hook of their target.
        let mut seen = std::collections::HashSet::new();
        loop {
            if !seen.insert(generation) {
                return None;
            }
            match self.command_by_generation(generation) {
                CommandGenerationLookup::Found {
                    command: Command::Builtin(handler),
                    ..
                } => {
                    let identity = self
                        .native_compilation
                        .borrow()
                        .stock
                        .get(&generation)?
                        .clone();
                    return self
                        .command_visible_for_surface_at(
                            &Command::Builtin(handler),
                            Some(generation),
                        )
                        .then_some((identity, handler, generation, lookup_generation));
                }
                CommandGenerationLookup::Found {
                    command: Command::Ensemble(_),
                    ..
                } => {
                    let handler = self.stock_ensemble_compiler(generation)?;
                    let identity = self
                        .native_compilation
                        .borrow()
                        .stock
                        .get(&generation)?
                        .clone();
                    return Some((identity, handler, generation, lookup_generation));
                }
                CommandGenerationLookup::Found {
                    command:
                        Command::Imported {
                            source_generation,
                            ensemble: None,
                            ..
                        },
                    ..
                } => generation = source_generation,
                _ => return None,
            }
        }
    }

    fn select_native_invocation(
        &self,
        command: &parse::Command<'_>,
        namespace: NsId,
        context: Context,
    ) -> Option<Box<SelectedInvocation>> {
        let first = command.words.first()?;
        if first.expand {
            return None;
        }
        let parse::WordBody::Literal(head) = &first.body else {
            return None;
        };
        let (_, lookup_generation) = self.resolve_dispatchable_with_generation(namespace, head)?;
        let lookup_generation = lookup_generation?;
        let (identity, spec, handler) = {
            let namespaces = self.namespaces.borrow();
            match namespaces.native_compiler_recipe(lookup_generation)? {
                crate::namespace::NativeCompilerRecipe::Registered { registration, spec } => (
                    registration,
                    spec,
                    namespaces.native_compiler_handler(lookup_generation),
                ),
                // Absence and native NoOp both genuinely emit ordinary invoke.
                crate::namespace::NativeCompilerRecipe::Absent
                | crate::namespace::NativeCompilerRecipe::ProcedureNoOp
                | crate::namespace::NativeCompilerRecipe::Unknown => return None,
            }
        };
        let generation = self
            .native_compiler_target(namespace, head)
            .map(|(_, _, generation, _)| generation)
            .unwrap_or(lookup_generation);
        let identity = std::str::from_utf8(&identity).ok()?;
        let observed = {
            let traces = self.traces.borrow();
            !traces.step_active.is_empty()
                || traces.cmd_traces.iter().any(|trace| {
                    trace.token == Some(lookup_generation)
                        && trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0
                })
        };
        let dialect = self.native_invocation_dialect();
        let prerequisites = spec.implementation_prerequisites(dialect);
        let replaced_worker = prerequisites
            .as_ref()
            .is_some_and(|path| !self.native_implementation_path_holds(generation, path));
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        let shapes: Vec<_> = command.words[1..].iter().map(source_shape).collect();
        let key = SelectionKey {
            dialect,
            registry: std::ptr::from_ref(registry).addr(),
            spec,
            identity: identity.into(),
            arguments: command.words[1..].iter().map(SelectionWord::of).collect(),
            shapes: shapes.clone().into(),
            context,
            generic: observed || replaced_worker,
        };
        let cached = self
            .native_compilation
            .borrow()
            .selection_cores
            .get(&key)
            .cloned();
        let core = match cached {
            Some(core) => core,
            None => {
                let arguments: Vec<_> = command.words[1..].iter().map(source_word).collect();
                let core = select_core(registry, &key, &arguments, &shapes).map(Rc::new);
                let mut state = self.native_compilation.borrow_mut();
                if state.selection_cores.len() >= MAX_SELECTION_CORES {
                    state.selection_cores.clear();
                }
                state.selection_cores.insert(key, core.clone());
                core
            }
        }?;
        Some(Box::new(SelectedInvocation {
            identity: identity.as_bytes().to_vec(),
            body_contexts: core.body_contexts.clone(),
            handler: matches!(core.selection, Selection::Inline { .. })
                .then_some(handler)
                .flatten(),
            facts: core.facts.clone(),
            spec,
            selection: core.selection,
            shapes,
            context,
        }))
    }

    fn native_script_entry(&self, source: &[u8]) -> ScriptActivation {
        let mut state = self.native_compilation.borrow_mut();
        if let Some(pending) = state.pending.last_mut().and_then(Option::take) {
            return pending;
        }
        let enclosing = state.scripts.last().cloned();
        if state.evaluating_arguments > 0 {
            return enclosing.unwrap_or_else(direct_activation);
        }
        if self.traces.borrow().exec_firing > 0 {
            return direct_activation();
        }
        if let Some(invocation) = state.invocations.last() {
            let selected = &invocation.selected;
            let matching = selected.facts.arg_roles.iter().find_map(|(index, role)| {
                let argument = selected.facts.argument_offset + usize::from(*index);
                (*role == ArgRole::Body
                    && invocation.arguments.get(argument).is_some_and(|&value| {
                        obj::has_string_rep(value) && obj_bytes(value) == source
                    }))
                .then_some(argument)
            });
            if let Some(argument) = matching {
                let context = selected
                    .body_contexts
                    .get(&argument)
                    .copied()
                    .unwrap_or_else(|| {
                        selected.spec.body_context_for_operand(
                            Some(self.native_invocation_dialect()),
                            selected.context,
                            selected.selection,
                            selected
                                .shapes
                                .get(argument)
                                .copied()
                                .unwrap_or(Shape::Opaque),
                        )
                    });
                if let Some(enclosing) = enclosing {
                    if enclosing
                        .chunk
                        .scripts
                        .contains_key(&(source.to_vec(), context))
                    {
                        return ScriptActivation {
                            context,
                            chunk: enclosing.chunk,
                        };
                    }
                }
                return ScriptActivation {
                    context,
                    chunk: Rc::new(CompiledChunk::default()),
                };
            }
        }
        direct_activation()
    }

    pub(super) fn enter_native_script(
        &self,
        source: &[u8],
    ) -> Result<(), NativeCompilerAdmissionFailure> {
        let mut activation = self.native_script_entry(source);
        if self.native_invocation_dialect().tcl_version == Some(tcl_dialect::TclVersion::V8_4)
            && activation.context.mode == NativeCompilationMode::BytecodeObject
            && !activation
                .chunk
                .scripts
                .contains_key(&(source.to_vec(), activation.context))
        {
            let mut chunk = CompiledChunk::default();
            if let Some(failure) = self.compile_native_script(
                source,
                self.current_ns.get(),
                activation.context,
                &mut chunk,
                0,
            ) {
                return Err(NativeCompilerAdmissionFailure::Presented(Box::new(failure)));
            }
            if chunk.unpresented_failure {
                return Err(NativeCompilerAdmissionFailure::Unpresented);
            }
            activation.chunk = Rc::new(chunk);
        }
        self.native_compilation
            .borrow_mut()
            .scripts
            .push(activation);
        Ok(())
    }

    pub(super) fn leave_native_script(&self) {
        self.native_compilation.borrow_mut().scripts.pop();
    }

    pub(super) fn native_command_selection(
        &self,
        source: &[u8],
        command: &parse::Command<'_>,
    ) -> Option<Box<SelectedInvocation>> {
        let state = self.native_compilation.borrow();
        let activation = state.scripts.last()?;
        if self.native_invocation_dialect().tcl_version == Some(tcl_dialect::TclVersion::V8_4) {
            let commands = parse::parse_script_with_config(source, self.lexer_config());
            let index = commands
                .iter()
                .position(|candidate| candidate.start == command.start)?;
            return activation
                .chunk
                .scripts
                .get(&(source.to_vec(), activation.context))?
                .get(index)?
                .clone();
        }
        let context = activation.context;
        drop(state);
        self.select_native_invocation(command, self.current_ns.get(), context)
    }

    pub(super) fn begin_native_arguments(&self) {
        self.native_compilation.borrow_mut().evaluating_arguments += 1;
    }
    pub(super) fn end_native_arguments(&self) {
        self.native_compilation.borrow_mut().evaluating_arguments -= 1;
    }

    pub(super) fn enter_native_builtin(
        &self,
        generation: Option<u64>,
        argv: &[*mut TclObj],
    ) -> NativeBuiltinAdmission {
        let identity = generation.and_then(|generation| {
            self.native_compilation
                .borrow()
                .stock
                .get(&generation)
                .cloned()
        });
        self.native_compilation
            .borrow_mut()
            .builtin_activations
            .push(NativeBuiltinActivation {
                identity,
                generation,
            });
        let selected =
            generation.and_then(|generation| self.runtime_native_invocation(generation, argv));
        let body = selected.is_some_and(|selected| {
            if !selected
                .facts
                .arg_roles
                .iter()
                .any(|(_, role)| *role == ArgRole::Body)
            {
                return false;
            }
            self.native_compilation
                .borrow_mut()
                .invocations
                .push(ActiveInvocation {
                    selected,
                    arguments: argv[1..].to_vec(),
                });
            true
        });
        NativeBuiltinAdmission {
            interpreter: self.clone(),
            body,
        }
    }

    /// Stock handler identity retained through rename and alias dispatch.
    /// Absence is explicit for host handlers and direct unbound calls.
    #[cfg_attr(not(have_tommath), allow(dead_code))]
    pub(crate) fn active_native_builtin_identity(&self) -> Option<Vec<u8>> {
        self.native_compilation
            .borrow()
            .builtin_activations
            .last()
            .and_then(|activation| activation.identity.clone())
    }

    /// Actual selected builtin registration generation, independent of argv/report.
    pub(crate) fn active_builtin_command_generation(&self) -> Option<u64> {
        self.native_compilation
            .borrow()
            .builtin_activations
            .last()?
            .generation
    }

    fn runtime_native_invocation(
        &self,
        generation: u64,
        argv: &[*mut TclObj],
    ) -> Option<Box<SelectedInvocation>> {
        let state = self.native_compilation.borrow();
        let identity = state.stock.get(&generation)?;
        let identity = std::str::from_utf8(identity).ok()?;
        let context = state.scripts.last().map_or_else(
            || direct_activation().context,
            |activation| activation.context,
        );
        let values: Vec<_> = argv[1..]
            .iter()
            .map(|&argument| obj::has_string_rep(argument).then(|| obj_bytes(argument)))
            .collect();
        let arguments: Vec<_> = values
            .iter()
            .map(|value| {
                value
                    .as_ref()
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .map_or(InvocationWord::Dynamic, InvocationWord::Literal)
            })
            .collect();
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        let words = InvocationWords::structured(InvocationWord::Literal(identity), &arguments)
            .with_dialect(self.native_invocation_dialect());
        let facts = registry
            .resolve_structured_invocation(words, None)
            .resolved()?
            .facts();
        let spec = facts.native_compilation?;
        Some(Box::new(SelectedInvocation {
            identity: identity.as_bytes().to_vec(),
            handler: None,
            facts,
            spec,
            selection: Selection::Generic,
            shapes: vec![Shape::Substituted; arguments.len()],
            context,
            body_contexts: HashMap::new(),
        }))
    }

    /// Native protocol selected for the currently executing builtin invocation.
    pub(crate) fn active_native_compilation_selection(&self) -> Selection {
        self.native_compilation
            .borrow()
            .invocations
            .last()
            .map_or(Selection::Generic, |invocation| {
                invocation.selected.selection
            })
    }

    pub(super) fn dispatch_native_selection(
        &mut self,
        selected: Option<Box<SelectedInvocation>>,
        argv: &[*mut TclObj],
    ) -> Code {
        let Some(selected) = selected else {
            return self.dispatch(argv);
        };
        if let Selection::NamedInvocation {
            lookup,
            arguments_from,
            protocol,
        } = selected.selection
        {
            return self.dispatch_native_named_invocation(
                lookup.slot.as_bytes(),
                arguments_from,
                protocol,
                argv,
            );
        }
        let Some(handler) = selected.handler else {
            return self.dispatch(argv);
        };
        self.native_compilation
            .borrow_mut()
            .builtin_activations
            .push(NativeBuiltinActivation {
                identity: Some(selected.identity.clone()),
                generation: None,
            });
        self.native_compilation
            .borrow_mut()
            .invocations
            .push(ActiveInvocation {
                selected,
                arguments: argv[1..].to_vec(),
            });
        let _admission = NativeBuiltinAdmission {
            interpreter: self.clone(),
            body: true,
        };
        self.set_result_bytes(b"");
        self.cmd_count.set(self.cmd_count.get() + 1);
        handler(self, argv)
    }

    fn dispatch_native_named_invocation(
        &mut self,
        slot: &[u8],
        arguments_from: usize,
        protocol: tcl_registry::native_compilation::NativeNamedInvocationProtocol,
        argv: &[*mut TclObj],
    ) -> Code {
        let originals = match super::native_arguments::OriginalArguments::enter(self, argv) {
            Ok(originals) => originals,
            Err(error) => return self.report_cmd_error(error),
        };
        let code = (|| {
            let Some(arguments) = argv.get(arguments_from + 1..) else {
                return self.dispatch(argv);
            };
            let head = super::new_string(slot);
            // SAFETY: this invocation owns the freshly allocated fixed-name value.
            unsafe { crate::obj::incr_ref_count(head) };
            let mut target = vec![head];
            target.extend_from_slice(arguments);
            let rewrites = protocol
                == tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite;
            let is_root = if rewrites {
                self.begin_ensemble_rewrite(
                    argv.iter()
                        .map(|word| crate::obj::Owned::retain(*word))
                        .collect(),
                    arguments_from + 1,
                    1,
                )
            } else {
                false
            };
            let code = if rewrites {
                self.dispatch_invoke(&target)
            } else {
                self.dispatch(&target)
            };
            let cleared = if is_root {
                self.clear_ensemble_rewrite_checked()
            } else {
                Ok(())
            };
            super::release_all(&[head]);
            if let Err(first) = cleared {
                self.refuse_native_execution(first);
            }
            code
        })();
        originals.finish(code)
    }

    pub(super) fn admit_native_compilation_error(
        &mut self,
        failure: &NativeCompilerAdmissionFailure,
        procedure: Option<&[u8]>,
    ) -> Code {
        let NativeCompilerAdmissionFailure::Presented(failure) = failure else {
            self.native_compilation.borrow_mut().admission_error =
                Some(NativeCompilationAdmissionError::NativePreflightRequired);
            #[cfg(debug_assertions)]
            trace_retained_native_refusal(
                "admission_error",
                &self.native_compilation.borrow().admission_error,
            );
            return self.error(b"native compiler preflight provider required");
        };
        let procedure = procedure.and_then(|name| std::str::from_utf8(name).ok());
        let info = failure.error_info_for_procedure(procedure);
        if let Some(error_code) = &failure.error_code {
            self.error_with_code(failure.message.as_bytes(), error_code.as_bytes());
        } else {
            self.error(failure.message.as_bytes());
        }
        let Some(info) = info else {
            return Code::Error;
        };
        self.exc.borrow_mut().info = Some(info.into_bytes());
        self.error_line.set(failure.body_line);
        Code::Error
    }
}

pub(super) struct NativeBuiltinAdmission {
    interpreter: Interp,
    body: bool,
}

impl Drop for NativeBuiltinAdmission {
    fn drop(&mut self) {
        let mut state = self.interpreter.native_compilation.borrow_mut();
        state.builtin_activations.pop();
        if self.body {
            state.invocations.pop();
        }
    }
}

fn direct_activation() -> ScriptActivation {
    ScriptActivation {
        context: Context {
            mode: NativeCompilationMode::Direct,
            frame: NativeCompilationFrame::ScriptCode,
            loop_depth: 0,
            catch_depth: Some(0),
        },
        chunk: Rc::new(CompiledChunk::default()),
    }
}

fn source_word<'a>(word: &'a parse::Word<'a>) -> InvocationWord<'a> {
    if word.expand {
        return InvocationWord::Expanded;
    }
    match &word.body {
        parse::WordBody::Literal(value) => {
            std::str::from_utf8(value).map_or(InvocationWord::Opaque, InvocationWord::Literal)
        }
        parse::WordBody::Parts(_) => InvocationWord::Dynamic,
    }
}

fn source_shape(word: &parse::Word<'_>) -> Shape {
    if word.expand {
        return Shape::Expanded;
    }
    match &word.body {
        parse::WordBody::Literal(_) if word.kind == parse::WordKind::Braced => Shape::BracedLiteral,
        parse::WordBody::Literal(_) => Shape::Literal,
        parse::WordBody::Parts(_) => Shape::Substituted,
    }
}

#[cfg(test)]
mod tests;
