// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executable C original-object bodies, with their actual local/literal owners.

use super::native_literal_pool::{
    NativeRuntimeLiteral, NativeRuntimeLiteralAction, NativeRuntimeLiteralArray,
};
use super::*;
#[path = "native_body_artifact/native_dictionary.rs"]
mod native_dictionary;
#[path = "native_body_artifact/native_error.rs"]
mod native_error;
use native_error::ErrorOperation;
#[path = "native_body_artifact/native_named.rs"]
mod native_named;
use native_dictionary::DictionaryLookupOperation;
#[path = "native_body_artifact/native_switch.rs"]
mod native_switch;
#[path = "native_body_artifact/native_try.rs"]
mod native_try;
use native_named::NamedOperation;
use native_try::TryOperation;
mod native_control;
mod native_string;
use native_string::StringMatchOperation;
mod native_each;
mod native_unset;
use native_each::EachOperation;
use native_unset::UnsetOperation;
#[path = "native_body_artifact/native_control_preparation.rs"]
mod native_control_preparation;
use native_control_preparation::PreparedControlOperands;
use std::collections::HashMap;
use std::rc::Weak;
use tcl_bytecode::{LiteralTable, LocalVarTable, NativeLiteralAction, NativeLiteralAllocation};
use tcl_lexer::{ExecutablePart, ExecutablePartArena, NativeWord, PartListId, SourceImage, Span};
use tcl_registry::native_compilation::{
    NativeCompilationContext as Context, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationSelection,
};
use tcl_registry::native_compiler_words::{NativeCompiledListRecipe, NativeCompilerWords};
use tcl_registry::native_instruction_plan::{
    native_instruction_plan, NativeAppendInstruction, NativeAppendOperands, NativeArgumentListStep,
    NativeInstructionPlan,
};
use tcl_runtime_api::native_compilation::{
    NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind,
};
use tcl_syntax::naming::{
    NativeCompiledVariableEnvironment as Environment, NativeCompiledVariableLookup as Lookup,
};
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::native_variable_words::NativeVariableWordOperand;
use tcl_syntax::value::{ValueError, ValueOps};

/// The owning declaration is non-owning in the object's native Bytecode primary.
#[derive(Clone)]
enum BodyContext {
    Script,
    Procedure(Weak<ProcDef>),
}

#[derive(Clone, PartialEq, Eq)]
struct CacheStamp {
    interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    namespace: NsId,
    epochs: (u64, u64),
    physical: tcl_dialect::TclVersion,
    grammar: tcl_lexer::LexerConfig,
    source_protocol: NativeStringProtocol,
    observers: Vec<(u64, Option<u64>, u8)>,
    steps: Vec<(Option<u64>, u8, Vec<u8>)>,
    borrowed_table: Option<usize>,
    fixed_math: Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable>,
}

pub(super) struct NativeBodyArtifact {
    stamp: CacheStamp,
    owner: BodyContext,
    image: SourceImage,
    region: Span,
    scripts: HashMap<Span, Script>,
    literals: Rc<NativeRuntimeLiteralArray>,
    locals: Option<NativeCompiledLocalLayout>,
    // Nonowning borrowed layout: a cached script must not extend procedure
    // resources merely by remembering their original table identity.
    _borrowed_table:
        Option<Weak<tcl_runtime_api::native_literal::NativeLocalNameTable<obj::Owned>>>,
}

struct Script {
    commands: Vec<CommandInstruction>,
    fatal: Option<SyntaxInstruction>,
    empty: Option<usize>,
}
struct SyntaxInstruction {
    cut: tcl_lexer::NativeScriptWordCut,
    message: usize,
    options: usize,
}
struct CommandInstruction {
    span: Span,
    words: Vec<WordInstruction>,
    operation: Operation,
}
struct WordInstruction {
    original: NativeWord,
    literal: Option<usize>,
    arena: ArenaInstruction,
}
struct ArenaInstruction {
    original: ExecutablePartArena,
    texts: HashMap<Span, usize>,
    locals: HashMap<Span, usize>,
    roots: HashMap<Span, usize>,
}
type NativeArenaFrame = (
    PartListId,
    usize,
    Vec<obj::Owned>,
    Option<(Span, Option<usize>)>,
);

enum Operation {
    StringMatch(StringMatchOperation),
    Error(ErrorOperation),
    DictionaryLookup(DictionaryLookupOperation),
    NamedInvocation(NamedOperation),
    Expression(Box<native_control::ExpressionOperation>),
    Try(TryOperation),
    Break,
    Continue,
    Control(Box<native_control::ControlOperation>),
    Each(EachOperation),
    NamespaceBindings(NamespaceOperation),
    Switch(SwitchOperation),
    TclOoHelper(
        tcl_registry::native_tcloo_compilation::NativeTclOoInstruction,
        Vec<Option<usize>>,
    ),
    Invoke,
    List(NativeCompiledListRecipe, Option<usize>),
    Concat {
        literal: Option<usize>,
        operands: Vec<NamespaceOperand>,
        prepared_words: HashMap<usize, WordInstruction>,
    },
    Unset(UnsetOperation),
    Load(Target),
    Store(Target, usize),
    Increment(Target, Option<usize>, Option<i32>),
    Append(Target, NativeAppendInstruction),
    SelectedReturn(
        tcl_registry::native_return_compilation::NativeReturnInstruction,
        Option<usize>,
        Option<usize>,
    ),
    Uplevel(
        tcl_registry::native_instruction_plan::NativeUplevelInstruction,
        Option<usize>,
    ),
}

enum NamespaceOperand {
    Original(usize),
    Literal(usize),
}
struct NamespaceBindingInstruction {
    name: NamespaceOperand,
    slot: usize,
    local: Vec<u8>,
    value: Option<NamespaceOperand>,
}
struct NamespaceOperation {
    kind: tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind,
    bindings: Vec<NamespaceBindingInstruction>,
    generic: Option<Vec<NamespaceOperand>>,
    prepared_words: HashMap<usize, WordInstruction>,
    prefix: Option<usize>,
    namespace: Option<NamespaceOperand>,
    empty: Option<usize>,
}

struct SwitchOperation {
    recipe: tcl_registry::native_switch_compilation::NativeSwitchInstruction,
    patterns: Vec<Option<usize>>,
    subject_literal: Option<usize>,
    empty: Option<usize>,
}

/// `DONE` exits this complete compiled procedure, including bracket programs,
/// without creating Tcl return-option state.
#[derive(Default)]
struct BodyExecution {
    done: bool,
}
struct Target {
    original: NativeVariableWordOperand,
    slot: Option<usize>,
    index: Option<ArenaInstruction>,
    root_literal: Option<usize>,
    index_literal: Option<usize>,
}

struct EvaluatedTarget {
    root: Vec<u8>,
    element: Option<Vec<u8>>,
    original_name: Option<obj::Owned>,
    original_index: Option<obj::Owned>,
    combined: bool,
}

extern "C" fn free_body(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns one boxed Rc, not its source object.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut Rc<NativeBodyArtifact>
        ));
    }
}
extern "C" fn duplicate_body(_source: *mut TclObj, _target: *mut TclObj) {
    // Native Bytecode duplication leaves the fresh duplicate string-only.
}
static BYTECODE_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"bytecode".as_ptr(),
    free_int_rep_proc: Some(free_body),
    dup_int_rep_proc: Some(duplicate_body),
    update_string_proc: None,
    set_from_any_proc: None,
};

fn cache(value: *mut TclObj) -> Option<Rc<NativeBodyArtifact>> {
    if !core::ptr::eq(obj::obj_type_ptr(value), &BYTECODE_TYPE) {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates this boxed owner.
    Some(unsafe { &*(obj::internal_rep(value) as usize as *const Rc<NativeBodyArtifact>) }.clone())
}
fn install(value: *mut TclObj, artifact: &Rc<NativeBodyArtifact>) {
    obj::change_type(
        value,
        &BYTECODE_TYPE,
        Box::into_raw(Box::new(Rc::clone(artifact))) as usize as u64,
    );
}

struct Builder<'a> {
    interp: &'a mut Interp,
    image: SourceImage,
    context: Context,
    stamp: CacheStamp,
    lvt: LocalVarTable,
    literals: LiteralTable,
    scripts: HashMap<Span, Script>,
    parse_failure: Option<tcl_lexer::NativeScriptWordCut>,
    compilation_failure: Option<tcl_registry::native_compilation::NativeCompilationFailure>,
    private_objects: HashMap<usize, obj::Owned>,
}

