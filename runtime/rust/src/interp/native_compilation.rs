// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native script-object admission and pre-argv operation selection.

use super::*;
use std::collections::HashMap;
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFailureScope, NativeCompilationFrame,
    NativeCompilationMode, NativeCompilationSelection as Selection, NativeCompilationSpec,
    NativeCompilationStep, NativeCompilationSteps, NativeCompilationWordShape as Shape,
    NativeCompiledBodies, NativeExpressionCompilerStep, NativeMathFunctionResolution,
};
use tcl_registry::{ArgRole, InvocationFacts, InvocationWord, InvocationWords};
use tcl_runtime_api::{
    NativeCompilationAdmissionError, NativeCompilationError, NativeCompilationErrorCommand,
};

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
    fixed_math: Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable>,
    execution: CompilationExecution,
    admission_error: Option<NativeCompilationAdmissionError>,
    native_access_refusal: Option<tcl_syntax::raw_string::NativeValueAccessRefusal>,
    host_command_refusal: Option<Box<tcl_runtime_api::NativeHostCommandRefusal>>,
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
impl CompilationState {
    pub(super) fn swap_execution(&mut self, other: &mut CompilationExecution) {
        std::mem::swap(&mut self.execution, other);
    }
}

struct NativeBuiltinActivation {
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
    /// Retained host admission error, distinct from a native Tcl completion.
    /// A failed preflight prevents script effects even if Tcl catches its result.
    pub fn native_compilation_admission_error(&self) -> Option<NativeCompilationAdmissionError> {
        self.native_compilation.borrow().admission_error
    }