impl Builder<'_> {
    fn reject_native_compilation(
        &mut self,
        failure: tcl_registry::native_compilation::NativeCompilationFailure,
    ) -> ValueError {
        self.compilation_failure = Some(failure);
        unavailable("native registered compiler rejected original source")
    }
    fn original_literal(&mut self, original: obj::Owned) -> usize {
        let index = self.literals.register_private_original();
        self.private_objects.insert(index, original);
        index
    }
    fn syntax_instruction(
        &mut self,
        cut: tcl_lexer::NativeScriptWordCut,
    ) -> Result<SyntaxInstruction, ValueError> {
        use tcl_registry::native_return_options::{
            NativeReturnOptionsApplication::Syntax, NativeSyntaxMessageAllocation,
        };
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_return_options_application(Syntax)
            .ok_or_else(|| unavailable("native syntax instruction producer"))?;
        self.interp.error(cut.cut.message.as_bytes());
        let message = match protocol
            .syntax_message_allocation()
            .ok_or_else(|| unavailable("native syntax message allocation"))?
        {
            NativeSyntaxMessageAllocation::UnsharedString => {
                self.literals.register_unshared(cut.cut.message.as_bytes())
            }
            NativeSyntaxMessageAllocation::RegisteredString => {
                self.literals.intern_bytes(cut.cut.message.as_bytes())
            }
            NativeSyntaxMessageAllocation::OriginalObject => {
                self.original_literal(obj::Owned::retain(self.interp.result_obj()))
            }
        };
        let options = self.interp.capture_original_c_syntax_options()?;
        let options = self.original_literal(options);
        if !protocol.syntax_retains_error_stack() {
            self.interp.reset_original_c_compiler_result()?;
        }
        Ok(SyntaxInstruction {
            cut,
            message,
            options,
        })
    }
    fn command_literal(
        &mut self,
        words: &NativeCompilerWords<'_>,
    ) -> Result<Option<usize>, ValueError> {
        let Some(head) = words.literal(0) else {
            return Ok(None);
        };
        let (point, names, context, binding) = self.command_literal_lookup(head)?;
        let selected =
            tcl_registry::native_command_literal::native_compiled_command_literal_from_lookup(
                point, names, context, words, binding,
            )
            .map_err(|_| unavailable("native command literal original lookup"))?;
        let Some(selected) = selected else {
            return Ok(None);
        };
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming {
            if !self.literals.prime_native_command_name(index, priming) {
                return Err(unavailable("native command literal priming geometry"));
            }
        }
        if selected.hide {
            self.literals.hide_native_literal(index);
        }
        Ok(Some(index))
    }
    fn command_literal_lookup(
        &self,
        bytes: &[u8],
    ) -> Result<
        (
            tcl_dialect::model::DialectPoint,
            tcl_syntax::naming::NamePolicyProtocol,
            tcl_runtime_api::native_command_name::NativeLiteralContext,
            Option<tcl_runtime_api::native_compilation::NativeCompilationBinding>,
        ),
        ValueError,
    > {
        use tcl_runtime_api::native_compilation::{
            NativeCommandImplementation, NativeCompilationBinding,
        };
        let names = self
            .interp
            .name_policy_protocol()
            .ok_or_else(|| unavailable("native command literal name issuer"))?;
        let point = self
            .interp
            .native_invocation_dialect()
            .execution_point()
            .ok_or_else(|| unavailable("native command literal engine"))?;
        let context = tcl_runtime_api::native_command_name::NativeLiteralContext {
            interpreter: self.stamp.interpreter,
            namespace_token: self.stamp.namespace as u64,
            entry_epoch: self.stamp.epochs.0,
            namespace_path: self
                .interp
                .namespaces
                .borrow()
                .native_context_path(self.stamp.namespace)
                .ok_or_else(|| unavailable("native command literal namespace"))?,
        };
        let binding = self
            .interp
            .native_command_name_from_binding(self.stamp.namespace, bytes)?
            .map(|cache| {
                let compiler_hook = self
                    .interp
                    .namespaces
                    .borrow()
                    .native_compiler_hook(cache.token)
                    .unwrap_or(
                        tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown,
                    );
                NativeCompilationBinding {
                    slot: cache.slot,
                    namespace_token: cache.namespace_token,
                    token: cache.token,
                    implementation_generation: cache.implementation_generation,
                    implementation: NativeCommandImplementation::Opaque,
                    compiler_hook,
                    compiler: None,
                    procedure_header: None,
                    has_execution_trace: self
                        .stamp
                        .observers
                        .iter()
                        .any(|(_, token, _)| *token == Some(cache.token)),
                }
            });
        Ok((point, names, context, binding))
    }

    fn selected_command_literal(&mut self, bytes: &[u8]) -> Result<usize, ValueError> {
        let (point, names, context, binding) = self.command_literal_lookup(bytes)?;
        let selected =
            tcl_registry::native_command_literal::native_compiled_command_name_literal_from_lookup(
                point, names, context, bytes, binding,
            )
            .map_err(|_| unavailable("native selected command literal original lookup"))?;
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming {
            if !self.literals.prime_native_command_name(index, priming) {
                return Err(unavailable(
                    "native selected command literal priming geometry",
                ));
            }
        }
        Ok(index)
    }

    fn retained_selected_command_literal(
        &mut self,
        bytes: &[u8],
        prerequisite: &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    ) -> Result<usize, ValueError> {
        let point = self
            .interp
            .native_invocation_dialect()
            .execution_point()
            .ok_or_else(|| unavailable("native selected command literal engine"))?;
        let names = self
            .interp
            .name_policy_protocol()
            .ok_or_else(|| unavailable("native selected command literal name issuer"))?;
        let context = tcl_runtime_api::native_command_name::NativeLiteralContext {
            interpreter: self.stamp.interpreter,
            namespace_token: self.stamp.namespace as u64,
            entry_epoch: self.stamp.epochs.0,
            namespace_path: self
                .interp
                .namespaces
                .borrow()
                .native_context_path(self.stamp.namespace)
                .ok_or_else(|| unavailable("native selected command literal namespace"))?,
        };
        let selected = tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
            point, names, context, bytes, prerequisite.clone(),
        ).map_err(|_| unavailable("native retained selected worker literal"))?;
        let index = self.literals.intern_native_command_bytes(
            &selected.bytes,
            &selected.context,
            selected.fully_qualified,
        );
        if let Some(priming) = selected.priming {
            if !self.literals.prime_native_command_name(index, priming) {
                return Err(unavailable(
                    "native retained selected worker priming geometry",
                ));
            }
        }
        Ok(index)
    }

    fn local(&mut self, name: &[u8], substitution: Option<bool>) -> Option<usize> {
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_compiled_variable_protocol()?;
        let environment = if self.context.frame == NativeCompilationFrame::ProcedureCode {
            Environment::DeclareProcedure
        } else if self.stamp.borrowed_table.is_some() {
            Environment::BorrowFrameSlots
        } else {
            Environment::None
        };
        let lookup = substitution.map_or_else(
            || protocol.command_lookup(name, environment),
            |single| protocol.substitution_lookup(name, single, environment),
        );
        match lookup {
            Lookup::CreateLocal => Some(self.lvt.intern_native(protocol, name)),
            Lookup::ExistingLocalOnly => self.lvt.find_native(protocol, name),
            Lookup::DynamicName => None,
        }
    }

    fn arena(
        &mut self,
        original: &ExecutablePartArena,
        depth: u32,
    ) -> Result<ArenaInstruction, ValueError> {
        let mut texts = HashMap::new();
        let mut locals = HashMap::new();
        let mut roots = HashMap::new();
        // List IDs preserve original index ownership without recursive trees.
        let mut pending = vec![(original.root(), 0usize)];
        while let Some((list, offset)) = pending.last_mut() {
            let Some(component) = original.list(*list).get(*offset) else {
                pending.pop();
                continue;
            };
            *offset += 1;
            match &component.part {
                ExecutablePart::Text(_) => {
                    let bytes = tcl_syntax::backslash::native_arena_text(
                        original,
                        component,
                        self.stamp.grammar.escapes,
                        self.stamp.source_protocol,
                    )
                    .map_err(|_| unavailable("native body original text geometry"))?;
                    texts.insert(component.span, self.literals.intern_bytes(&bytes));
                }
                ExecutablePart::Variable { name, index } => {
                    let bytes = original
                        .bytes(*name)
                        .ok_or_else(|| unavailable("native body variable geometry"))?;
                    if let Some(slot) = self.local(bytes, Some(index.is_none())) {
                        locals.insert(*name, slot);
                    } else {
                        roots.insert(*name, self.literals.intern_bytes(bytes));
                    }
                    if let Some(index) = index {
                        pending.push((*index, 0));
                    }
                }
                ExecutablePart::Command { body } => self.script(*body, depth + 1)?,
                ExecutablePart::Expression { .. } => {
                    return Err(unavailable(
                        "native body expression-sugar compiler instruction",
                    ));
                }
                ExecutablePart::ParseError(_) => {
                    return Err(unavailable("native body complete word parse ownership"));
                }
            }
        }
        Ok(ArenaInstruction {
            original: original.clone(),
            texts,
            locals,
            roots,
        })
    }

    fn target(
        &mut self,
        original: NativeVariableWordOperand,
        depth: u32,
    ) -> Result<Target, ValueError> {
        let slot = match &original {
            NativeVariableWordOperand::Literal { name, .. }
            | NativeVariableWordOperand::CompoundArray { name, .. } => self.local(name, None),
            NativeVariableWordOperand::DynamicWord => None,
        };
        let index = match &original {
            NativeVariableWordOperand::CompoundArray { index, .. } => {
                Some(self.arena(index, depth)?)
            }
            _ => None,
        };
        let root_literal = if slot.is_none() {
            match &original {
                NativeVariableWordOperand::Literal { name, .. }
                | NativeVariableWordOperand::CompoundArray { name, .. } => {
                    Some(self.literals.intern_bytes(name))
                }
                NativeVariableWordOperand::DynamicWord => None,
            }
        } else {
            None
        };
        let index_literal = match &original {
            NativeVariableWordOperand::Literal {
                index: Some(index), ..
            } => Some(self.literals.intern_bytes(index)),
            _ => None,
        };
        Ok(Target {
            original,
            slot,
            index,
            root_literal,
            index_literal,
        })
    }

    fn operation(
        &mut self,
        words: &[NativeWord],
        depth: u32,
    ) -> Result<Option<Operation>, ValueError> {
        let captured = NativeCompilerWords::capture(words, self.stamp.source_protocol)
            .map_err(|_| unavailable("native body original compiler words"))?;
        if captured
            .shapes()
            .first()
            .and_then(|shape| shape.compiler_head(Some(self.interp.native_invocation_dialect())))
            != Some(true)
        {
            return Ok(Some(Operation::Invoke));
        }
        let Some(head) = captured.literal(0) else {
            return Ok(Some(Operation::Invoke));
        };
        let generation = self
            .interp
            .namespaces
            .borrow()
            .resolve_generation(self.stamp.namespace, head);
        let Some(generation) = generation else {
            return Ok(Some(Operation::Invoke));
        };
        let dialect = self.interp.native_invocation_dialect();
        let mut arguments_from = 1;
        let mut selected_prerequisite = None;
        let mut selected_plan = None;
        let (identity, spec) = if let Some(compiler) = self
            .interp
            .native_ensemble_compiler_configuration(generation)
        {
            use tcl_registry::native_ensemble::NativeEnsembleWorkerSelection;
            use tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite;
            let Some(public) = self
                .interp
                .native_compilation_binding_at(self.stamp.namespace, head)?
            else {
                return Ok(None);
            };
            if public.has_execution_trace {
                return Ok(Some(Operation::Invoke));
            }
            let configuration = compiler
                .ensemble
                .as_ref()
                .expect("actual ensemble configuration");
            let selector =
                tcl_registry::native_ensemble::original_ensemble_selector(&captured, dialect)
                    .map_err(|_| unavailable("native original ensemble selector geometry"))?;
            let selection = tcl_registry::native_ensemble::select_worker_from_original_target(
                configuration,
                selector.as_ref().and_then(|word| word.literal.as_deref()),
                selector.as_ref().map(|word| word.shape),
                Some(dialect),
                |namespace, member, position| {
                    let original = self.interp.native_ensemble_original_target(
                        generation,
                        member.as_bytes(),
                        position,
                    )?;
                    self.interp
                        .native_compilation_binding_from_original(namespace as usize, original)
                        .ok()
                        .flatten()
                },
            );
            let (member, worker) = match selection {
                NativeEnsembleWorkerSelection::Generic => return Ok(Some(Operation::Invoke)),
                NativeEnsembleWorkerSelection::Unknown => return Ok(None),
                NativeEnsembleWorkerSelection::Worker { member, binding } => (member, binding),
            };
            let prerequisite = NativeCommandCompilerPrerequisite {
                interpreter: self.stamp.interpreter,
                lookup_namespace_token: self.stamp.namespace as u64,
                invocation_word: head.into(),
                slot: public.slot,
                namespace_token: public.namespace_token,
                token: public.token,
                implementation_generation: public.implementation_generation,
                compiler,
                selected_worker: Some((*worker).clone()),
                nested_compilers: Vec::new(),
                guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
            };
            let registry = crate::environment::store_for_profile(self.interp.dialect_profile());
            let replacements = [member.as_bytes().to_vec()];
            use tcl_registry::native_selected_worker::{
                OriginalSelectedWorkerCompilation, OriginalSelectedWorkerInvocation,
            };
            let selected =
                tcl_registry::native_selected_worker::compile_original_selected_worker_path(
                    registry,
                    &worker,
                    OriginalSelectedWorkerInvocation {
                        words: &captured,
                        operand_from: 2,
                        replacements: &replacements,
                        dialect,
                        context: self.context,
                    },
                    prerequisite,
                    |ensemble, configuration, member, shape| {
                        tcl_registry::native_ensemble::select_worker_from_original_target(
                            configuration,
                            member,
                            shape,
                            Some(dialect),
                            |namespace, member, position| {
                                let original = self.interp.native_ensemble_original_target(
                                    ensemble.token,
                                    member.as_bytes(),
                                    position,
                                )?;
                                self.interp
                                    .native_compilation_binding_from_original(
                                        namespace as usize,
                                        original,
                                    )
                                    .ok()
                                    .flatten()
                            },
                        )
                    },
                );
            let prerequisite = selected.prerequisite;
            let worker = selected.worker;
            match selected.compilation {
                OriginalSelectedWorkerCompilation::PublicGeneric => {
                    return Ok(Some(Operation::Invoke));
                }
                OriginalSelectedWorkerCompilation::Unavailable => return Ok(None),
                OriginalSelectedWorkerCompilation::Named(recipe) => {
                    return self
                        .named_operation(&captured, recipe, Some(&prerequisite), depth)
                        .map(|operation| Some(Operation::NamedInvocation(operation)));
                }
                OriginalSelectedWorkerCompilation::Operation {
                    spec,
                    selection,
                    plan,
                } => {
                    arguments_from = selected.operand_from;
                    selected_prerequisite = Some(prerequisite);
                    selected_plan = Some((selection, *plan));
                    let identity = worker
                        .compiler
                        .as_ref()
                        .ok_or_else(|| unavailable("selected worker compiler"))?
                        .registry_identity
                        .as_bytes()
                        .to_vec();
                    (identity, spec)
                }
            }
        } else {
            let recipe = self
                .interp
                .namespaces
                .borrow()
                .native_compiler_recipe(generation);
            match recipe {
                Some(
                    crate::namespace::NativeCompilerRecipe::Absent
                    | crate::namespace::NativeCompilerRecipe::ProcedureNoOp,
                ) => return Ok(Some(Operation::Invoke)),
                Some(crate::namespace::NativeCompilerRecipe::Registered { registration, spec }) => {
                    (registration, spec)
                }
                Some(crate::namespace::NativeCompilerRecipe::Unknown) | None => return Ok(None),
            }
        };
        let dialect = self.interp.native_invocation_dialect();
        let identity = std::str::from_utf8(&identity)
            .map_err(|_| unavailable("native body registered compiler identity"))?;
        // Private workers belong to the captured compiler recipe. Callable
        // origin following supplies only that separately checked dependency.
        let target_generation = self
            .interp
            .native_compiler_target(self.stamp.namespace, head)
            .map(|(_, _, generation, _)| generation)
            .unwrap_or(generation);
        if let Some(path) = spec
            .implementation_prerequisites(dialect)
            .filter(|_| selected_prerequisite.is_none())
        {
            if !self.interp.native_implementation_path_holds(
                target_generation,
                identity.as_bytes(),
                &path,
            ) {
                return Ok(Some(Operation::Invoke));
            }
        }
        let observed = {
            let traces = self.interp.traces.borrow();
            !traces.step_active.is_empty()
                || traces.cmd_traces.iter().any(|trace| {
                    trace.token == Some(generation)
                        && trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0
                })
        };
        if observed {
            return Ok(Some(Operation::Invoke));
        }
        let selection = selected_plan.as_ref().map_or_else(
            || spec.select_native_words(&captured, arguments_from, Some(dialect), self.context),
            |(selection, _)| *selection,
        );
        if let Some(kind) = spec.namespace_binding_kind() {
            let recipe = tcl_registry::native_namespace_binding_compilation::compile_native_namespace_bindings(
                &captured, arguments_from, self.stamp.physical, self.context, kind,
            ).map_err(|_| unavailable("native namespace binding compiler words"))?;
            if recipe.outcome == tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingOutcome::Unknown {
                return Ok(None);
            }
            return self
                .namespace_operation(&captured, recipe, depth)
                .map(|operation| Some(Operation::NamespaceBindings(operation)));
        }
        if matches!(
            spec.grammar,
            tcl_registry::native_compilation::NativeCompilationGrammar::Conditional
                | tcl_registry::native_compilation::NativeCompilationGrammar::ForLoop
                | tcl_registry::native_compilation::NativeCompilationGrammar::Catch
                | tcl_registry::native_compilation::NativeCompilationGrammar::WhileLoop
        ) {
            let recipe = tcl_registry::native_control_instructions::native_control_instruction(
                spec.grammar,
                &captured,
                arguments_from,
                dialect,
                self.context,
            )
            .map_err(|_| unavailable("native original control compiler"))?;
            if recipe.outcome
                == tcl_registry::native_control_compilation::NativeControlOutcome::Generic
            {
                let _prepared =
                    self.prepare_control_steps(&captured, &recipe.preparations, depth)?;
                return Ok(Some(Operation::Invoke));
            }
            return self
                .control_operation(&captured, recipe, depth)
                .map(|operation| {
                    Some(if operation.recipe.outcome
                    == tcl_registry::native_control_compilation::NativeControlOutcome::Generic {
                    Operation::Invoke
                } else { Operation::Control(Box::new(operation)) })
                });
        }
        if spec.grammar == tcl_registry::native_compilation::NativeCompilationGrammar::Try {
            let recipe = tcl_registry::native_try_compilation::compile_native_try(
                &captured,
                arguments_from,
                self.stamp.physical,
                self.context,
            )
            .map_err(|_| unavailable("native original try compiler"))?;
            return self.try_operation(&captured, recipe, depth).map(Some);
        }
        if spec.grammar == tcl_registry::native_compilation::NativeCompilationGrammar::Foreach {
            let recipe = tcl_registry::native_each_compilation::compile_native_each(
                &captured,
                arguments_from,
                self.stamp.physical,
                self.context,
                spec.each_collection()
                    .ok_or_else(|| unavailable("native iterator collection policy"))?,
            )
            .map_err(|_| unavailable("native original iterator compiler"))?;
            return self.each_operation(&captured, recipe, depth).map(Some);
        }
        if selection == NativeCompilationSelection::Generic {
            return Ok(Some(Operation::Invoke));
        }
        if !matches!(
            selection,
            NativeCompilationSelection::Inline { .. }
                | NativeCompilationSelection::NamedInvocation { .. }
        ) {
            return Ok(None);
        }
        let plan = match selected_plan {
            Some((_, plan)) => plan,
            None => match native_instruction_plan(
                spec,
                selection,
                &captured,
                arguments_from,
                dialect,
                self.context,
            ) {
                Ok(plan) => plan,
                Err(_) => return Ok(None),
            },
        };
        Ok(Some(match plan {
            NativeInstructionPlan::StringMatch(recipe) => {
                Operation::StringMatch(self.string_match_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Error(recipe) => {
                Operation::Error(self.error_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::DictionaryLookup(recipe) => Operation::DictionaryLookup(
                self.dictionary_lookup_operation(&captured, recipe, depth)?,
            ),
            NativeInstructionPlan::NamedInvocation(recipe) => Operation::NamedInvocation(
                self.named_operation(&captured, recipe, selected_prerequisite.as_ref(), depth)?,
            ),
            NativeInstructionPlan::Expression(recipe) => Operation::Expression(Box::new(
                self.expression_operation(&captured, recipe, depth)?,
            )),
            NativeInstructionPlan::Break => Operation::Break,
            NativeInstructionPlan::Continue => Operation::Continue,
            NativeInstructionPlan::Try(_) => {
                return Err(unavailable("native try preparation entry"));
            }
            NativeInstructionPlan::Control(_) => {
                return Err(unavailable("native control preparation entry"));
            }
            NativeInstructionPlan::Each(_) => {
                return Err(unavailable("native iterator preparation entry"));
            }
            NativeInstructionPlan::NamespaceBindings(_) => {
                return Err(unavailable("native namespace binding preparation entry"));
            }
            NativeInstructionPlan::TclOoHelper(recipe) => {
                use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction as Helper;
                let operands = match &recipe {
                    Helper::Next { words, .. } => words.as_slice(),
                    Helper::ObjectInfo { operand, .. } => std::slice::from_ref(operand),
                    Helper::SelfObject | Helper::SelfNamespace => &[],
                };
                let literals = operands.iter().map(|word|match word {
                        tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::LiteralExpansion{value,..}=>Some(self.literals.intern_bytes(value)),
                        _=>None,
                    }).collect();
                Operation::TclOoHelper(recipe, literals)
            }
            NativeInstructionPlan::Switch(recipe) => Operation::Switch(SwitchOperation {
                recipe,
                patterns: Vec::new(),
                subject_literal: None,
                empty: None,
            }),
            NativeInstructionPlan::Unset(recipe) => {
                Operation::Unset(self.unset_operation(&captured, recipe, depth)?)
            }
            NativeInstructionPlan::Load { target } => Operation::Load(self.target(target, depth)?),
            NativeInstructionPlan::Store { target, value_word } => {
                Operation::Store(self.target(target, depth)?, value_word)
            }
            NativeInstructionPlan::Increment {
                target,
                amount_word,
                immediate,
            } => Operation::Increment(self.target(target, depth)?, amount_word, immediate),
            NativeInstructionPlan::Append { target, recipe } => {
                Operation::Append(self.target(target, depth)?, recipe)
            }
            NativeInstructionPlan::List(recipe) => {
                let index = match &recipe {
                    NativeCompiledListRecipe::EmptyString => Some(self.literals.intern_bytes(b"")),
                    NativeCompiledListRecipe::PrivateConstant { members } => Some(
                        self.literals
                            .register_private_constant_list(members, self.stamp.source_protocol),
                    ),
                    NativeCompiledListRecipe::DynamicElements => None,
                };
                Operation::List(recipe, index)
            }
            NativeInstructionPlan::Concat(recipe) => {
                use tcl_registry::native_instruction_plan::{
                    NativeConcatInstruction, NativeConcatLiteralAllocation,
                };
                match recipe {
                    NativeConcatInstruction::Literal { bytes, allocation } => {
                        let index = match allocation {
                            NativeConcatLiteralAllocation::Registered => {
                                self.literals.intern_bytes(&bytes)
                            }
                            NativeConcatLiteralAllocation::PrivateEmpty => {
                                self.literals.register_unshared(&bytes)
                            }
                            NativeConcatLiteralAllocation::PrivateString => {
                                self.literals.register_private_concat_string(&bytes)
                            }
                        };
                        Operation::Concat {
                            literal: Some(index),
                            operands: Vec::new(),
                            prepared_words: HashMap::new(),
                        }
                    }
                    NativeConcatInstruction::Operands(operands) => {
                        let mut prepared_words = HashMap::new();
                        let operands = operands
                            .iter()
                            .map(|operand| {
                                self.namespace_operand(
                                    &captured,
                                    operand,
                                    &mut prepared_words,
                                    depth,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Operation::Concat {
                            literal: None,
                            operands,
                            prepared_words,
                        }
                    }
                }
            }
            NativeInstructionPlan::ReturnOptions(recipe) => {
                Operation::SelectedReturn(recipe, None, None)
            }
            NativeInstructionPlan::Uplevel(recipe) => {
                let default = recipe
                    .level_word
                    .is_none()
                    .then(|| self.literals.intern_bytes(b"1"));
                Operation::Uplevel(recipe, default)
            }
        }))
    }

    fn namespace_word(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        index: usize,
        command_head: bool,
        depth: u32,
    ) -> Result<WordInstruction, ValueError> {
        let original = captured
            .original_words()
            .get(index)
            .ok_or_else(|| unavailable("native namespace original word index"))?;
        let literal = if command_head {
            self.command_literal(captured)?
        } else {
            captured
                .literal(index)
                .map(|bytes| self.literals.intern_bytes(bytes))
        };
        let arena = if literal.is_none() {
            self.arena(original.executable_parts(), depth)?
        } else {
            ArenaInstruction {
                original: original.executable_parts().clone(),
                texts: HashMap::new(),
                locals: HashMap::new(),
                roots: HashMap::new(),
            }
        };
        Ok(WordInstruction {
            original: original.clone(),
            literal,
            arena,
        })
    }

    fn namespace_operand(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        operand: &tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand,
        prepared_words: &mut HashMap<usize, WordInstruction>,
        depth: u32,
    ) -> Result<NamespaceOperand, ValueError> {
        use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand;
        match operand {
            NativeCompilerWordOperand::Original(index) => {
                if !prepared_words.contains_key(index) {
                    let word = self.namespace_word(captured, *index, *index == 0, depth)?;
                    prepared_words.insert(*index, word);
                }
                Ok(NamespaceOperand::Original(*index))
            }
            NativeCompilerWordOperand::LiteralExpansion { value, .. } => {
                Ok(NamespaceOperand::Literal(self.literals.intern_bytes(value)))
            }
        }
    }

    /// Withdraw executable child publication from a declined compiler attempt.
    /// Its literal allocations and local declarations remain in this builder.
    fn rollback_word_publication(&mut self, original: &NativeWord) {
        for component in original.executable_parts().all_parts() {
            if let ExecutablePart::Command { body } = component.part {
                self.scripts
                    .retain(|region, _| region.start() < body.start() || region.end() > body.end());
            }
        }
    }

    fn namespace_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        recipe: tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
        depth: u32,
    ) -> Result<NamespaceOperation, ValueError> {
        use tcl_registry::native_namespace_binding_compilation::{
            NativeNamespaceBindingOutcome, NativeNamespaceBindingVisit,
        };
        let mut prepared_words = HashMap::new();
        let mut locals = HashMap::new();
        let mut operands = HashMap::new();
        let mut prefix = None;
        for visit in &recipe.visits {
            match visit {
                NativeNamespaceBindingVisit::Literal(value) => {
                    prefix = Some(self.literals.intern_bytes(value));
                }
                NativeNamespaceBindingVisit::DeclareLocal(name) => {
                    let protocol = self
                        .interp
                        .native_invocation_dialect()
                        .native_compiled_variable_protocol()
                        .ok_or_else(|| {
                            unavailable("native namespace local declaration protocol")
                        })?;
                    locals.insert(name.clone(), self.lvt.intern_native(protocol, name));
                }
                NativeNamespaceBindingVisit::Word(operand) => {
                    let value =
                        self.namespace_operand(captured, operand, &mut prepared_words, depth)?;
                    operands.insert(operand.clone(), value);
                }
            }
        }
        let mut bindings = Vec::new();
        let generic = if recipe.outcome == NativeNamespaceBindingOutcome::Generic {
            let projected =
                tcl_registry::native_compiler_word_projection::project_native_compiler_words(
                    captured,
                    self.stamp.physical,
                )
                .map_err(|_| unavailable("native namespace fallback parser words"))?;
            let mut generic = Vec::with_capacity(projected.len());
            for word in projected {
                if let tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(index) = &word.operand {
                    if prepared_words.contains_key(index) {
                        self.rollback_word_publication(&captured.original_words()[*index]);
                        let prepared = self.namespace_word(captured, *index, *index == 0, depth)?;
                        prepared_words.insert(*index, prepared);
                    }
                }
                generic.push(self.namespace_operand(
                    captured,
                    &word.operand,
                    &mut prepared_words,
                    depth,
                )?);
            }
            Some(generic)
        } else {
            for binding in &recipe.bindings {
                bindings.push(NamespaceBindingInstruction {
                    name: operands
                        .remove(&binding.name)
                        .ok_or_else(|| unavailable("native namespace name preparation"))?,
                    slot: *locals
                        .get(&binding.local)
                        .ok_or_else(|| unavailable("native namespace local preparation"))?,
                    local: binding.local.clone(),
                    value: binding
                        .value
                        .as_ref()
                        .map(|value| {
                            operands
                                .remove(value)
                                .ok_or_else(|| unavailable("native namespace value preparation"))
                        })
                        .transpose()?,
                });
            }
            None
        };
        let namespace = if generic.is_none() {
            recipe
                .namespace
                .as_ref()
                .map(|operand| {
                    operands
                        .remove(operand)
                        .ok_or_else(|| unavailable("native original namespace operand preparation"))
                })
                .transpose()?
        } else {
            None
        };
        let empty = generic.is_none().then(|| self.literals.intern_bytes(b""));
        Ok(NamespaceOperation {
            kind: recipe.kind,
            bindings,
            generic,
            prepared_words,
            prefix,
            namespace,
            empty,
        })
    }

    fn script(&mut self, region: Span, depth: u32) -> Result<(), ValueError> {
        if self.scripts.contains_key(&region) {
            return Ok(());
        }
        if NATIVE_EVAL_DEPTH_LIMIT.exceeded(depth) {
            return Err(unavailable("native body compiler script depth"));
        }
        let plan =
            tcl_lexer::native_script_words_in(self.image.clone(), region, self.stamp.grammar)
                .map_err(|_| unavailable("native body executable script geometry"))?;
        if plan.fatal_tail.is_some() && self.stamp.physical == tcl_dialect::TclVersion::V8_4 {
            // C8.4 has no executable parse-error instruction to publish.
            self.parse_failure = plan.fatal_tail;
            return Err(ValueError::CommandProtocolUnavailable(
                "native C8.4 failed body parse",
            ));
        }
        let mut commands = Vec::with_capacity(plan.commands.len());
        for command in plan.commands {
            let Some(mut operation) = self.operation(&command.words, depth)? else {
                return Err(unavailable("native body registered instruction capability"));
            };
            let capture = NativeCompilerWords::capture(&command.words, self.stamp.source_protocol)
                .map_err(|_| unavailable("native body original word value geometry"))?;
            let mut words = Vec::with_capacity(command.words.len());
            for (index, original) in command.words.iter().enumerate() {
                if let Operation::Expression(expression) = &mut operation {
                    if let Some(word) = expression.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Try(control) = &mut operation {
                    if let Some(word) = control.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Control(control) = &mut operation {
                    if let Some(word) = control.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Each(each) = &mut operation {
                    if let Some(word) = each.prepared.words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::NamespaceBindings(scope) = &mut operation {
                    if let Some(word) = scope.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Concat { prepared_words, .. } = &mut operation {
                    if let Some(word) = prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::NamedInvocation(named) = &mut operation {
                    if let Some(word) = named.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Error(error) = &mut operation {
                    if let Some(word) = error.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::StringMatch(matcher) = &mut operation {
                    if let Some(word) = matcher.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::DictionaryLookup(dictionary) = &mut operation {
                    if let Some(word) = dictionary.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                if let Operation::Unset(unset) = &mut operation {
                    if let Some(word) = unset.prepared_words.remove(&index) {
                        words.push(word);
                        continue;
                    }
                }
                // Opcode variable operands are not emitted as ordinary argument
                // objects. Their indexed receiver layout was allocated above.
                let emitted = match &operation {
                    Operation::StringMatch(_) => false,
                    Operation::Error(_) | Operation::DictionaryLookup(_) | Operation::Unset(_) => false,
                    Operation::TclOoHelper(tcl_registry::native_tcloo_compilation::NativeTclOoInstruction::Next{words,..},_)=>words.iter().any(|word|matches!(word,tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(original) if index==*original)),
                    Operation::TclOoHelper(tcl_registry::native_tcloo_compilation::NativeTclOoInstruction::ObjectInfo{operand,..},_)=>matches!(operand,tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(original) if index==*original),
                    Operation::Invoke => true,
                    Operation::Load(target) => {
                        index == 1
                            && matches!(target.original, NativeVariableWordOperand::DynamicWord)
                    }
                    Operation::Store(target, value) => {
                        index == *value
                            || (index == 1
                                && matches!(
                                    target.original,
                                    NativeVariableWordOperand::DynamicWord
                                ))
                    }
                    Operation::Increment(target, amount, immediate) => {
                        (index == 1
                            && matches!(target.original, NativeVariableWordOperand::DynamicWord))
                            || (Some(index) == *amount && immediate.is_none())
                    }
                    Operation::Append(target, recipe) => {
                        recipe.values.contains(&index)
                            || (index == 1
                                && matches!(
                                    target.original,
                                    NativeVariableWordOperand::DynamicWord
                                ))
                    }
                    Operation::List(NativeCompiledListRecipe::DynamicElements, _) => index > 0,
                    Operation::Concat { operands, .. } => operands.iter().any(|operand| matches!(operand, NamespaceOperand::Original(word) if index == *word)),
                    Operation::Switch(switch) => matches!(
                        switch.recipe.subject,
                        tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(word)
                            if index == word
                    ),
                    Operation::SelectedReturn(recipe,_,_) => {
                        Some(index)==recipe.value_word || match &recipe.options {
                            tcl_registry::native_return_compilation::NativeReturnOptionsOperand::StackWord(word)=>index==*word,
                            tcl_registry::native_return_compilation::NativeReturnOptionsOperand::StackPairs(range)=>range.contains(&index),
                            _=>false,
                        }
                    }
                    Operation::Uplevel(recipe, _) => Some(index) == recipe.level_word || recipe.script_words.contains(&index),
                    _ => false,
                };
                let literal = if emitted && matches!(operation, Operation::Invoke) && index == 0 {
                    self.command_literal(&capture)?
                } else if emitted {
                    capture
                        .literal(index)
                        .map(|bytes| self.literals.intern_bytes(bytes))
                } else {
                    None
                };
                let arena = if emitted && literal.is_none() {
                    self.arena(original.executable_parts(), depth)?
                } else {
                    ArenaInstruction {
                        original: original.executable_parts().clone(),
                        texts: HashMap::new(),
                        locals: HashMap::new(),
                        roots: HashMap::new(),
                    }
                };
                words.push(WordInstruction {
                    original: original.clone(),
                    literal,
                    arena,
                });
            }
            if let Operation::Switch(switch) = &mut operation {
                self.prepare_body_switch(switch, depth)?;
            }
            if let Operation::SelectedReturn(recipe, empty, options) = &mut operation {
                if recipe.value_word.is_none() {
                    *empty = Some(self.literals.intern_bytes(b""));
                }
                if recipe.exit
                    == tcl_registry::native_return_compilation::NativeReturnExit::Immediate
                {
                    let tcl_registry::native_return_compilation::NativeReturnOptionsOperand::Static(
                        literal,
                    ) = &recipe.options
                    else {
                        unreachable!("static Return")
                    };
                    *options = Some(
                        self.literals
                            .register_private_return_options(literal.clone()),
                    );
                }
            }
            commands.push(CommandInstruction {
                span: command.span,
                words,
                operation,
            });
        }
        let empty = if commands.is_empty() && plan.fatal_tail.is_none() {
            Some(
                if tcl_runtime_api::native_literal::source_literal_empty_result_is_unshared(
                    self.stamp.source_protocol,
                ) {
                    self.literals.register_unshared(b"")
                } else {
                    self.literals.intern_bytes(b"")
                },
            )
        } else {
            None
        };
        let fatal = plan
            .fatal_tail
            .map(|cut| self.syntax_instruction(cut))
            .transpose()?;
        self.scripts.insert(
            region,
            Script {
                commands,
                fatal,
                empty,
            },
        );
        Ok(())
    }
}

static NEXT_LAYOUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn next_layout_token() -> u64 {
    tcl_runtime_api::checked_counter::allocate(&NEXT_LAYOUT)
        .expect("native compiled layout tokens exhausted")
}

pub(crate) fn cache_snapshot(
    value: *mut TclObj,
) -> Option<tcl_syntax::native_object::NativeObjectCacheSnapshot> {
    cache(value).map(
        |artifact| tcl_syntax::native_object::NativeObjectCacheSnapshot::Bytecode {
            version: artifact.stamp.physical,
        },
    )
}

fn unavailable(purpose: &'static str) -> ValueError {
    ValueError::CommandProtocolUnavailable(purpose)
}

impl Interp {
    pub(super) fn original_procedure_artifact_is_current(&self, procedure: &Rc<ProcDef>) -> bool {
        let Ok(original) = procedure.body.checked_ptr() else {
            return false;
        };
        let Some(current) = cache(original) else {
            return false;
        };
        matches!(&current.owner, BodyContext::Procedure(owner) if Weak::ptr_eq(owner, &Rc::downgrade(procedure)))
            && self.native_body_stamp(procedure.namespace(), true).as_ref() == Some(&current.stamp)
    }

    fn native_body_stamp(&self, namespace: NsId, procedure: bool) -> Option<CacheStamp> {
        let traces = self.traces.borrow();
        Some(CacheStamp {
            interpreter: self.native_command_interpreter,
            namespace,
            epochs: self.native_compiler_cache_epochs(namespace)?,
            physical: self.native_invocation_dialect().tcl_version?,
            grammar: self.lexer_config(),
            source_protocol: self.source_string_protocol()?,
            fixed_math: self.native_math_function_table(),
            observers: traces
                .cmd_traces
                .iter()
                .filter(|trace| trace.ops & crate::cmd_trace::ops::EXEC_ANY != 0)
                .map(|trace| (trace.id, trace.token, trace.ops))
                .collect(),
            steps: traces
                .step_active
                .iter()
                .map(|trace| (trace.token, trace.ops, trace.command.clone()))
                .collect(),
            borrowed_table: if !procedure && self.frames.borrow().in_proc() {
                Some(Rc::as_ptr(self.frames.borrow().native_local_name_table()?) as usize)
            } else {
                None
            },
        })
    }

    /// Prepare a complete executable artifact before installing its native
    /// primary. Unsupported registered instructions preserve the interpreter's
    /// existing uncached path; unavailable geometry is a typed host refusal.
    pub(super) fn prepare_original_c_body(
        &mut self,
        original: *mut TclObj,
        namespace: NsId,
        procedure: Option<&Rc<ProcDef>>,
    ) -> Result<Option<Rc<NativeBodyArtifact>>, Code> {
        if let Some(procedure) = procedure {
            let live = procedure
                .body
                .checked_ptr()
                .map_err(|error| self.report_cmd_error(error.into()))?;
            if live != original {
                return Err(self.report_cmd_error(
                    unavailable("native procedure body original identity").into(),
                ));
            }
        }
        let Some(mut stamp) = self.native_body_stamp(namespace, procedure.is_some()) else {
            return Ok(None);
        };
        let matches_owner = |owner: &BodyContext| match (owner, procedure) {
            (BodyContext::Script, None) => true,
            (BodyContext::Procedure(owner), Some(procedure)) => {
                Weak::ptr_eq(owner, &Rc::downgrade(procedure))
            }
            _ => false,
        };
        if let Some(current) = cache(original)
            .filter(|current| current.stamp == stamp && matches_owner(&current.owner))
        {
            return Ok(Some(current));
        }
        let bytes = ValueOps::native_string_bytes(self, &original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if cache(original).is_some() {
            obj::change_type(original, core::ptr::null(), 0);
        }
        stamp = self
            .native_body_stamp(namespace, procedure.is_some())
            .ok_or_else(|| {
                self.report_cmd_error(unavailable("native body source callback context").into())
            })?;
        let image = SourceImage::native(bytes.as_ref());
        let end = u32::try_from(image.len())
            .map_err(|_| self.report_cmd_error(unavailable("native body source extent").into()))?;
        let region = Span::new(0, end);
        if stamp.physical == tcl_dialect::TclVersion::V8_4 {
            let plan = tcl_lexer::native_script_words_in(image.clone(), region, stamp.grammar)
                .map_err(|_| {
                    self.report_cmd_error(
                        unavailable("native body executable script geometry").into(),
                    )
                })?;
            if let Some(fatal) = plan.fatal_tail {
                // C8.4 parses the complete compiled body before entering its
                // first command. A later malformed command therefore prevents
                // an earlier return from executing and leaves no Bytecode.
                return Err(self.error(fatal.cut.message.as_bytes()));
            }
        }
        let context = Context {
            mode: NativeCompilationMode::BytecodeObject,
            frame: if procedure.is_some() {
                NativeCompilationFrame::ProcedureCode
            } else {
                NativeCompilationFrame::ScriptCode
            },
            loop_depth: 0,
            catch_depth: Some(0),
        };
        let names: Vec<_> = procedure.map_or_else(Vec::new, |procedure| {
            procedure
                .params
                .iter()
                .map(|param| tcl_runtime_api::NameBytes::from(param.name.as_slice()))
                .collect()
        });
        let borrowed_table = if stamp.borrowed_table.is_some() {
            self.frames.borrow().native_local_name_table().cloned()
        } else {
            None
        };
        let borrowed_names = borrowed_table
            .as_ref()
            .map(|table| {
                table
                    .names
                    .iter()
                    .map(|name| {
                        name.as_ref()
                            .map(|name| {
                                ValueOps::native_string_bytes(self, &name.as_ptr())
                                    .map(|bytes| tcl_runtime_api::NameBytes::from(bytes.as_ref()))
                            })
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, ValueError>>()
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        // Canonical name materialisation may call a native updater. The same
        // table and physical compiler context must still own the compilation.
        if self
            .native_body_stamp(namespace, procedure.is_some())
            .as_ref()
            != Some(&stamp)
        {
            return Err(
                self.report_cmd_error(unavailable("native body borrowed table changed").into())
            );
        }
        let mut lvt = borrowed_names.as_ref().map_or_else(
            || LocalVarTable::from_native_names(&names),
            |names| LocalVarTable::from_native_slot_names(names),
        );
        lvt.set_native_protocol(
            self.native_invocation_dialect()
                .native_compiled_variable_protocol(),
        );
        let mut builder = Builder {
            interp: self,
            image: image.clone(),
            context,
            stamp: stamp.clone(),
            lvt,
            literals: LiteralTable::new(),
            scripts: HashMap::new(),
            parse_failure: None,
            compilation_failure: None,
            private_objects: HashMap::new(),
        };
        if let Err(error) = builder.script(region, 0) {
            if let Some(failure) = builder.compilation_failure {
                return Err(builder.interp.report_cmd_error(
                    tcl_cmd_core::CmdError::from_byte_details(tcl_cmd_core::CmdErrorDetails {
                        message: failure.message.unwrap_or_default().into_bytes(),
                        string_result: None,
                        error_code: tcl_cmd_core::CmdErrorCodeUpdate::Set(
                            failure
                                .error_code
                                .unwrap_or_else(|| "NONE".to_owned())
                                .into_bytes(),
                        ),
                        error_info: failure.error_info.map(String::into_bytes),
                        error_line: None,
                        primitive_getter: None,
                    }),
                ));
            }
            if let Some(fatal) = builder.parse_failure {
                return Err(builder.interp.error(fatal.cut.message.as_bytes()));
            }
            if matches!(
                &error,
                ValueError::CommandProtocolUnavailable(
                    "native body registered instruction capability"
                )
            ) {
                return Ok(None);
            }
            return Err(builder.interp.report_cmd_error(error.into()));
        }
        let entries: Vec<_> = builder
            .literals
            .entries()
            .iter()
            .enumerate()
            .map(|(index, literal)| {
                Ok(match literal.allocation() {
                    NativeLiteralAllocation::PrivateLogicalBoolean85(value) => {
                        NativeRuntimeLiteral::PrivateLogicalBoolean85(*value)
                    }
                    NativeLiteralAllocation::PrivateInteger(value) => {
                        NativeRuntimeLiteral::UnsharedOriginal(obj::Owned::fresh(
                            obj::new_wide_int_obj(*value),
                        ))
                    }
                    NativeLiteralAllocation::PrivateExpressionNumber { version, value } => {
                        if *version != stamp.physical || *version < tcl_dialect::TclVersion::V8_5 {
                            return Err(unavailable("native folded-number literal issuer"));
                        }
                        let original = obj::Owned::fresh(obj::new_string_bytes(b""));
                        let protocol = tcl_registry::InvocationDialect::for_version(*version)
                            .native_scalar_getter_protocol()
                            .ok_or_else(|| unavailable("native folded-number literal producer"))?;
                        obj::adopt_native_scalar_cache(
                            original.as_ptr(),
                            tcl_syntax::scalar_getter::NativeScalarCache::Number(value.number()),
                            protocol,
                        )?;
                        obj::invalidate_string(original.as_ptr());
                        NativeRuntimeLiteral::UnsharedOriginal(original)
                    }
                    NativeLiteralAllocation::PrivateConstantList { members, protocol } => {
                        NativeRuntimeLiteral::PrivateConstantList {
                            members: members.clone(),
                            protocol: *protocol,
                        }
                    }
                    NativeLiteralAllocation::Unshared => {
                        NativeRuntimeLiteral::UnsharedBytes(literal.bytes().to_vec())
                    }
                    NativeLiteralAllocation::PrivateConcatString => {
                        NativeRuntimeLiteral::PrivateConcatString(literal.bytes().to_vec())
                    }
                    NativeLiteralAllocation::PrivateReturnOptions(recipe) => {
                        NativeRuntimeLiteral::UnsharedOriginal(
                            crate::native_return_merge::manufacture(recipe).map_err(|error| {
                                let _ = error;
                                unavailable("native private Return literal manufacture")
                            })?,
                        )
                    }
                    NativeLiteralAllocation::PrivateOriginal => {
                        NativeRuntimeLiteral::UnsharedOriginal(
                            builder.private_objects.remove(&index).ok_or_else(|| {
                                unavailable("native private original literal lacks supplied owner")
                            })?,
                        )
                    }
                    NativeLiteralAllocation::RegisteredNativeCommand {
                        context,
                        fully_qualified,
                    } => NativeRuntimeLiteral::RegisteredBytes {
                        bytes: literal.bytes().to_vec(),
                        namespace: tcl_runtime_api::native_literal::command_literal_partition(
                            stamp.source_protocol,
                            tcl_core_types::NsId(
                                u32::try_from(context.namespace_token).map_err(|_| {
                                    unavailable("native literal namespace partition")
                                })?,
                            ),
                            *fully_qualified,
                        ),
                    },
                    NativeLiteralAllocation::RegisteredData => {
                        NativeRuntimeLiteral::RegisteredBytes {
                            bytes: literal.bytes().to_vec(),
                            namespace: None,
                        }
                    }
                    NativeLiteralAllocation::RegisteredCommand { .. } => {
                        return Err(unavailable("native literal lacks original namespace token"));
                    }
                })
            })
            .collect::<Result<Vec<_>, ValueError>>()
            .map_err(|error| builder.interp.report_cmd_error(error.into()))?;
        let actions: Vec<_> = builder
            .literals
            .native_actions()
            .iter()
            .map(|action| match action {
                NativeLiteralAction::RetainSyntaxErrorInfo { options, message } => {
                    NativeRuntimeLiteralAction::RetainSyntaxErrorInfo {
                        options: *options,
                        message: *message,
                    }
                }
                NativeLiteralAction::Register(index) => {
                    NativeRuntimeLiteralAction::Register(*index)
                }
                NativeLiteralAction::AdoptExpressionNumber {
                    index,
                    version,
                    value,
                } => NativeRuntimeLiteralAction::AdoptExpressionNumber {
                    index: *index,
                    version: *version,
                    value: value.clone(),
                },
                NativeLiteralAction::Hide(index) => NativeRuntimeLiteralAction::Hide(*index),
                NativeLiteralAction::PrimeExpressionBoolean84(index) => {
                    NativeRuntimeLiteralAction::PrimeExpressionBoolean84(*index)
                }
                NativeLiteralAction::PrimeCommandName { index, receipt } => {
                    NativeRuntimeLiteralAction::PrimeCommandName {
                        index: *index,
                        receipt: receipt.clone(),
                    }
                }
            })
            .collect();
        let literals = builder
            .interp
            .create_native_literal_array_with_actions(original, &entries, &actions)
            .map_err(|error| builder.interp.report_cmd_error(error.into()))?;
        if builder
            .interp
            .native_body_stamp(namespace, procedure.is_some())
            .as_ref()
            != Some(&stamp)
        {
            return Err(builder.interp.report_cmd_error(
                unavailable("native body compiler context changed during publication").into(),
            ));
        }
        let locals = procedure.map(|_| NativeCompiledLocalLayout {
            owner: stamp.interpreter,
            token: next_layout_token(),
            epoch: stamp.epochs.0,
            kind: NativeCompiledLocalLayoutKind::Procedure,
            names: builder.lvt.native_slot_names(),
        });
        let artifact = Rc::new(NativeBodyArtifact {
            stamp,
            owner: procedure.map_or(BodyContext::Script, |procedure| {
                BodyContext::Procedure(Rc::downgrade(procedure))
            }),
            image,
            region,
            scripts: builder.scripts,
            literals,
            locals,
            _borrowed_table: borrowed_table.as_ref().map(Rc::downgrade),
        });
        if let (Some(procedure), Some(layout)) = (procedure, artifact.locals.as_ref()) {
            procedure.retain_native_compiled_names(layout);
        }
        install(original, &artifact);
        Ok(Some(artifact))
    }

    pub(super) fn execute_original_c_body(
        &mut self,
        artifact: &Rc<NativeBodyArtifact>,
        frame: CmdFrame,
    ) -> Code {
        if !self.codegen_activation_enter() {
            return Code::Error;
        }
        if let Err(code) = self.reset_native_ensemble_rewrite(
            tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
        ) {
            self.codegen_activation_leave(code);
            return code;
        }
        self.cmd_frames.borrow_mut().push(frame);
        let mut execution = BodyExecution::default();
        let code = self.execute_body_region(artifact, artifact.region, &mut execution);
        let code = if execution.done && code == Code::Return {
            Code::Ok
        } else {
            code
        };
        self.cmd_frames.borrow_mut().pop();
        self.codegen_activation_leave(code);
        code
    }

    fn execute_body_region(
        &mut self,
        artifact: &NativeBodyArtifact,
        region: Span,
        execution: &mut BodyExecution,
    ) -> Code {
        let script = artifact
            .scripts
            .get(&region)
            .expect("admitted original script instruction region");
        if let Some(empty) = script.empty {
            self.set_result(
                artifact
                    .literals
                    .original(empty)
                    .expect("emitted empty script literal"),
            );
        }
        for command in &script.commands {
            if let Some(frame) = self.cmd_frames.borrow_mut().last_mut() {
                frame.line = frame.line_base
                    + line_of(artifact.image.bytes(), command.span.start() as usize);
                frame.cmd = artifact.image.bytes()[command.span.as_range()].to_vec();
                frame.original_command = None;
            }
            let code = self.execute_body_instruction(artifact, command, execution);
            if execution.done {
                return code;
            }
            if code != Code::Ok || self.host_refusal_pending() {
                if code == Code::Error && !self.host_refusal_pending() {
                    self.log_command_bytes(
                        line_of(artifact.image.bytes(), command.span.start() as usize),
                        &artifact.image.bytes()[command.span.as_range()],
                    );
                }
                return if self.host_refusal_pending() {
                    Code::Error
                } else {
                    code
                };
            }
        }
        if let Some(fatal) = &script.fatal {
            let options = artifact
                .literals
                .original(fatal.options)
                .expect("emitted original SYNTAX options");
            let code = match self.process_original_c_return_options(
                tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                1,
                0,
                options,
            ) {
                Ok(code) => code,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            if code != Code::Ok {
                self.set_result(
                    artifact
                        .literals
                        .original(fatal.message)
                        .expect("emitted original SYNTAX result"),
                );
                self.clear_error_logged();
                if code == Code::Error {
                    self.capture_original_return_instruction_context(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax,
                        self.result.get(),
                        options,
                    );
                }
            }
            if code == Code::Error {
                self.log_command_bytes(
                    line_of(artifact.image.bytes(), fatal.cut.command_start as usize),
                    &artifact.image.bytes()
                        [fatal.cut.command_start as usize..region.end() as usize],
                );
            }
            return code;
        }
        Code::Ok
    }

    fn body_word(
        &mut self,
        artifact: &NativeBodyArtifact,
        word: &WordInstruction,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        if let Some(index) = word.literal {
            return Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(index)
                    .expect("admitted PUSH literal"),
            ));
        }
        self.body_arena(artifact, &word.arena, execution)
    }

    fn body_namespace_operand(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operand: &NamespaceOperand,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        match operand {
            NamespaceOperand::Original(index) => {
                self.body_word(artifact, &command.words[*index], execution)
            }
            NamespaceOperand::Literal(index) => Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(*index)
                    .expect("original parser-expanded namespace literal"),
            )),
        }
    }

    fn body_arena(
        &mut self,
        artifact: &NativeBodyArtifact,
        arena: &ArenaInstruction,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        // One component passes its actual object through; concatenation reaches
        // the native getter only when multiple components require bytes.
        let mut pending: Vec<NativeArenaFrame> = vec![(arena.original.root(), 0, Vec::new(), None)];
        loop {
            let frame = pending.last_mut().expect("native word root");
            let Some(component) = arena.original.list(frame.0).get(frame.1) else {
                let (_, _, values, _) = pending.pop().expect("completed native operand");
                let value = self.concatenate_body_values(values)?;
                let Some(parent) = pending.last_mut() else {
                    return Ok(value);
                };
                let (name, slot) = parent.3.take().expect("actual array index owner");
                let root = arena.original.bytes(name).expect("retained variable root");
                let read = self.body_read_original_parts(
                    &EvaluatedTarget {
                        root: root.to_vec(),
                        element: None,
                        original_name: arena.roots.get(&name).map(|index| {
                            obj::Owned::retain(
                                artifact
                                    .literals
                                    .original(*index)
                                    .expect("emitted variable base"),
                            )
                        }),
                        original_index: Some(value),
                        combined: false,
                    },
                    slot,
                )?;
                parent.2.push(read);
                continue;
            };
            frame.1 += 1;
            let value = match &component.part {
                ExecutablePart::Text(_) => obj::Owned::retain(
                    artifact
                        .literals
                        .original(
                            *arena
                                .texts
                                .get(&component.span)
                                .expect("actual emitted TEXT literal"),
                        )
                        .expect("actual TEXT object"),
                ),
                ExecutablePart::Variable {
                    name,
                    index: Some(index),
                } => {
                    frame.3 = Some((*name, arena.locals.get(name).copied()));
                    pending.push((*index, 0, Vec::new(), None));
                    continue;
                }
                ExecutablePart::Variable { name, index: None } => self.body_read_original_parts(
                    &EvaluatedTarget {
                        root: arena
                            .original
                            .bytes(*name)
                            .expect("retained variable root")
                            .to_vec(),
                        element: None,
                        original_name: arena.roots.get(name).map(|index| {
                            obj::Owned::retain(
                                artifact
                                    .literals
                                    .original(*index)
                                    .expect("emitted variable name"),
                            )
                        }),
                        original_index: None,
                        combined: true,
                    },
                    arena.locals.get(name).copied(),
                )?,
                ExecutablePart::Command { body } => {
                    let saved = self.cmd_frames.borrow().last().map(|frame| {
                        (
                            frame.line,
                            frame.cmd.clone(),
                            frame.original_command.clone(),
                        )
                    });
                    let code = self.execute_body_region(artifact, *body, execution);
                    if let (Some((line, cmd, original)), Some(frame)) =
                        (saved, self.cmd_frames.borrow_mut().last_mut())
                    {
                        frame.line = line;
                        frame.cmd = cmd;
                        frame.original_command = original;
                    }
                    if code != Code::Ok {
                        return Err(code);
                    }
                    obj::Owned::retain(self.result_obj())
                }
                ExecutablePart::ParseError(message) => return Err(self.error(message.as_bytes())),
                ExecutablePart::Expression { .. } => unreachable!("unadmitted expression sugar"),
            };
            frame.2.push(value);
        }
    }

    fn concatenate_body_values(&mut self, mut values: Vec<obj::Owned>) -> Result<obj::Owned, Code> {
        if values.len() == 1 {
            return Ok(values.pop().expect("sole component"));
        }
        let mut bytes = Vec::new();
        for value in values {
            bytes.extend_from_slice(
                &ValueOps::native_string_bytes(self, &value.as_ptr())
                    .map_err(|error| self.report_cmd_error(error.into()))?,
            );
        }
        Ok(obj::Owned::fresh(obj::new_string_bytes(&bytes)))
    }

    fn body_target(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        target: &Target,
        execution: &mut BodyExecution,
    ) -> Result<EvaluatedTarget, Code> {
        self.body_target_at(artifact, command, target, 1, execution)
    }

    fn body_target_at(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        target: &Target,
        word: usize,
        execution: &mut BodyExecution,
    ) -> Result<EvaluatedTarget, Code> {
        let original_name = target.root_literal.map(|index| {
            obj::Owned::retain(
                artifact
                    .literals
                    .original(index)
                    .expect("emitted base name PUSH"),
            )
        });
        match &target.original {
            NativeVariableWordOperand::Literal { name, index, .. } => Ok(EvaluatedTarget {
                root: name.clone(),
                element: index.clone(),
                original_name,
                original_index: target.index_literal.map(|index| {
                    obj::Owned::retain(
                        artifact
                            .literals
                            .original(index)
                            .expect("emitted index PUSH"),
                    )
                }),
                combined: false,
            }),
            NativeVariableWordOperand::CompoundArray { name, .. } => {
                let value = self.body_arena(
                    artifact,
                    target.index.as_ref().expect("emitted array index"),
                    execution,
                )?;
                Ok(EvaluatedTarget {
                    root: name.clone(),
                    element: None,
                    original_name,
                    original_index: Some(value),
                    combined: false,
                })
            }
            NativeVariableWordOperand::DynamicWord => {
                let value = self.body_word(artifact, &command.words[word], execution)?;
                Ok(EvaluatedTarget {
                    root: Vec::new(),
                    element: None,
                    original_name: Some(value),
                    original_index: None,
                    combined: true,
                })
            }
        }
    }

    fn body_capture_target(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        creates: bool,
    ) -> Result<super::native_variable_names::OriginalCVariableCapture, Code> {
        let root = evaluated.root.as_slice();
        if let Some(slot) = slot {
            // The original PUSH operands are materialised only when the
            // variable instruction executes, after later value/amount words.
            let element = evaluated
                .original_index
                .as_ref()
                .map(|value| {
                    ValueOps::native_string_bytes(self, &value.as_ptr()).map(|bytes| bytes.to_vec())
                })
                .transpose()
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let captured = crate::vars::capture_original_indexed_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                slot,
                element.clone(),
                creates,
            );
            let captured = captured
                .map_err(|error| crate::builtins::var_error(self, root, error))?
                .ok_or_else(|| self.no_such_variable(root, element.as_deref()))?;
            if creates
                && self
                    .native_invocation_dialect()
                    .tcl_version
                    .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
            {
                captured.0.retain_original_element_key(
                    evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                );
            }
            Ok(super::native_variable_names::OriginalCVariableCapture {
                receiver: captured.0,
                home: captured.1,
                root: root.to_vec(),
                element,
            })
        } else {
            use tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose as Purpose;
            if evaluated.combined {
                return self
                    .capture_original_c_variable_report(
                        evaluated
                            .original_name
                            .as_ref()
                            .expect("emitted variable name")
                            .as_ptr(),
                        if creates {
                            Purpose::Write
                        } else {
                            Purpose::Read
                        },
                    )?
                    .ok_or_else(|| self.no_such_variable(root, evaluated.element.as_deref()));
            }
            self.capture_original_c_parts_report(
                evaluated
                    .original_name
                    .as_ref()
                    .expect("emitted stack base name")
                    .as_ptr(),
                evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                if creates {
                    Purpose::Write
                } else {
                    Purpose::Read
                },
            )?
            .ok_or_else(|| self.no_such_variable(root, evaluated.element.as_deref()))
        }
    }

    fn body_read_original_parts(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
    ) -> Result<obj::Owned, Code> {
        let capture = self.body_capture_target(evaluated, slot, false)?;
        let root = capture.root.as_slice();
        let element = capture.element.as_deref();
        let receiver = capture.receiver;
        let home = capture.home;
        if self.has_variable_traces() {
            let access = self.trace_access(root, root, element, &home, false);
            if self.fire_var_trace_resolved(&home, &access, b"read") {
                return Err(crate::builtins::var_error(
                    self,
                    root,
                    crate::frame::VarError::TraceError,
                ));
            }
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        let value = receiver
            .read()
            .map_err(|error| crate::builtins::var_error(self, root, error))?
            .ok_or_else(|| self.no_such_variable(root, element))?;
        Ok(obj::Owned::retain(value))
    }

    fn body_store(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        value: &obj::Owned,
    ) -> Code {
        let capture = match self.body_capture_target(evaluated, slot, true) {
            Ok(captured) => captured,
            Err(code) => return code,
        };
        let root = capture.root.as_slice();
        let element = capture.element.as_deref();
        let receiver = capture.receiver;
        let home = capture.home;
        if let Err(error) = receiver.store(value.as_ptr()) {
            return crate::builtins::var_error(self, root, error);
        }
        if self.has_variable_traces() {
            let access = self.trace_access(root, root, element, &home, false);
            if self.fire_var_trace_resolved(&home, &access, b"write") {
                return crate::builtins::var_error(self, root, crate::frame::VarError::TraceError);
            }
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        match receiver.read() {
            Ok(Some(value)) => self.set_result(value),
            _ => self.set_result_bytes(b""),
        }
        Code::Ok
    }

    fn body_argument_list(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        steps: &[NativeArgumentListStep],
        strip_single_expanded: bool,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        let mut stack = Vec::<obj::Owned>::new();
        for step in steps {
            match *step {
                NativeArgumentListStep::Word(index) => {
                    stack.push(self.body_word(artifact, &command.words[index], execution)?);
                }
                NativeArgumentListStep::List(count) => {
                    let operands = stack.split_off(stack.len() - count);
                    let members: Vec<_> = operands.iter().map(obj::Owned::as_ptr).collect();
                    stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(
                        &members,
                        artifact.stamp.source_protocol,
                    )));
                }
                NativeArgumentListStep::Concat => {
                    let source = stack.pop().expect("argument List source");
                    let target = stack.pop().expect("argument List target");
                    let value = crate::list::concatenate_native_lists(
                        target.as_ptr(),
                        source.as_ptr(),
                        artifact.stamp.source_protocol,
                    )
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                    stack.push(value);
                }
            }
        }
        let value = stack.pop().expect("native argument List result");
        debug_assert!(stack.is_empty());
        if strip_single_expanded {
            crate::list::native_full_list_range(value.as_ptr(), artifact.stamp.source_protocol)
                .map_err(|error| self.report_cmd_error(error.into()))
        } else {
            Ok(value)
        }
    }

    fn body_publish_append(
        &mut self,
        capture: super::native_variable_names::OriginalCVariableCapture,
        value: obj::Owned,
    ) -> Code {
        let root = capture.root.as_slice();
        let element = capture.element.as_deref();
        if let Err(error) = capture.receiver.store(value.as_ptr()) {
            return crate::builtins::var_error(self, root, error);
        }
        drop(value);
        let access = self.trace_access(root, root, element, &capture.home, false);
        if self.fire_var_trace_resolved(&capture.home, &access, b"write") {
            return crate::builtins::var_error(self, root, crate::frame::VarError::TraceError);
        }
        if self.host_refusal_pending() {
            return Code::Error;
        }
        match capture.receiver.read() {
            Ok(Some(value)) => self.set_result(value),
            _ => self.set_result_bytes(b""),
        }
        Code::Ok
    }

    fn body_append_list_element(
        &mut self,
        capture: super::native_variable_names::OriginalCVariableCapture,
        element: *mut TclObj,
        protocol: NativeStringProtocol,
    ) -> Code {
        let root = capture.root.as_slice();
        if capture.receiver.is_constant() {
            return crate::builtins::var_error(self, root, crate::frame::VarError::IsConstant);
        }
        let original = match capture.receiver.read_initial() {
            Ok(original) => original,
            Err(error) => return crate::builtins::var_error(self, root, error),
        };
        let value = match original {
            None => obj::Owned::fresh(obj::new_string_bytes(b"")),
            Some(value) if obj::is_shared(value) => {
                let duplicate = obj::duplicate(value);
                crate::list::duplicate_native_backing(value, duplicate, protocol);
                obj::Owned::fresh(duplicate)
            }
            Some(value) => obj::Owned::retain(value),
        };
        // TclPtrSetVar's single-element append installs the selected header
        // before conversion. A malformed receiver still keeps that duplicate.
        if let Err(error) = capture.receiver.store(value.as_ptr()) {
            return crate::builtins::var_error(self, root, error);
        }
        if let Err(error) =
            crate::list::append_prepared_native_elements(value.as_ptr(), &[element], protocol)
        {
            return self.report_cmd_error(error.into());
        }
        self.body_publish_append(capture, value)
    }

    fn body_append_list_values(
        &mut self,
        evaluated: &EvaluatedTarget,
        slot: Option<usize>,
        input: *mut TclObj,
        protocol: NativeStringProtocol,
    ) -> Code {
        // LAPPEND_LIST converts its evaluated input before variable lookup.
        let elements = match crate::list::list_elements_native_checked(input, protocol) {
            Ok(elements) => elements,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let capture = match self.body_capture_target(evaluated, slot, true) {
            Ok(capture) => capture,
            Err(code) => return code,
        };
        let root = capture.root.as_slice();
        let access =
            self.trace_access(root, root, capture.element.as_deref(), &capture.home, false);
        let read_failed = self.fire_var_trace_resolved(&capture.home, &access, b"read");
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let original = if read_failed {
            let reason = self
                .traces
                .borrow_mut()
                .pending_err
                .take()
                .unwrap_or_default();
            self.var_trace_error(root, b"read", &reason);
            None
        } else {
            match capture.receiver.read_initial() {
                Ok(value) => value,
                Err(error) => {
                    crate::builtins::var_error(self, root, error);
                    None
                }
            }
        };
        if original.is_none() && !read_failed {
            self.no_such_variable(root, capture.element.as_deref());
        }
        let Some(empty) =
            tcl_registry::native_instruction_plan::native_list_append_empty_publication(protocol)
        else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native List append instruction",
                )
                .into(),
            );
        };
        let value = match original {
            // Native missing-variable LAPPEND_LIST stores its prepared input.
            None if elements.is_empty() && empty.fresh_missing_receiver => {
                obj::Owned::fresh(obj::new_obj())
            }
            None => obj::Owned::retain(input),
            Some(value) => {
                if elements.is_empty() && empty.skip_existing_store {
                    if let Err(error) = crate::list::list_elements_native_checked(value, protocol) {
                        return self.report_cmd_error(error.into());
                    }
                    // The borrowed receiver is live in its original variable or
                    // array-default owner; the result setter retains it.
                    unsafe { self.set_obj_result(value) };
                    return Code::Ok;
                }
                match crate::list::append_native_list_elements(value, &elements, protocol) {
                    Ok(value) => value,
                    Err(error) => return self.report_cmd_error(error.into()),
                }
            }
        };
        self.body_publish_append(capture, value)
    }

    fn body_helper_operand(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        words: &[tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand],
        literal_slots: &[Option<usize>],
        index: usize,
        execution: &mut BodyExecution,
    ) -> Result<obj::Owned, Code> {
        match &words[index] {
            tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(
                original,
            ) => self.body_word(artifact, &command.words[*original], execution),
            _ => Ok(obj::Owned::retain(
                artifact
                    .literals
                    .original(literal_slots[index].expect("projected member slot"))
                    .expect("compiled original member"),
            )),
        }
    }

    fn execute_body_instruction(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        execution: &mut BodyExecution,
    ) -> Code {
        let result = (|| -> Result<Code, Code> {
            match &command.operation {
                Operation::StringMatch(matcher) => {
                    self.execute_body_string_match(artifact, command, matcher, execution)
                }
                Operation::Error(error) => {
                    self.execute_body_error(artifact, command, error, execution)
                }
                Operation::DictionaryLookup(dictionary) => {
                    self.execute_body_dictionary_lookup(artifact, command, dictionary, execution)
                }
                Operation::NamedInvocation(named) => {
                    self.execute_body_named(artifact, command, named, execution)
                }
                Operation::Uplevel(recipe, default) => {
                    let level = match recipe.level_word {
                        Some(index) => {
                            self.body_word(artifact, &command.words[index], execution)?
                        }
                        None => obj::Owned::retain(
                            artifact
                                .literals
                                .original(default.expect("implicit uplevel level"))
                                .expect("original level literal"),
                        ),
                    };
                    let mut fragments = command.words[recipe.script_words.clone()]
                        .iter()
                        .map(|word| self.body_word(artifact, word, execution))
                        .collect::<Result<Vec<_>, _>>()?;
                    let script = if fragments.len() == 1 {
                        fragments.pop().expect("original sole uplevel script")
                    } else {
                        let originals =
                            fragments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                        let value = tcl_cmd_core::list::concat_selected(self, &originals)
                            .map_err(|error| self.report_cmd_error(error))?;
                        obj::Owned::fresh(value)
                    };
                    let target =
                        crate::cmd_eval::select_compiled_uplevel_frame(self, level.as_ptr())?;
                    let location = if recipe.script_words.len() == 1 {
                        command.words[recipe.script_words.start]
                            .literal
                            .and_then(|_| {
                                let frames = self.cmd_frames.borrow();
                                let frame = frames.last()?;
                                let file = frame.file.clone()?;
                                Some((
                                    Some(file),
                                    frame.line_base
                                        + line_of(
                                            artifact.image.bytes(),
                                            command.words[recipe.script_words.start]
                                                .original
                                                .span()
                                                .start()
                                                as usize,
                                        ),
                                ))
                            })
                    } else {
                        None
                    };
                    if let Some((file, line)) = location.as_ref() {
                        self.arg_locs
                            .borrow_mut()
                            .push((script.as_ptr(), file.clone(), *line));
                    }
                    let code = self.eval_uplevel_obj(target, script.as_ptr());
                    if location.is_some() {
                        self.arg_locs.borrow_mut().pop();
                    }
                    if code == Code::Error {
                        self.append_body_frame(b"uplevel");
                    }
                    Ok(code)
                }
                Operation::Expression(expression) => self
                    .execute_body_expression_instruction(artifact, command, expression, execution),
                Operation::Try(operation) => {
                    self.execute_body_try(artifact, command, operation, execution)
                }
                Operation::Break => Ok(Code::Break),
                Operation::Continue => Ok(Code::Continue),
                Operation::Control(control) => {
                    self.execute_body_control(artifact, command, control, execution)
                }
                Operation::Each(each) => self.execute_body_each(artifact, command, each, execution),
                Operation::Switch(switch) => {
                    self.execute_body_switch(artifact, command, switch, execution)
                }
                Operation::TclOoHelper(recipe, literal_slots) => {
                    use tcl_registry::native_tcloo_compilation::NativeTclOoInstruction as Helper;
                    let code = match recipe {
                        Helper::SelfObject | Helper::SelfNamespace => {
                            self.execute_native_oo_self(matches!(recipe, Helper::SelfNamespace))
                        }
                        Helper::ObjectInfo { operation, operand } => {
                            let original = self.body_helper_operand(
                                artifact,
                                command,
                                std::slice::from_ref(operand),
                                literal_slots,
                                0,
                                execution,
                            )?;
                            self.execute_native_oo_object_info(*operation, original.as_ptr())
                        }
                        Helper::Next { class, words, list } => {
                            let mut stack = Vec::<obj::Owned>::new();
                            if let Some(steps) = list {
                                for step in steps {
                                    match *step {
                                        NativeArgumentListStep::Word(index) => {
                                            stack.push(self.body_helper_operand(
                                                artifact,
                                                command,
                                                words,
                                                literal_slots,
                                                index,
                                                execution,
                                            )?)
                                        }
                                        NativeArgumentListStep::List(count) => {
                                            let operands = stack.split_off(stack.len() - count);
                                            let members = operands
                                                .iter()
                                                .map(obj::Owned::as_ptr)
                                                .collect::<Vec<_>>();
                                            stack.push(obj::Owned::fresh(
                                                crate::list::new_list_obj_native(
                                                    &members,
                                                    artifact.stamp.source_protocol,
                                                ),
                                            ));
                                        }
                                        NativeArgumentListStep::Concat => {
                                            let source = stack.pop().expect("helper List source");
                                            let target = stack.pop().expect("helper List target");
                                            stack.push(
                                                crate::list::concatenate_native_lists(
                                                    target.as_ptr(),
                                                    source.as_ptr(),
                                                    artifact.stamp.source_protocol,
                                                )
                                                .map_err(|error| {
                                                    self.report_cmd_error(error.into())
                                                })?,
                                            );
                                        }
                                    }
                                }
                            } else {
                                for index in 0..words.len() {
                                    stack.push(self.body_helper_operand(
                                        artifact,
                                        command,
                                        words,
                                        literal_slots,
                                        index,
                                        execution,
                                    )?);
                                }
                            }
                            let arguments = if list.is_some() {
                                // Keep the genuine parent List alive through the reached call.
                                crate::list::list_elements_native_checked(
                                    stack[0].as_ptr(),
                                    artifact.stamp.source_protocol,
                                )
                                .map_err(|error| self.report_cmd_error(error.into()))?
                            } else {
                                stack.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                            };
                            if arguments.len() < if *class { 2 } else { 1 } {
                                return Err(self.report_cmd_error(
                                    unavailable(
                                        "native TclOO invocation List has insufficient words",
                                    )
                                    .into(),
                                ));
                            }
                            self.execute_native_oo_next(&arguments, *class, artifact.stamp.physical)
                        }
                    };
                    Ok(code)
                }

                Operation::Invoke
                | Operation::NamespaceBindings(NamespaceOperation {
                    generic: Some(_), ..
                }) => {
                    let mut values = Vec::new();
                    let mut original_words = Vec::new();
                    let written;
                    let operands = if let Operation::NamespaceBindings(NamespaceOperation {
                        generic: Some(operands),
                        ..
                    }) = &command.operation
                    {
                        operands.as_slice()
                    } else {
                        written = (0..command.words.len())
                            .map(NamespaceOperand::Original)
                            .collect::<Vec<_>>();
                        written.as_slice()
                    };
                    for operand in operands {
                        let word = match operand {
                            NamespaceOperand::Original(index) => Some(&command.words[*index]),
                            NamespaceOperand::Literal(_) => None,
                        };
                        let value =
                            self.body_namespace_operand(artifact, command, operand, execution)?;
                        if execution.done {
                            return Ok(Code::Ok);
                        }
                        if word.is_some_and(|word| word.original.group().expand) {
                            let members = crate::list::list_elements_native_checked(
                                value.as_ptr(),
                                artifact.stamp.source_protocol,
                            )
                            .map_err(|error| self.report_cmd_error(error.into()))?;
                            original_words.extend(std::iter::repeat_n(None, members.len()));
                            values.extend(members.into_iter().map(obj::Owned::retain));
                        } else {
                            original_words.push(word);
                            values.push(value);
                        }
                    }
                    if values.is_empty() {
                        self.set_result_bytes(b"");
                        return Ok(Code::Ok);
                    }
                    let argv: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
                    let file = self
                        .cmd_frames
                        .borrow()
                        .last()
                        .and_then(|frame| frame.file.clone());
                    let mut added = 0;
                    if file.is_some() {
                        for (word, value) in original_words.iter().zip(&values) {
                            if let Some(word) = word.filter(|word| word.literal.is_some()) {
                                let line = self
                                    .cmd_frames
                                    .borrow()
                                    .last()
                                    .map_or(0, |frame| frame.line_base)
                                    + line_of(
                                        artifact.image.bytes(),
                                        word.original.span().start() as usize,
                                    );
                                self.arg_locs.borrow_mut().push((
                                    value.as_ptr(),
                                    file.clone(),
                                    line,
                                ));
                                added += 1;
                            }
                        }
                    }
                    let code = self.dispatch(&argv);
                    if added != 0 {
                        let mut locations = self.arg_locs.borrow_mut();
                        let length = locations.len() - added;
                        locations.truncate(length);
                    }
                    Ok(code)
                }
                Operation::NamespaceBindings(scope) => {
                    use tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind;
                    let prefix = scope.prefix.map(|index| {
                        obj::Owned::retain(
                            artifact
                                .literals
                                .original(index)
                                .expect("pooled root namespace operand"),
                        )
                    });
                    let namespace_object = scope
                        .namespace
                        .as_ref()
                        .map(|operand| {
                            self.body_namespace_operand(artifact, command, operand, execution)
                        })
                        .transpose()?;
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                    for binding in &scope.bindings {
                        let name = self.body_namespace_operand(
                            artifact,
                            command,
                            &binding.name,
                            execution,
                        )?;
                        if execution.done {
                            return Ok(Code::Ok);
                        }
                        let namespace = if scope.kind == NativeNamespaceBindingKind::Global {
                            let prefix = prefix.as_ref().expect("original root namespace operand");
                            self.native_namespace_object_lookup(prefix.as_ptr())
                                .map_err(|error| self.report_cmd_error(error.into()))?
                                .ok_or_else(|| {
                                    self.report_cmd_error(
                                        unavailable("compiled root namespace operand lookup")
                                            .into(),
                                    )
                                })?
                        } else if scope.kind == NativeNamespaceBindingKind::Upvar {
                            let original = namespace_object
                                .as_ref()
                                .expect("original namespace upvar operand");
                            match self.native_namespace_object_lookup(original.as_ptr()) {
                                Ok(Some(namespace)) => namespace,
                                Ok(None) => return Ok(crate::cmd_namespace::ns_operation_not_found(
                                    self, original.as_ptr(), tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                                )),
                                Err(error) => return Ok(self.report_cmd_error(error.into())),
                            }
                        } else {
                            let frames = self.frames.borrow();
                            frames.frame_ns(frames.current_level())
                        };
                        let code = self.link_original_compiled_namespace_variable(
                            name.as_ptr(),
                            namespace,
                            binding.slot,
                            scope.kind == NativeNamespaceBindingKind::Variable,
                        );
                        if code != Code::Ok || self.host_refusal_pending() {
                            return Ok(code);
                        }
                        drop(name);
                        if let Some(operand) = &binding.value {
                            let value =
                                self.body_namespace_operand(artifact, command, operand, execution)?;
                            if execution.done {
                                return Ok(Code::Ok);
                            }
                            let code = self.body_store(
                                &EvaluatedTarget {
                                    root: binding.local.clone(),
                                    element: None,
                                    original_name: None,
                                    original_index: None,
                                    combined: false,
                                },
                                Some(binding.slot),
                                &value,
                            );
                            if code != Code::Ok || self.host_refusal_pending() {
                                return Ok(code);
                            }
                        }
                    }
                    drop(prefix);
                    self.set_result(
                        artifact
                            .literals
                            .original(scope.empty.expect("namespace empty result"))
                            .expect("pooled namespace empty result"),
                    );
                    Ok(Code::Ok)
                }
                Operation::Unset(unset) => {
                    self.execute_body_unset(artifact, command, unset, execution)
                }
                Operation::Load(target) => {
                    let evaluated = self.body_target(artifact, command, target, execution)?;
                    let value = if evaluated.combined {
                        obj::Owned::retain(
                            self.read_original_c_variable(
                                evaluated
                                    .original_name
                                    .as_ref()
                                    .expect("original dynamic name")
                                    .as_ptr(),
                            )?,
                        )
                    } else {
                        self.body_read_original_parts(&evaluated, target.slot)?
                    };
                    self.set_result(value.as_ptr());
                    Ok(Code::Ok)
                }
                Operation::Store(target, value) => {
                    let evaluated = self.body_target(artifact, command, target, execution)?;
                    let value = self.body_word(artifact, &command.words[*value], execution)?;
                    if evaluated.combined {
                        self.store_original_c_variable(
                            evaluated
                                .original_name
                                .as_ref()
                                .expect("original dynamic name")
                                .as_ptr(),
                            value.as_ptr(),
                        )?;
                        return Ok(Code::Ok);
                    }
                    Ok(self.body_store(&evaluated, target.slot, &value))
                }
                Operation::Increment(target, amount, immediate) => {
                    let evaluated = self.body_target(artifact, command, target, execution)?;
                    let value = if let Some(immediate) = immediate {
                        obj::Owned::fresh(obj::new_wide_int_obj(i64::from(*immediate)))
                    } else {
                        self.body_word(
                            artifact,
                            &command.words[amount.expect("explicit nonimmediate amount")],
                            execution,
                        )?
                    };
                    Ok(self.increment_original_compiled_target(
                        &evaluated.root,
                        evaluated.element.as_deref(),
                        target.slot,
                        (
                            evaluated.original_name.as_ref().map(obj::Owned::as_ptr),
                            evaluated.original_index.as_ref().map(obj::Owned::as_ptr),
                            evaluated.combined,
                        ),
                        value.as_ptr(),
                    ))
                }
                Operation::Append(target, recipe) => {
                    let evaluated = self.body_target(artifact, command, target, execution)?;
                    match &recipe.operands {
                        NativeAppendOperands::StringObjects => {
                            // Native multi-append evaluates every value before
                            // its first append, then publishes each in order.
                            let values = command.words[recipe.values.clone()]
                                .iter()
                                .map(|word| self.body_word(artifact, word, execution))
                                .collect::<Result<Vec<_>, _>>()?;
                            for value in values {
                                let capture =
                                    self.body_capture_target(&evaluated, target.slot, true)?;
                                let code = self.append_native_captured(capture, &[value.as_ptr()]);
                                if code != Code::Ok {
                                    return Ok(code);
                                }
                            }
                            Ok(Code::Ok)
                        }
                        NativeAppendOperands::ListElement => {
                            let value = self.body_word(
                                artifact,
                                &command.words[recipe.values.start],
                                execution,
                            )?;
                            let capture =
                                self.body_capture_target(&evaluated, target.slot, true)?;
                            Ok(self.body_append_list_element(
                                capture,
                                value.as_ptr(),
                                artifact.stamp.source_protocol,
                            ))
                        }
                        NativeAppendOperands::ListElements {
                            steps,
                            strip_single_expanded,
                        } => {
                            let value = self.body_argument_list(
                                artifact,
                                command,
                                steps,
                                *strip_single_expanded,
                                execution,
                            )?;
                            Ok(self.body_append_list_values(
                                &evaluated,
                                target.slot,
                                value.as_ptr(),
                                artifact.stamp.source_protocol,
                            ))
                        }
                    }
                }
                Operation::List(recipe, index) => {
                    let value = if let Some(index) = index {
                        obj::Owned::retain(
                            artifact
                                .literals
                                .original(*index)
                                .expect("actual compiler List literal"),
                        )
                    } else {
                        debug_assert!(matches!(recipe, NativeCompiledListRecipe::DynamicElements));
                        let values = command.words[1..]
                            .iter()
                            .map(|word| self.body_word(artifact, word, execution))
                            .collect::<Result<Vec<_>, _>>()?;
                        let members: Vec<_> = values.iter().map(obj::Owned::as_ptr).collect();
                        obj::Owned::fresh(crate::list::new_list_obj_native(
                            &members,
                            artifact.stamp.source_protocol,
                        ))
                    };
                    self.set_result(value.as_ptr());
                    Ok(Code::Ok)
                }
                Operation::Concat {
                    literal, operands, ..
                } => {
                    let value = if let Some(index) = literal {
                        obj::Owned::retain(
                            artifact
                                .literals
                                .original(*index)
                                .expect("original concat literal"),
                        )
                    } else {
                        let originals = operands
                            .iter()
                            .map(|operand| {
                                self.body_namespace_operand(artifact, command, operand, execution)
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let pointers = originals.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                        obj::Owned::fresh(
                            tcl_cmd_core::list::concat_selected(self, &pointers)
                                .map_err(|error| self.report_cmd_error(error))?,
                        )
                    };
                    self.set_result(value.as_ptr());
                    Ok(Code::Ok)
                }
                Operation::SelectedReturn(recipe, empty, options) => {
                    use tcl_registry::native_return_compilation::{
                        NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
                    };
                    let stack_options = match &recipe.options {
                        Options::StackWord(index) => {
                            Some(self.body_word(artifact, &command.words[*index], execution)?)
                        }
                        Options::StackPairs(range) => {
                            let originals = command.words[range.clone()]
                                .iter()
                                .map(|word| self.body_word(artifact, word, execution))
                                .collect::<Result<Vec<_>, _>>()?;
                            let members =
                                originals.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                            Some(obj::Owned::fresh(crate::list::new_list_obj_native(
                                &members,
                                artifact.stamp.source_protocol,
                            )))
                        }
                        Options::Static(_) => None,
                    };
                    let value = match recipe.value_word {
                        Some(index) => {
                            self.body_word(artifact, &command.words[index], execution)?
                        }
                        None => obj::Owned::retain(
                            artifact
                                .literals
                                .original(empty.expect("implicit return result"))
                                .expect("empty literal"),
                        ),
                    };
                    if recipe.exit == Exit::Done {
                        self.set_result(value.as_ptr());
                        execution.done = true;
                        Ok(Code::Return)
                    } else if recipe.exit == Exit::Fallthrough {
                        self.set_result(value.as_ptr());
                        Ok(Code::Ok)
                    } else {
                        let merged;
                        let (code, level, original) = if recipe.exit == Exit::Stack {
                            let options = stack_options.as_ref().expect("stack original options");
                            let (mut ops, protocol) =
                                crate::return_options::NativeReturnOps::selected(self)
                                    .map_err(|error| self.report_cmd_error(error))?;
                            merged = tcl_cmd_core::native_return_merge::merge_stack(
                                &mut ops, protocol, options,
                            )
                            .map_err(|error| self.report_cmd_error(error))?;
                            (
                                merged.code,
                                i64::from(merged.level),
                                merged.options.as_ptr(),
                            )
                        } else {
                            let Options::Static(literal) = &recipe.options else {
                                unreachable!()
                            };
                            (
                                literal.code,
                                i64::from(literal.level),
                                artifact
                                    .literals
                                    .original(options.expect("private Return slot"))
                                    .expect("private Return original"),
                            )
                        };
                        let code=self.process_original_c_return_options(tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,code,level,original).map_err(|error|self.report_cmd_error(error.into()))?;
                        self.set_result(value.as_ptr());
                        if code == Code::Error && recipe.exit == Exit::Immediate {
                            self.capture_original_return_instruction_context(
                                tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                                value.as_ptr(),
                                original,
                            );
                        }
                        Ok(code)
                    }
                }
            }
        })();
        result.unwrap_or_else(|code| code)
    }
}

impl NativeBodyArtifact {
    pub(super) fn compiled_local_layout(&self) -> Option<&NativeCompiledLocalLayout> {
        self.locals.as_ref()
    }
}

#[cfg(test)]
mod control_expression_tests;
#[cfg(test)]
#[path = "native_body_artifact/namespace_tests.rs"]
mod namespace_tests;
#[cfg(test)]
mod object_info_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_c91_uplevel_artifact_selects_frame_then_enters_original_script() {
        for (source, expected) in [
            (b"set x BEFORE; proc p {} {set x INNER;uplevel 1 {set x OUTER};return $x};list [p] $x".as_slice(), b"INNER OUTER".as_slice()),
            (b"set x BEFORE; proc p {} {set x INNER;uplevel set x OUTER;return $x};list [p] $x".as_slice(), b"INNER OUTER".as_slice()),
            (b"set x BEFORE; proc p {} {uplevel 1 set x OUTER};list [p] $x".as_slice(), b"OUTER OUTER".as_slice()),
            (b"set x BEFORE;proc p {} {catch {uplevel #99 {}} message;set x INNER;return [list $message $x]};list [p] $x".as_slice(), b"{{bad level \"#99\"} INNER} BEFORE".as_slice()),
        ] {
            let mut interp = interpreter("tcl9.1");
            assert_eq!(interp.eval_str(source), Code::Ok, "{}", String::from_utf8_lossy(source));
            assert_eq!(interp.result_bytes(), expected);
            assert!(!interp.host_refusal_pending());
            let procedure = interp.proc_def(b"p").expect("original procedure");
            let artifact = cache(procedure.body.as_ptr()).expect("actual original bytecode");
            assert!(artifact.scripts.values().any(|script| script.commands.iter().any(|command| matches!(command.operation, Operation::Uplevel(..)))));
        }
    }

    thread_local! {
        static GETTER_ORDER: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn update_index(value: *mut TclObj) {
        GETTER_ORDER.with(|events| events.borrow_mut().push("getter"));
        // SAFETY: this exact updater receives the same live original operand.
        unsafe { obj::set_native_updater_string_rep(value, b"k", false) };
    }
    static INDEX_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"artifactIndex".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update_index),
        set_from_any_proc: None,
    };
    fn later_operand(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        GETTER_ORDER.with(|events| events.borrow_mut().push("value"));
        interp.set_result_bytes(b"2");
        Code::Ok
    }

    pub(super) fn interpreter(profile: &str) -> Interp {
        let interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .expect("authenticated original C constructor");
        assert!(
            interp.native_compiler_cache_epochs(GLOBAL).is_some(),
            "{profile}: actual compiler epochs"
        );
        assert!(
            interp.source_string_protocol().is_some(),
            "{profile}: selected original source protocol"
        );
        interp
    }

    #[test]
    fn original_procedure_artifact_executes_indexed_slots_and_reuses_original_cache() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interpreter(profile);
            assert_eq!(
                interp
                    .eval_str(b"proc p {value} {set x $value; incr x; set a(k) $x; return $a(k)}"),
                Code::Ok,
                "{profile}"
            );
            let procedure = interp.proc_def(b"p").expect("original declaration");
            assert!(cache(procedure.body.as_ptr()).is_none());
            assert_eq!(interp.eval_str(b"p 3"), Code::Ok, "{profile}");
            assert_eq!(interp.result_bytes(), b"4", "{profile}");
            let first = cache(procedure.body.as_ptr()).expect("actual emitted original artifact");
            let layout = first
                .compiled_local_layout()
                .expect("actual emitted local slots");
            assert_eq!(
                layout
                    .names
                    .iter()
                    .map(|name| name.as_ref().map(|name| name.as_bytes()))
                    .collect::<Vec<_>>(),
                vec![Some(&b"value"[..]), Some(&b"x"[..]), Some(&b"a"[..])],
                "{profile}"
            );
            assert_eq!(interp.eval_str(b"p 7"), Code::Ok, "{profile}");
            assert_eq!(interp.result_bytes(), b"8", "{profile}");
            assert!(
                Rc::ptr_eq(
                    &first,
                    &cache(procedure.body.as_ptr()).expect("reused original cache")
                ),
                "{profile}"
            );
            let duplicate = obj::Owned::fresh(obj::duplicate(procedure.body.as_ptr()));
            assert!(cache(duplicate.as_ptr()).is_none(), "{profile}");
            assert_eq!(
                obj_bytes(duplicate.as_ptr()),
                obj_bytes(procedure.body.as_ptr())
            );
        }
    }

    #[test]
    fn original_append_artifacts_keep_registered_operands_and_native_local_layout() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for (body, expected) in [
                (
                    b"set r $input; append r A; return $r".as_slice(),
                    b"XA".as_slice(),
                ),
                (
                    b"set r $input; append r A B; return $r".as_slice(),
                    b"XAB".as_slice(),
                ),
                (
                    b"set r $input; lappend r A; return $r".as_slice(),
                    b"X A".as_slice(),
                ),
                (
                    b"set r $input; lappend r A B; return $r".as_slice(),
                    b"X A B".as_slice(),
                ),
                (
                    b"set a(k) $input; lappend a(k) A; return $a(k)".as_slice(),
                    b"X A".as_slice(),
                ),
            ] {
                let mut interp = interpreter(profile);
                let original = obj::Owned::fresh(new_string(body));
                interp.define_proc(
                    b"p",
                    vec![Param {
                        name: b"input".to_vec(),
                        default: None,
                    }],
                    original.as_ptr(),
                );
                assert_eq!(
                    interp.eval_str(b"p X"),
                    Code::Ok,
                    "{profile}/{body:?}: {:?}",
                    interp.result_bytes()
                );
                assert_eq!(interp.result_bytes(), expected, "{profile}/{body:?}");
                let artifact = cache(original.as_ptr()).expect("retained original append artifact");
                let script = artifact.scripts.get(&artifact.region).unwrap();
                assert!(
                    script.commands.iter().any(|command| matches!(
                        command.operation,
                        Operation::Append(..)
                    ) || matches!(
                        command.operation,
                        Operation::Invoke
                    )),
                    "{profile}: executable selected recipe"
                );
                let layout = artifact.compiled_local_layout().unwrap();
                assert!(
                    layout.names.iter().any(|name| name
                        .as_ref()
                        .is_some_and(|name| name.as_bytes() == b"r" || name.as_bytes() == b"a")),
                    "{profile}: retained indexed receiver"
                );
                assert_eq!(interp.eval_str(b"p Y"), Code::Ok, "{profile}/{body:?}");
                assert!(Rc::ptr_eq(&artifact, &cache(original.as_ptr()).unwrap()));
            }
        }
    }

    #[test]
    fn registered_append_artifacts_match_43_native_callback_controls() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut count = 0;
        for row in include_str!("../../tests/data/native_registered_append.tsv").lines() {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 7);
            let profile = fields[0];
            let mut interp = interpreter(profile);
            assert_eq!(interp.eval_str(b"proc watch {n1 n2 op} {lappend ::events $op}; proc arm {} {uplevel 1 {trace add variable r {read write} watch}}; set events {}"), Code::Ok, "{profile}/{}: observer setup", fields[1]);
            let original = obj::Owned::fresh(new_string(&decode(fields[2])));
            interp.define_proc(
                b"p",
                vec![Param {
                    name: b"input".to_vec(),
                    default: None,
                }],
                original.as_ptr(),
            );
            let head = obj::Owned::fresh(new_string(b"p"));
            let input = obj::Owned::fresh(new_string(&decode(fields[3])));
            let code = interp.dispatch(&[head.as_ptr(), input.as_ptr()]);
            assert_eq!(
                code.as_int(),
                fields[4].parse::<i64>().unwrap(),
                "{profile}/{}: {:?}",
                fields[1],
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                decode(fields[5]),
                "{profile}/{}: result",
                fields[1]
            );
            let events = interp
                .var_get(b"events")
                .expect("actual callback event variable");
            assert_eq!(
                obj_bytes(events),
                decode(fields[6]),
                "{profile}/{}: observer order",
                fields[1]
            );
            assert!(
                cache(original.as_ptr()).is_some(),
                "{profile}/{}: original executable body",
                fields[1]
            );
            count += 1;
        }
        assert_eq!(count, 43);
    }

    #[test]
    fn original_artifact_keeps_opcode_selection_before_argument_mutation() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interpreter(profile);
            assert_eq!(interp.eval_str(b"proc change {} {rename set savedSet; proc set args {return CUSTOM}; return VALUE}; proc p {} {set x [change]; return $x}"), Code::Ok);
            assert_eq!(interp.eval_str(b"p"), Code::Ok, "{profile}");
            assert_eq!(interp.result_bytes(), b"VALUE", "{profile}");
            assert!(
                cache(interp.proc_def(b"p").expect("p").body.as_ptr()).is_some(),
                "{profile}"
            );
        }
    }

    #[test]
    fn original_index_getter_follows_later_store_and_increment_operands() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for body in [
                b"set a($index) [later]; return $a(k)".as_slice(),
                b"set a(k) 1; incr a($index) [later]; return $a(k)".as_slice(),
                b"set $index [later]; return $k".as_slice(),
            ] {
                let mut interp = interpreter(profile);
                interp.register_builtin(b"later", later_operand);
                let original_body = obj::Owned::fresh(new_string(body));
                interp.define_proc(
                    b"p",
                    vec![Param {
                        name: b"index".to_vec(),
                        default: None,
                    }],
                    original_body.as_ptr(),
                );
                let index = obj::Owned::fresh(obj::new_obj());
                obj::change_type(index.as_ptr(), &INDEX_TYPE, 0);
                obj::invalidate_string(index.as_ptr());
                let head = obj::Owned::fresh(new_string(b"p"));
                GETTER_ORDER.with(|events| events.borrow_mut().clear());
                assert_eq!(
                    interp.dispatch(&[head.as_ptr(), index.as_ptr()]),
                    Code::Ok,
                    "{profile}/{body:?}"
                );
                GETTER_ORDER.with(|events| {
                    assert_eq!(
                        events.borrow().as_slice(),
                        ["value", "getter"],
                        "{profile}/{body:?}"
                    )
                });
                assert!(
                    cache(interp.proc_def(b"p").unwrap().body.as_ptr()).is_some(),
                    "{profile}: actual executable artifact"
                );
            }
        }
    }

    #[test]
    fn procedure_done_in_bracket_exits_without_return_option_state() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interpreter(profile);
            assert_eq!(
                interp.eval_str(b"proc p {} {set x [return EARLY]; set ::late YES}; p"),
                Code::Ok,
                "{profile}"
            );
            assert_eq!(interp.result_bytes(), b"EARLY", "{profile}");
            assert!(
                interp.var_get(b"::late").is_none(),
                "{profile}: later store is not entered"
            );
            assert!(cache(interp.proc_def(b"p").unwrap().body.as_ptr()).is_some());
            assert!(
                interp.pending_return_option_objects().is_empty(),
                "{profile}: DONE has no merged options operand"
            );
        }
    }

    #[test]
    fn original_artifact_empty_source_and_parse_failure_keep_native_publication_policy() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interpreter(profile);
            for (name, body) in [
                (b"empty".as_slice(), b"".as_slice()),
                (b"bad", b"{"),
                (b"early", b"return EARLY; {"),
            ] {
                let original = obj::Owned::fresh(new_string(body));
                interp.define_proc(name, Vec::new(), original.as_ptr());
            }
            assert_eq!(interp.eval_str(b"empty"), Code::Ok, "{profile}");
            assert!(
                cache(interp.proc_def(b"empty").expect("empty").body.as_ptr()).is_some(),
                "{profile}"
            );
            assert_eq!(interp.eval_str(b"bad"), Code::Error, "{profile}");
            let modern = profile != "tcl8.4";
            assert_eq!(
                cache(interp.proc_def(b"bad").expect("bad").body.as_ptr()).is_some(),
                modern,
                "{profile}"
            );
            assert_eq!(
                interp.eval_str(b"early"),
                if modern { Code::Ok } else { Code::Error },
                "{profile}"
            );
            if modern {
                assert_eq!(interp.result_bytes(), b"EARLY");
            }
        }
    }

    #[test]
    fn syntax_instruction_reuses_same_stringless_options_dictionary() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interpreter(profile);
            let original = obj::Owned::fresh(new_string(b"{"));
            interp.define_proc(b"bad", Vec::new(), original.as_ptr());
            assert_eq!(interp.eval_str(b"bad"), Code::Error, "{profile}");
            let body = interp.proc_def(b"bad").unwrap().body.as_ptr();
            let first = cache(body).expect("authentic cached SYNTAX program");
            let fatal = first.scripts[&first.region].fatal.as_ref().unwrap();
            let options = first.literals.original(fatal.options).unwrap();
            assert!(core::ptr::eq(
                obj::obj_type_ptr(options),
                &crate::dict::TCL_DICT_TYPE
            ));
            assert!(
                !obj::has_string_rep(options),
                "{profile}: TclAddLiteralObj does not GetString"
            );
            assert_eq!(interp.eval_str(b"bad"), Code::Error, "{profile}");
            assert!(Rc::ptr_eq(&first, &cache(body).unwrap()), "{profile}");
            assert_eq!(first.literals.original(fatal.options), Some(options));
            assert!(core::ptr::eq(
                obj::obj_type_ptr(options),
                &crate::dict::TCL_DICT_TYPE
            ));
            assert!(
                !obj::has_string_rep(options),
                "{profile}: SYNTAX does not reach RETURN_STK List conversion"
            );
        }
    }

    #[test]
    fn original_artifact_legacy_recompilation_uses_actual_roles_and_fresh_body() {
        use tcl_runtime_api::native_procedure_roles::NativeProcedureReference;
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = interpreter(profile);
            assert_eq!(
                interp.eval_str(b"proc p {{value DEFAULT}} {return $value}; p"),
                Code::Ok
            );
            let first = interp.proc_def(b"p").unwrap();
            let body = first.body.checked_ptr().unwrap();
            let default = first.params[0]
                .default
                .as_ref()
                .unwrap()
                .checked_ptr()
                .unwrap();
            assert!(cache(body).is_some(), "{profile}: genuine warm artifact");
            let role = NativeProcedureReference::acquire(&first)
                .expect("actual retained native procedure role");
            assert_eq!(
                interp.eval_str(b"rename set savedSet; p"),
                Code::Ok,
                "{profile}"
            );
            assert_eq!(interp.result_bytes(), b"DEFAULT");
            let next = interp.proc_def(b"p").unwrap();
            let legacy = matches!(profile, "tcl8.4" | "tcl8.5");
            assert_eq!(
                !Rc::ptr_eq(&first, &next),
                legacy,
                "{profile}: actual client data"
            );
            assert_eq!(
                next.body.as_ptr() != body,
                legacy,
                "{profile}: actual chosen source"
            );
            assert_eq!(
                next.params[0].default.as_ref().unwrap().as_ptr(),
                default,
                "{profile}: same default"
            );
            assert!(
                cache(next.body.as_ptr()).is_some(),
                "{profile}: newly admitted original body"
            );
            drop(role);
        }
    }

    #[test]
    fn original_artifact_recompiles_for_physical_compiler_mutation() {
        let mut interp = interpreter("tcl9.1");
        assert_eq!(
            interp.eval_str(b"proc p {} {set x ONE; return $x}; p"),
            Code::Ok
        );
        let procedure = interp.proc_def(b"p").expect("p");
        let first = cache(procedure.body.as_ptr()).expect("first artifact");
        assert_eq!(
            interp.eval_str(b"proc unrelated {} {return OTHER}; p"),
            Code::Ok
        );
        assert!(Rc::ptr_eq(
            &first,
            &cache(procedure.body.as_ptr()).expect("unchanged compile epoch")
        ));
        assert_eq!(
            interp.eval_str(b"rename set savedSet; proc set args {return CUSTOM}; p"),
            Code::Error
        );
        let second = cache(procedure.body.as_ptr()).expect("new generic artifact");
        assert!(!Rc::ptr_eq(&first, &second));
    }
    #[test]
    fn registered_return_artifacts_match_44_native_controls() {
        fn bytes(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for line in
            include_str!("../../../../rust/tcl-registry/tests/data/registered-return44.tsv").lines()
        {
            let fields = line.split('\t').collect::<Vec<_>>();
            let mut interp = interpreter(match fields[0] {
                "8.5.19" => "tcl8.5",
                "8.6.18" => "tcl8.6",
                "9.0.4" => "tcl9.0",
                "9.1.0" => "tcl9.1",
                _ => panic!("native version"),
            });
            let mut source = b"proc p {opts msg code} {".to_vec();
            source.extend(bytes(fields[2]));
            source.extend(b"}; p {-custom DYNAMIC} MSG error");
            let code = interp.eval_str(&source);
            assert_eq!(
                code,
                Code::from_int(fields[3].parse().unwrap()),
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                interp.result_bytes()
            );
            assert_eq!(
                interp.result_bytes(),
                bytes(fields[4]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 44);
    }
}