    /// Reached Unicode access failure retained outside guest completion.
    pub fn unicode_access_refusal(&self) -> Option<tcl_syntax::raw_string::UnicodeAccessError> {
        match self.native_access_refusal() {
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(error)) => Some(error),
            _ => None,
        }
    }

    /// Reached native value operation refusal retained outside guest completion.
    pub fn native_access_refusal(
        &self,
    ) -> Option<tcl_syntax::raw_string::NativeValueAccessRefusal> {
        self.native_compilation.borrow().native_access_refusal
    }

    /// Any retained host-only execution failure; guest capture cannot consume it.
    pub fn host_refusal_pending(&self) -> bool {
        let state = self.native_compilation.borrow();
        state.admission_error.is_some()
            || state.native_access_refusal.is_some()
            || state.host_command_refusal.is_some()
    }

    /// Transport a reached child host failure without manufacturing guest options.
    /// Each interpreter retains its own actual state; no value or lookup identity
    /// crosses this channel, and an earlier parent refusal remains authoritative.
    pub(crate) fn transport_host_refusal_from(&mut self, origin: &Interp) -> super::Code {
        if self.host_refusal_pending() {
            return super::Code::Error;
        }
        let origin = origin.native_compilation.borrow();
        let mut target = self.native_compilation.borrow_mut();
        if target.admission_error.is_none() {
            target.admission_error = origin.admission_error;
        }
        if target.native_access_refusal.is_none() {
            target.native_access_refusal = origin.native_access_refusal;
        }
        if target.host_command_refusal.is_none() {
            target
                .host_command_refusal
                .clone_from(&origin.host_command_refusal);
        }
        super::Code::Error
    }

    /// Actual retained host cause; guest completion state never supplies it.
    pub fn native_execution_refusal(&self) -> Option<tcl_runtime_api::NativeExecutionError> {
        let state = self.native_compilation.borrow();
        if let Some(error) = state.admission_error {
            return Some(tcl_runtime_api::NativeExecutionError::CompilationAdmission(
                error,
            ));
        }
        if let Some(error) = state.native_access_refusal {
            return Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                error,
            ));
        }
        state
            .host_command_refusal
            .clone()
            .map(tcl_runtime_api::NativeExecutionError::HostCommandRefusal)
    }

    /// Original host callback metadata, without projecting its diagnostic text.
    pub fn native_host_command_refusal(&self) -> Option<tcl_runtime_api::NativeHostCommandRefusal> {
        self.native_compilation
            .borrow()
            .host_command_refusal
            .as_deref()
            .cloned()
    }

    /// A host callback's retained reporting text, independently of guest result bytes.
    pub fn host_command_refusal(&self) -> Option<String> {
        self.native_host_command_refusal()
            .map(|failure| failure.reason)
    }

    pub(crate) fn refuse_host_command(&mut self, reason: impl Into<String>) -> Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let namespace_token = self.current_ns();
        let namespace = self.namespaces().qualified_name(namespace_token);
        // This field is diagnostic display only; actual ownership remains the
        // retained interpreter and namespace token, including for opaque bytes.
        let namespace = std::str::from_utf8(&namespace)
            .map_or_else(|_| namespace.escape_ascii().to_string(), str::to_owned);
        let profile = self.dialect_profile().cache_key();
        let failure = tcl_runtime_api::NativeHostCommandRefusal {
            reason: reason.into(),
            source_profile: profile,
            native_profile: profile,
            interpreter: self.native_command_interpreter,
            frame: self.frames.borrow().current_level(),
            namespace: namespace.into(),
            namespace_token: namespace_token as u64,
        };
        self.native_compilation.borrow_mut().host_command_refusal = Some(Box::new(failure));
        Code::Error
    }

    pub(crate) fn refuse_unicode_access(
        &mut self,
        error: tcl_syntax::raw_string::UnicodeAccessError,
    ) -> crate::interp::Code {
        self.refuse_native_access(error.into())
    }

    pub(crate) fn refuse_native_access(
        &mut self,
        error: tcl_syntax::raw_string::NativeValueAccessRefusal,
    ) -> crate::interp::Code {
        if self.host_refusal_pending() {
            return Code::Error;
        }
        self.native_compilation
            .borrow_mut()
            .native_access_refusal
            .get_or_insert(error);
        crate::interp::Code::Error
    }

    pub(crate) fn reset_native_compilation_admission(&self) {
        let mut state = self.native_compilation.borrow_mut();
        state.admission_error = None;
        state.native_access_refusal = None;
        state.host_command_refusal = None;
    }

    pub(super) fn seal_native_compiler_tokens(&self) {
        self.native_compilation
            .borrow_mut()
            .stock
            .extend(self.registry_builtin_names.borrow().clone());
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
        let Some(count) = tcl_registry::native_ensemble::configuration_compiler_mutations(
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
                            &identity,
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
        let arguments: Vec<_> = command.words[1..].iter().map(source_word).collect();
        let shapes: Vec<_> = command.words[1..].iter().map(source_shape).collect();
        let words = InvocationWords::structured(InvocationWord::Literal(identity), &arguments)
            .with_dialect(self.native_invocation_dialect());
        let registry = crate::environment::store_for_profile(self.dialect_profile());
        let facts = registry
            .resolve_structured_invocation(words, None)
            .resolved()?
            .facts();
        let observed = {
            let traces = self.traces.borrow();
            !traces.step_active.is_empty()
                || traces.cmd_traces.iter().any(|trace| {
                    trace.token == Some(lookup_generation)
                        && trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0
                })
        };
        let prerequisites = spec.implementation_prerequisites(self.native_invocation_dialect());
        let replaced_worker = prerequisites.as_ref().is_some_and(|path| {
            !self.native_implementation_path_holds(generation, identity.as_bytes(), path)
        });
        let selection = if observed || replaced_worker {
            Selection::Generic
        } else {
            spec.select(
                words,
                &shapes,
                Some(self.native_invocation_dialect()),
                context,
            )
        };
        let mut body_contexts = HashMap::new();
        if let NativeCompiledBodies::Known(bodies) = spec.compiled_bodies(
            selection,
            words,
            &shapes,
            &facts,
            Some(self.native_invocation_dialect()),
        ) {
            for body in bodies {
                let enclosing = spec.body_context_for_operand(
                    Some(self.native_invocation_dialect()),
                    context,
                    selection,
                    shapes[body.argument],
                );
                if let Some(entered) = body.entered_context(enclosing) {
                    body_contexts.insert(body.argument, entered);
                }
            }
        }
        Some(Box::new(SelectedInvocation {
            identity: identity.as_bytes().to_vec(),
            body_contexts,
            handler: matches!(selection, Selection::Inline { .. })
                .then_some(handler)
                .flatten(),
            facts,
            spec,
            selection,
            shapes,
            context,
        }))
    }

    fn compile_native_script(
        &self,
        source: &[u8],
        namespace: NsId,
        context: Context,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return None;
        }
        let key = (source.to_vec(), context);
        if chunk.scripts.contains_key(&key) {
            return None;
        }
        let commands = parse::parse_script_with_config(source, self.lexer_config());
        let selections: Vec<_> = commands
            .iter()
            .map(|command| self.select_native_invocation(command, namespace, context))
            .collect();
        chunk.scripts.insert(key, selections.clone());
        for (index, (command, mut selected)) in commands.iter().zip(selections).enumerate() {
            if let Some(failure) = self.compile_native_command(
                source,
                command,
                selected.as_deref_mut(),
                CompilerEntry { namespace, context },
                chunk,
                depth,
            ) {
                return Some(failure);
            }
            if chunk.unpresented_failure {
                return None;
            }
            if let Some(entries) = chunk.scripts.get_mut(&(source.to_vec(), context)) {
                entries[index] = selected;
            }
        }
        None
    }

    fn compile_native_command(
        &self,
        source: &[u8],
        command: &parse::Command<'_>,
        selected: Option<&mut SelectedInvocation>,
        entry: CompilerEntry,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        let CompilerEntry { namespace, context } = entry;
        let context_note = || {
            Some(NativeCompilationErrorCommand {
                text: String::from_utf8(source[command.start..command.end].to_vec()).ok()?,
                line: line_of(source, command.start),
                before_context: Vec::new(),
                after_context: Vec::new(),
            })
        };
        let has_selected = selected.is_some();
        if let Some(selected) = selected {
            let arguments: Vec<_> = command.words[1..].iter().map(source_word).collect();
            let head = &selected.facts.canonical_command;
            let words = InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                .with_dialect(self.native_invocation_dialect());
            let spellings = command
                .words
                .iter()
                .map(|word| std::str::from_utf8(word.raw_source))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            if let Some(failure) = selected.spec.failure_for_selection_with_source(
                selected.selection,
                words,
                &selected.facts,
                Some(self.native_invocation_dialect()),
                command.words.first().and_then(|word| match &word.body {
                    parse::WordBody::Literal(value) => std::str::from_utf8(value).ok(),
                    _ => None,
                })?,
                &spellings[1..],
            ) {
                let Some(message) = failure.message else {
                    chunk.unpresented_failure = true;
                    return None;
                };
                return Some(NativeCompilationError {
                    message,
                    error_code: failure.error_code,
                    command_contexts: vec![context_note()?],
                    body_line: line_of(source, command.start),
                });
            }
            if let Some(failure) =
                self.compile_native_substitutions(source, command, entry, chunk, depth)
            {
                return Some(failure);
            }
            if chunk.unpresented_failure {
                return None;
            }
            if let NativeCompilationSteps::Known(steps) = selected.spec.compilation_steps(
                selected.selection,
                words,
                &selected.shapes,
                &selected.facts,
                Some(self.native_invocation_dialect()),
            ) {
                for step in steps {
                    let body = match step {
                        NativeCompilationStep::Body(body) => body,
                        NativeCompilationStep::Expression(expression) => {
                            let parse::WordBody::Literal(text) =
                                &command.words.get(expression.argument + 1)?.body
                            else {
                                continue;
                            };
                            let parse_context =
                                tcl_syntax::expr::parser::ExprParseContext::for_profile(
                                    self.dialect_profile(),
                                );
                            let text = std::str::from_utf8(text).ok()?;
                            let table = self.native_math_function_table();
                            let visits = expression.compiler_steps(text, &parse_context, |name| {
                                let Some(table) = &table else {
                                    return NativeMathFunctionResolution::Unknown;
                                };
                                match table.lookup(name) {
                                    tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row) => row.arity.map_or(NativeMathFunctionResolution::Unknown, |arity| NativeMathFunctionResolution::Known { arity }),
                                    tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Absent => NativeMathFunctionResolution::Absent,
                                    tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Unknown => NativeMathFunctionResolution::Unknown,
                                }
                            });
                            for visit in visits {
                                match visit {
                                    NativeExpressionCompilerStep::Unknown => {
                                        chunk.unpresented_failure = true;
                                        return None;
                                    }
                                    NativeExpressionCompilerStep::Failure(failure) => {
                                        let mut note = context_note()?;
                                        note.before_context
                                            .push(expression.error_context.note().into());
                                        let Some(message) = failure.message else {
                                            chunk.unpresented_failure = true;
                                            return None;
                                        };
                                        return Some(NativeCompilationError {
                                            message,
                                            error_code: failure.error_code,
                                            command_contexts: vec![note],
                                            body_line: line_of(source, command.start),
                                        });
                                    }
                                    NativeExpressionCompilerStep::Script(span) => {
                                        let script = text
                                            .get(span.as_range())?
                                            .strip_prefix('[')?
                                            .strip_suffix(']')?;
                                        if let Some(mut failure) = self.compile_native_script(
                                            script.as_bytes(),
                                            namespace,
                                            context,
                                            chunk,
                                            depth + 1,
                                        ) {
                                            failure
                                                .command_contexts
                                                .last_mut()?
                                                .after_context
                                                .push(expression.error_context.note().into());
                                            failure.command_contexts.push(context_note()?);
                                            failure.body_line = line_of(source, command.start);
                                            return Some(failure);
                                        }
                                        if chunk.unpresented_failure {
                                            return None;
                                        }
                                    }
                                }
                            }
                            continue;
                        }
                    };
                    let parse::WordBody::Literal(script) =
                        &command.words.get(body.argument + 1)?.body
                    else {
                        continue;
                    };
                    let entered = selected.spec.body_context_for_operand(
                        Some(self.native_invocation_dialect()),
                        context,
                        selected.selection,
                        selected.shapes[body.argument],
                    );
                    let entered = body.entered_context(entered)?;
                    if let Some(mut failure) =
                        self.compile_native_script(script, namespace, entered, chunk, depth + 1)
                    {
                        if body.failure_scope == NativeCompilationFailureScope::FallbackToGeneric {
                            // A declined catch compiles its protected body only
                            // after runtime reaches the wrapper.
                            chunk.scripts.remove(&(script.to_vec(), entered));
                            selected.selection = Selection::Generic;
                            selected.handler = None;
                            selected.body_contexts.clear();
                            continue;
                        }
                        let note = body.error_context.note(Some(failure.body_line))?;
                        failure
                            .command_contexts
                            .last_mut()?
                            .after_context
                            .push(note);
                        failure.command_contexts.push(context_note()?);
                        failure.body_line = line_of(source, command.start);
                        return Some(failure);
                    }
                    if chunk.unpresented_failure {
                        if body.failure_scope == NativeCompilationFailureScope::FallbackToGeneric {
                            chunk.unpresented_failure = false;
                            chunk.scripts.remove(&(script.to_vec(), entered));
                            selected.selection = Selection::Generic;
                            selected.handler = None;
                            selected.body_contexts.clear();
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
        if !has_selected {
            return self.compile_native_substitutions(source, command, entry, chunk, depth);
        }
        None
    }

    fn compile_native_substitutions(
        &self,
        source: &[u8],
        command: &parse::Command<'_>,
        entry: CompilerEntry,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        for word in &command.words {
            if let parse::WordBody::Parts(parts) = &word.body {
                if let Some(mut failure) =
                    self.compile_native_word_parts(parts, entry, chunk, depth)
                {
                    failure
                        .command_contexts
                        .push(NativeCompilationErrorCommand {
                            text: String::from_utf8(source[command.start..command.end].to_vec())
                                .ok()?,
                            line: line_of(source, command.start),
                            before_context: Vec::new(),
                            after_context: Vec::new(),
                        });
                    failure.body_line = line_of(source, command.start);
                    return Some(failure);
                }
                if chunk.unpresented_failure {
                    return None;
                }
            }
        }
        None
    }

    fn compile_native_word_parts(
        &self,
        parts: &[parse::WordPart<'_>],
        entry: CompilerEntry,
        chunk: &mut CompiledChunk,
        depth: u32,
    ) -> Option<NativeCompilationError> {
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return None;
        }
        for part in parts {
            let failure = match part {
                parse::WordPart::Command(script) => self.compile_native_script(
                    script,
                    entry.namespace,
                    entry.context,
                    chunk,
                    depth + 1,
                ),
                parse::WordPart::Variable(variable) => variable.index.as_ref().and_then(|parts| {
                    self.compile_native_word_parts(parts, entry, chunk, depth + 1)
                }),
                _ => None,
            };
            if failure.is_some() || chunk.unpresented_failure {
                return failure;
            }
        }
        None
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

    pub(super) fn enter_native_procedure_frame<O: obj::ObjectPointer>(
        &mut self,
        params: &[Param<O>],
        call_args: &[*mut TclObj],
        usage_called: &[u8],
        context: (NsId, &mut CallMeta<'_>),
        bindings: Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
        compiled_layout: Option<&tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    ) -> Result<(NsId, Box<CmdFrame>), Code> {
        use tcl_syntax::formal_params::FormalByteArgumentBinding as Binding;
        let (ns, meta) = context;
        let caller_level = self.current_level();
        // Recursion bound (catchable, not a stack overflow).
        if self.recursion_depth.get() >= self.recursion_limit.get() {
            return Err(self.error(b"too many nested evaluations (infinite loop?)"));
        }
        self.recursion_depth.set(self.recursion_depth.get() + 1);

        self.enter_namespace_activation(ns);
        if meta.same_level {
            self.frames.borrow_mut().push_same_level(ns);
        } else {
            self.frames.borrow_mut().push(ns);
        }
        if self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Tcl) {
            if let Some(procedure) = meta.c_procedure {
                self.frames
                    .borrow_mut()
                    .retain_c_procedure(Rc::clone(procedure));
            }
            if let Some(argv) = meta.original_argv {
                self.frames
                    .borrow_mut()
                    .install_original_error_stack_argv(argv);
            }
        }
        if self.native_invocation_dialect().native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            let namespace = match meta.jim_namespace {
                Some(original) => Some(Rc::new(obj::Owned::retain(original.as_ptr()))),
                None => self.namespaces.borrow_mut().take_jim_namespace_owner(ns),
            };
            let Some(namespace) = namespace else {
                let popped = self.pop_native_call_frame();
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim procedure frame namespace",
                    )
                    .into(),
                ));
            };
            self.frames.borrow_mut().retain_jim_activation_objects(
                meta.jim_parameters.map(obj::Owned::retain),
                meta.jim_body.map(obj::Owned::retain),
                namespace,
            );
        }
        if let Some(statics) = meta.statics.take() {
            let activation_level = self.current_level();
            self.frames
                .borrow_mut()
                .table_mut(activation_level)
                .expect("new procedure frame")
                .install_statics(statics);
        }
        // Record the invocation words for `info level N`: an OO constructor
        // supplies the `create`/`new` invocation words verbatim; otherwise the
        // invoked name plus the supplied arguments.
        let words = if meta.original_argv.is_some()
            && self
                .native_invocation_dialect()
                .native_string_protocol()
                .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            // Native C borrows objc/objv. No argument string getter is reached
            // merely to install the frame's invocation transport.
            Vec::new()
        } else {
            meta.level_words.take().unwrap_or_else(|| {
                let mut words = Vec::with_capacity(call_args.len() + 1);
                words.push(usage_called.to_vec());
                words.extend(call_args.iter().map(|&arg| obj_bytes(arg)));
                words
            })
        };
        self.frames.borrow_mut().set_words(words);
        let saved_ns = self.current_ns.get();
        self.current_ns.set(ns);

        let indexed = self
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .filter(|protocol| protocol.has_indexed_locals());
        if let Some(protocol) = indexed {
            let names: Vec<_> = params
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect();
            self.frames
                .borrow_mut()
                .install_formal_cells(&names, protocol);
        }
        if let Some(layout) = compiled_layout {
            let installed = self
                .frames
                .borrow_mut()
                .install_native_compiled_local_layout(layout);
            if let Err(error) = installed {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved_ns);
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(crate::builtins::var_error(self, usage_called, error));
            }
        }
        if let Err(error) = self.refresh_native_local_name_table() {
            let popped = self.pop_native_call_frame();
            self.current_ns.set(saved_ns);
            self.leave_namespace_activation(popped);
            self.recursion_depth.set(self.recursion_depth.get() - 1);
            return Err(self.report_cmd_error(error.into()));
        }

        if let Some(resolver) = meta.oo_variable_resolver.take() {
            let installed = self
                .frames
                .borrow_mut()
                .install_tcloo_variable_resolver(resolver);
            if let Err(error) = installed {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved_ns);
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(crate::builtins::var_error(self, usage_called, error));
            }
        }
        if let Some(layout) = compiled_layout {
            let protocol = match self.require_variable_name_protocol() {
                Ok(protocol) => protocol,
                Err(error) => {
                    let popped = self.pop_native_call_frame();
                    self.current_ns.set(saved_ns);
                    self.leave_namespace_activation(popped);
                    self.recursion_depth.set(self.recursion_depth.get() - 1);
                    return Err(crate::builtins::var_error(self, usage_called, error));
                }
            };
            // C compiled-local initialisers exclude arguments and temporaries:
            // TclInitCompiledLocals in C8, InitResolvedLocals in C9. Actual
            // formal slots keep their local cells; retained layout None entries
            // independently exclude unnamed temporaries.
            // naming.procedure.compiled-local-resolver-formal-exclusion
            // docs/design/analysis/name-resolution-proofs/procedure-compiled-local-resolver-formal-exclusion.md
            for (slot, primary) in layout.names.iter().enumerate().skip(params.len()) {
                let Some(primary) = primary else { continue };
                for (local, target) in meta.link_vars {
                    let selected = tcl_syntax::naming::native_oo_variable_resolver_matches(
                        protocol,
                        tcl_syntax::naming::NativeOoVariableResolverPurpose::CompiledPrimary,
                        local,
                        primary.as_bytes(),
                    );
                    match selected {
                        Ok(false) => continue,
                        Ok(true) => {}
                        Err(_) => {
                            let popped = self.pop_native_call_frame();
                            self.current_ns.set(saved_ns);
                            self.leave_namespace_activation(popped);
                            self.recursion_depth.set(self.recursion_depth.get() - 1);
                            return Err(self.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "TclOO compiled variable resolver",
                                )
                                .into(),
                            ));
                        }
                    }
                    let bound = crate::vars::make_tcloo_compiled_variable(
                        &mut self.frames.borrow_mut(),
                        &mut self.namespaces.borrow_mut(),
                        ns,
                        slot,
                        target,
                    );
                    if let Err(error) = bound {
                        let popped = self.pop_native_call_frame();
                        self.current_ns.set(saved_ns);
                        self.leave_namespace_activation(popped);
                        self.recursion_depth.set(self.recursion_depth.get() - 1);
                        return Err(crate::builtins::var_error(self, primary.as_bytes(), error));
                    }
                    break;
                }
            }
        }

        for binding in bindings {
            let stored = match binding {
                Binding::Value {
                    parameter,
                    argument,
                } => (if indexed.is_some() {
                    self.frames
                        .borrow_mut()
                        .store_formal_cell(parameter, call_args[argument])
                } else {
                    self.var_set_named(&params[parameter].name, call_args[argument])
                })
                .map_err(|_| {
                    self.set_error(b"proc parameter binding failed");
                }),
                Binding::Default { parameter } => {
                    let value = params[parameter].default.as_ref().unwrap().as_ptr();
                    if let Err(error) = obj::check_native_liveness(value) {
                        let popped = self.pop_native_call_frame();
                        self.current_ns.set(saved_ns);
                        self.leave_namespace_activation(popped);
                        self.recursion_depth.set(self.recursion_depth.get() - 1);
                        return Err(self.report_cmd_error(error.into()));
                    }
                    let result = if indexed.is_some() {
                        self.frames.borrow_mut().store_formal_cell(parameter, value)
                    } else {
                        self.var_set_named(&params[parameter].name, value)
                    };
                    result.map_err(|_| {
                        self.set_error(b"proc parameter binding failed");
                    })
                }
                Binding::Rest {
                    parameter,
                    name,
                    start,
                    len,
                } => {
                    let value = self.new_list_object(&call_args[start..start + len]);
                    let result = if indexed.is_some() {
                        self.frames.borrow_mut().store_formal_cell(parameter, value)
                    } else {
                        self.var_set(&name, value)
                    };
                    if result.is_err() {
                        drop_fresh(value);
                    }
                    result.map_err(|_| {
                        self.set_error(b"proc parameter binding failed");
                    })
                }
                Binding::CallerLink { name, argument, .. } => {
                    let target = obj_bytes(call_args[argument]);
                    let exists = crate::vars::exists_at(
                        &self.frames.borrow(),
                        &self.namespaces(),
                        &target,
                        caller_level,
                    );
                    if !exists {
                        let mut message = b"can't read \"".to_vec();
                        message.extend_from_slice(&target);
                        message.extend_from_slice(b"\": no such variable");
                        self.set_error(&message);
                        Err(())
                    } else if crate::cmd_var::bind_upvar_at(self, caller_level, &target, &name)
                        == Code::Ok
                    {
                        Ok(())
                    } else {
                        Err(())
                    }
                }
            };
            if stored.is_err() {
                let popped = self.pop_native_call_frame();
                self.current_ns.set(saved_ns);
                self.leave_namespace_activation(popped);
                self.recursion_depth.set(self.recursion_depth.get() - 1);
                return Err(Code::Error);
            }
        }

        // The proc body runs as its own `info frame` level: `type proc` (or
        // `source` if defined in a sourced file), the proc FQN, and the new call
        // level (set after `frames.push`, so `current_level` is the proc's).
        // A TclOO method body carries its method context for `info frame`
        // (`method`/`class`|`object`), which displaces the `proc` key.
        let oo = match &meta.err {
            ProcFrame::Method { kind, owner, what } => {
                let method = match what {
                    MethodFrameWhat::Named(n) => n.to_vec(),
                    MethodFrameWhat::Constructor | MethodFrameWhat::Destructor => Vec::new(),
                };
                Some((method, kind.to_vec(), owner.to_vec()))
            }
            _ => None,
        };
        // An `apply` lambda reports `lambda <expr>` (not `proc`) in `info frame`.
        let lambda = match &meta.err {
            ProcFrame::Lambda(expr) => Some(expr.to_vec()),
            _ => None,
        };
        let (proc_lvl, proc_idx) = {
            let f = self.frames.borrow();
            (f.current_level(), f.current_frame_index())
        };
        let proc_frame = CmdFrame {
            kind: if meta.source.is_some() {
                FrameKind::Source
            } else {
                FrameKind::Proc
            },
            file: meta.source.take(),
            proc: meta.fqn.map(<[u8]>::to_vec),
            level: proc_lvl,
            omit_level: false,
            frame_index: proc_idx,
            line_base: meta.body_line_base,
            proc_line_base: meta.body_line_base,
            cmd: Vec::new(),
            original_command: None,
            line: 1,
            oo,
            lambda,
        };
        Ok((saved_ns, Box::new(proc_frame)))
    }

    pub(super) fn prepare_native_procedure(
        &self,
        source: &[u8],
        namespace: NsId,
    ) -> Result<Box<NativeProcedureAdmission>, NativeCompilerAdmissionFailure> {
        let context =
            if self.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Tcl) {
                Context {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                }
            } else {
                direct_activation().context
            };
        let mut chunk = CompiledChunk::default();
        if self.native_invocation_dialect().tcl_version == Some(tcl_dialect::TclVersion::V8_4) {
            if let Some(failure) =
                self.compile_native_script(source, namespace, context, &mut chunk, 0)
            {
                return Err(NativeCompilerAdmissionFailure::Presented(Box::new(failure)));
            }
        }
        if chunk.unpresented_failure {
            return Err(NativeCompilerAdmissionFailure::Unpresented);
        }
        Ok(Box::new(NativeProcedureAdmission {
            interpreter: self.clone(),
            activation: Some(ScriptActivation {
                context,
                chunk: Rc::new(chunk),
            }),
            active: false,
        }))
    }

    pub(super) fn release_native_procedure(&self) {
        self.native_compilation.borrow_mut().pending.pop();
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
        if is_root {
            self.clear_ensemble_rewrite();
        }
        super::release_all(&[head]);
        code
    }

    pub(super) fn admit_native_compilation_error(
        &mut self,
        failure: &NativeCompilerAdmissionFailure,
        procedure: Option<&[u8]>,
    ) -> Code {
        let NativeCompilerAdmissionFailure::Presented(failure) = failure else {
            self.native_compilation.borrow_mut().admission_error =
                Some(NativeCompilationAdmissionError::NativePreflightRequired);
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

pub(super) fn formal_parameters<O: obj::ObjectPointer>(
    parameters: &[Param<O>],
    grammar: tcl_dialect::ParameterGrammar,
) -> Vec<tcl_syntax::formal_params::ByteFormalParameter> {
    parameters
        .iter()
        .map(|parameter| tcl_syntax::formal_params::ByteFormalParameter {
            name: parameter.name.clone(),
            default: parameter.default.as_ref().map(|value| {
                if grammar == tcl_dialect::ParameterGrammar::Jim && parameter.name == b"args" {
                    obj_bytes(value.as_ptr())
                } else {
                    Vec::new()
                }
            }),
        })
        .collect()
}

pub(super) fn native_parameter_plan<O: obj::ObjectPointer>(
    parameters: &[Param<O>],
    argument_count: usize,
    grammar: tcl_dialect::ParameterGrammar,
) -> Result<
    Vec<tcl_syntax::formal_params::FormalByteArgumentBinding>,
    tcl_syntax::formal_params::FormalArityError,
> {
    tcl_syntax::formal_params::bind_formal_argument_bytes(
        &formal_parameters(parameters, grammar),
        argument_count,
        grammar,
    )
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

pub(super) struct NativeProcedureAdmission {
    interpreter: Interp,
    activation: Option<ScriptActivation>,
    active: bool,
}

impl NativeProcedureAdmission {
    pub(super) fn activate(&mut self) {
        self.interpreter
            .native_compilation
            .borrow_mut()
            .pending
            .push(self.activation.take());
        self.active = true;
    }
}

impl Drop for NativeProcedureAdmission {
    fn drop(&mut self) {
        if self.active {
            self.interpreter.release_native_procedure();
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
mod tests {
    use super::*;

    #[cfg(have_tommath)]
    #[test]
    fn safe_index_missing_fixed_table_stays_an_outer_refusal() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert!(interp.native_math_function_table().is_some());
        interp.native_compilation.borrow_mut().fixed_math = None;
        let error =
            tcl_cmd_core::index::resolve_for_ops(&mut interp, "0 && sqrt(1)", 2).unwrap_err();
        assert!(error.native_access_refusal().is_some());
        assert_eq!(interp.report_cmd_error(error), Code::Error);
        assert!(interp.native_access_refusal().is_some());

        let mut caught = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        caught.native_compilation.borrow_mut().fixed_math = None;
        assert_eq!(
            caught.eval_str(b"catch {lindex {A B} {0 && sqrt(1)}} captured"),
            Code::Error,
        );
        assert!(caught.native_access_refusal().is_some());
    }

    fn interpreter(dialect: &str) -> Interp {
        let profile = tcl_registry::model::resolve_environment(dialect).unit_profile();
        let mut interpreter = Interp::new();
        interpreter.set_dialect_profile(profile);
        interpreter
    }

    #[test]
    fn variable_output_addresses_follow_the_selected_native_write_order() {
        for reference in tcl_test_support::available_tclshs() {
            for source in tcl_test_support::variable_outputs::variable_output_lookup_scripts(
                reference.version,
            ) {
                let oracle = tcl_test_support::run_script(
                    &reference.path,
                    format!("puts [{source}]\n").as_bytes(),
                )
                .unwrap();
                assert!(oracle.success(), "{source}");
                assert!(oracle.stderr.is_empty(), "{source}: {:?}", oracle.stderr);
                let mut interpreter =
                    interpreter(&format!("tcl{}", reference.version.version_string()));
                assert_eq!(
                    interpreter.eval_str(source.as_bytes()),
                    Code::Ok,
                    "{source}"
                );
                let mut actual = interpreter.result_bytes();
                actual.push(b'\n');
                assert_eq!(actual, oracle.stdout, "{}: {source}", reference.patchlevel);
            }
        }
    }

    #[test]
    fn compiled_native_selection_precedes_argv_in_procedures_but_direct_eval_remains_late() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interpreter = interpreter(dialect);
            let script = b"proc p {} {set x [proc set args {return CUSTOM}]; info exists x}; p";
            assert_eq!(interpreter.eval_str(script), Code::Ok, "{dialect}");
            assert_eq!(
                interpreter.result_bytes(),
                if dialect == "jim" { b"0" } else { b"1" },
                "{dialect}"
            );
            let mut interpreter = self::interpreter(dialect);
            assert_eq!(
                interpreter
                    .eval_str(b"eval {set x [proc set args {return CUSTOM}]}; info exists x"),
                Code::Ok
            );
            assert_eq!(interpreter.result_bytes(), b"0", "{dialect}");
        }
    }

    #[test]
    fn legacy_compilation_rejects_the_entire_chunk_before_arity_and_prunes_only_literal_booleans() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for (body, legacy_rejects) in [
                ("set ::before 1; set x extra bad", true),
                ("set ::before 1; if {0} {set x extra bad}", false),
                ("set ::before 1; if {0+0} {set x extra bad}", true),
                ("set ::before 1; while {0} {set x extra bad}", false),
                ("set ::before 1; for {} {0} {} {set x extra bad}", true),
                ("set ::before 1; foreach x {} {set x extra bad}", true),
                ("set ::before 1; catch {set x extra bad}", false),
            ] {
                let mut interpreter = interpreter(dialect);
                let source = format!(
                    "set before 0; proc p {{required}} {{{body}}}; catch {{p}} message; list $before $message"
                );
                assert_eq!(
                    interpreter.eval_str(source.as_bytes()),
                    Code::Ok,
                    "{dialect}: {body}"
                );
                let result = interpreter.result_bytes();
                let result = String::from_utf8(result).unwrap();
                assert!(result.starts_with("0 "), "{dialect}: {body}: {result}");
                let usage = if dialect == "tcl8.4" && legacy_rejects {
                    "set varName"
                } else {
                    "p required"
                };
                assert!(result.contains(usage), "{dialect}: {body}: {result}");
            }
        }
    }

    #[test]
    fn native_runtime_matches_all_six_compiler_traversal_oracles() {
        fn capture(interpreter: &mut Interp, arguments: &[*mut TclObj]) -> Code {
            let mut output = interpreter
                .var_get(b"::observations")
                .map_or_else(Vec::new, obj_bytes);
            output.extend_from_slice(&obj_bytes(arguments[1]));
            output.push(b'\n');
            let value = new_string(&output);
            if interpreter.var_set(b"::observations", value).is_err() {
                drop_fresh(value);
                return Code::Error;
            }
            interpreter.set_result_bytes(b"");
            Code::Ok
        }
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal.tcl"
        ));
        for (dialect, expected) in [
            (
                "tcl8.4",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-8.4.txt"
                )),
            ),
            (
                "tcl8.5",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-8.5.txt"
                )),
            ),
            (
                "tcl8.6",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-8.6.txt"
                )),
            ),
            (
                "tcl9.0",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-9.0.txt"
                )),
            ),
            (
                "tcl9.1",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-9.1.txt"
                )),
            ),
            (
                "jim",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../rust/tcl-syntax/tests/data/body_execution/native-compiler-traversal-jim.txt"
                )),
            ),
        ] {
            let mut interpreter = interpreter(dialect);
            interpreter.register_builtin(b"puts", capture);
            assert_eq!(
                interpreter.eval_str(fixture.as_bytes()),
                Code::Ok,
                "{dialect}: {:?}",
                interpreter.result_bytes()
            );
            let observed = interpreter
                .var_get(b"::observations")
                .map(obj_bytes)
                .unwrap();
            assert_eq!(String::from_utf8(observed).unwrap(), expected, "{dialect}");
        }
    }

    #[test]
    fn unpresented_native_rejection_is_a_host_obligation_before_any_effects() {
        let mut interpreter = interpreter("tcl8.4");
        assert_eq!(
            interpreter.eval_str(
                b"set before 0; proc p {} {set ::before 1; expr {1 ** 2}}; catch p; set after 1"
            ),
            Code::Error,
        );
        assert_eq!(
            interpreter.native_compilation_admission_error(),
            Some(NativeCompilationAdmissionError::NativePreflightRequired,)
        );
        assert_eq!(
            interpreter.var_get(b"before").map(obj_bytes),
            Some(b"0".to_vec())
        );
        assert!(interpreter.var_get(b"after").is_none());
    }

    #[test]
    fn legacy_error_contexts_preserve_inline_annotations_and_source_word_delimiters() {
        let mut interpreter = interpreter("tcl8.4");
        assert_eq!(interpreter.eval_str(b"proc p {} {\nset before 1\nif 1 {\nset x extra bad\n}\n}; catch p; set ::errorInfo"), Code::Ok);
        assert_eq!(interpreter.result_bytes(), b"wrong # args: should be \"set varName ?newValue?\"\n    while compiling\n\"set x extra bad\"\n    (\"if\" then script line 2)\n    while compiling\n\"if 1 {\nset x extra bad\n}\"\n    (compiling body of proc \"p\", line 3)\n    invoked from within\n\"p\"");
        for (body, expected) in [
            (
                "if {1}",
                "wrong # args: no script following \"{1}\" argument",
            ),
            (
                "if \"1\"",
                "wrong # args: no script following \"\"1\"\" argument",
            ),
            ("{if}", "wrong # args: no expression after \"if\" argument"),
        ] {
            let source = format!("proc p {{}} {{{body}}}; catch p message; set message");
            assert_eq!(interpreter.eval_str(source.as_bytes()), Code::Ok);
            assert_eq!(interpreter.result_bytes(), expected.as_bytes(), "{body}");
        }
    }

    #[test]
    fn selected_error_preserves_native_handler_across_argument_mutation() {
        for (dialect, expected) in [
            ("tcl8.4", b"0 CUSTOM".as_slice()),
            ("tcl8.5", b"0 CUSTOM".as_slice()),
            ("tcl8.6", b"1 MESSAGE".as_slice()),
            ("tcl9.0", b"1 MESSAGE".as_slice()),
            ("tcl9.1", b"1 MESSAGE".as_slice()),
            ("jim", b"0 CUSTOM".as_slice()),
        ] {
            for body in [
                "error [rename error native_error;proc error args {return CUSTOM};list MESSAGE]",
                "error MESSAGE [rename error native_error;proc error args {return CUSTOM};list INFO]",
            ] {
                let mut interpreter = interpreter(dialect);
                let source =
                    format!("proc p {{}} {{{body}}};set code [catch p result];list $code $result");
                assert_eq!(
                    interpreter.eval_str(source.as_bytes()),
                    Code::Ok,
                    "{dialect}: {body}"
                );
                assert_eq!(interpreter.result_bytes(), expected, "{dialect}: {body}");
            }
        }
    }

    #[test]
    fn execution_traces_decline_native_selection_and_custom_host_handlers_never_gain_stock_proof() {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.1"] {
            let mut interpreter = interpreter(dialect);
            assert_eq!(interpreter.eval_str(b"proc observe args {}; trace add execution set enter observe; proc p {} {set x [proc set args {return CUSTOM}]; info exists x}; p"), Code::Ok);
            assert_eq!(interpreter.result_bytes(), b"0", "{dialect}");
        }
        fn custom(interpreter: &mut Interp, _: &[*mut TclObj]) -> Code {
            interpreter.set_result_bytes(b"HOST");
            Code::Ok
        }
        let mut interpreter = interpreter("tcl8.4");
        interpreter.register_builtin(b"set", custom);
        assert_eq!(
            interpreter.eval_str(b"proc p {} {set x extra bad}; p"),
            Code::Ok
        );
        assert_eq!(interpreter.result_bytes(), b"HOST");
    }
}
