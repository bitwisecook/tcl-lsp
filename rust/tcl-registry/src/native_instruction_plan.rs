// SPDX-License-Identifier: AGPL-3.0-or-later
//! Portable original-word instruction recipes for admitted C compiler operations.
//!
//! The concrete producer must independently authenticate the selected command
//! token, compiler attachment and implementation prerequisites. This projection
//! does not grant that authority or create a local-variable table.

use crate::InvocationDialect;
use crate::native_compilation::{
    NativeAppendKind, NativeCompilationContext, NativeCompilationFrame, NativeCompilationGrammar,
    NativeCompilationSelection, NativeCompilationSpec,
};
use crate::native_compiler_words::{
    NativeCompiledListRecipe, NativeCompiledListUnavailable, NativeCompilerWords,
};
use tcl_syntax::native_variable_words::{
    NativeVariableWordOperand, NativeVariableWordUnavailable, native_variable_word,
};

/// Executable operation selected from one original complete word vector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInstructionPlan {
    /// Original mathematical compiler stack, operand visits and anonymous cell.
    MathOperator(crate::native_mathop_compilation::NativeMathopInstruction),
    /// Original scalar operands followed by the selected native getter.
    Scalar(crate::native_scalar_compilation::NativeScalarInstruction),
    /// Original namespace/frame operands and actual selected native operation.
    Introspection(crate::native_introspection_compilation::NativeIntrospectionInstruction),
    /// Original counted namespace string operand and selected compiler program.
    NamespaceString(crate::native_namespace_string_compilation::NativeNamespaceStringInstruction),
    /// Original Array target, RHS and private foreach compiler geometry.
    Array(crate::native_array_compilation::NativeArrayCompilation),
    /// Original List/index operands and the native immediate-index protocol.
    ListIndex(crate::native_list_index_compilation::NativeListIndexInstruction),
    /// Original native ranges or interleaved list assignment stores.
    ListOperations(crate::native_list_operations_compilation::NativeListOperationInstruction),
    /// One retained frame-level operand followed by original ordered alias targets.
    Upvar(crate::native_upvar_compilation::NativeUpvarInstruction),
    /// Original variable geometry followed by a quiet existence observation.
    InfoExists(crate::native_info_exists_compilation::NativeInfoExistsInstruction),
    /// Native unset validation followed by sequential original receiver operations.
    Unset(crate::native_unset_compilation::NativeUnsetInstruction),
    /// Original Error operands and dialect-specific options construction.
    Error(crate::native_error_compilation::NativeErrorInstruction),
    /// Original coroutine stack and namespace-capture operations.
    Coroutine(crate::native_coroutine_compilation::NativeCoroutineInstruction),
    /// Original native dictionary/key/default stack operands.
    DictionaryLookup(crate::native_dictionary_compilation::NativeDictionaryLookupInstruction),
    /// Original dictionary mutation operands and physical receiver.
    DictionaryMutation(crate::native_dictionary_compilation::NativeDictionaryMutationInstruction),
    /// Original dictionary scope slots, auxiliaries and protected writeback.
    DictionaryScope(
        crate::native_control_compilation::NativeControlCompilation<
            crate::native_dictionary_scope_compilation::NativeDictionaryScopeInstruction,
        >,
    ),
    /// Original compiler visits retained when ordinary dispatch is selected.
    GenericPreparation(Vec<crate::native_control_compilation::NativeControlPreparationStep>),
    /// A compile-selected private name followed by independently guarded late lookup.
    NamedInvocation(NativeNamedInvocationInstruction),
    /// Original static tree or authentic substituted `EXPR_STK` operand recipe.
    Expression(crate::native_expression_program::NativeExpressionInstruction),
    /// The independently registered lexical break instruction.
    Break,
    /// The independently registered lexical continue instruction.
    Continue,
    /// Ordered conditional/loop/catch compiler preparation and real ranges.
    Control(
        crate::native_control_compilation::NativeControlCompilation<
            crate::native_control_instructions::NativeControlInstruction,
        >,
    ),
    /// Original try handler binding, matcher and protected cleanup geometry.
    Try(
        crate::native_control_compilation::NativeControlCompilation<
            crate::native_try_compilation::NativeTryInstruction,
        >,
    ),
    /// Original foreach/lmap auxiliary groups and physical temporary reservations.
    Each(
        crate::native_control_compilation::NativeControlCompilation<
            crate::native_each_compilation::NativeEachInstruction,
        >,
    ),
    /// Original literal-arm compiler, including skipped duplicate bodies.
    Switch(crate::native_switch_compilation::NativeSwitchInstruction),
    /// Push the original registered empty string, dynamic list or private List.
    List(NativeCompiledListRecipe),
    /// Original compile-known concatenation or ordered runtime operands.
    Concat(NativeConcatInstruction),
    /// Ordered root/current namespace bindings, including declined preparation.
    NamespaceBindings(
        crate::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
    ),
    /// Actual original helper stack/frame instruction geometry.
    TclOoHelper(crate::native_tcloo_compilation::NativeTclOoInstruction),
    /// Original pattern and subject followed by the selected C string matcher.
    StringMatch(crate::native_string_compilation::NativeStringMatchInstruction),
    /// Original trim subject and optional original character set.
    StringTrim(crate::native_string_trim_compilation::NativeStringTrimInstruction),
    /// Read the selected original variable operand.
    Load {
        /// Exact index of the target in the complete original word vector.
        target_word: usize,
        /// Original compiler variable-word geometry.
        target: NativeVariableWordOperand,
    },
    /// Store the value of an original word in the selected variable operand.
    Store {
        /// Exact index of the target in the complete original word vector.
        target_word: usize,
        /// Original compiler variable-word geometry.
        target: NativeVariableWordOperand,
        /// Index in the complete original vector, including its command head.
        value_word: usize,
    },
    /// Increment the original selected variable operand.
    Increment {
        /// Exact index of the target in the complete original word vector.
        target_word: usize,
        /// Original compiler variable-word geometry.
        target: NativeVariableWordOperand,
        /// Index of an explicit original amount, absent for native default one.
        amount_word: Option<usize>,
        /// Native compile-time immediate; absence requires evaluated conversion.
        immediate: Option<i32>,
    },
    /// Append evaluated original operands to a selected physical variable.
    Append {
        /// Exact index of the target in the complete original word vector.
        target_word: usize,
        /// Original receiver geometry, before any operand evaluation.
        target: NativeVariableWordOperand,
        /// Selected compiler's value construction and publication recipe.
        recipe: NativeAppendInstruction,
    },
    /// Options-bearing original native return, including default private headers.
    ReturnOptions(crate::native_return_compilation::NativeReturnInstruction),
    /// C9.1's original level operand and concatenated script stack geometry.
    Uplevel(NativeUplevelInstruction),
}

/// The compiler evaluates these original words without compiling the script body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeUplevelInstruction {
    /// Original explicit level word, or the native compiler's literal `1`.
    pub level_word: Option<usize>,
    /// Original script fragments, concatenated by the native stack instruction.
    pub script_words: std::ops::Range<usize>,
}

/// Concatenation prepares only original operands, never a generated script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeConcatInstruction {
    /// All original compiler operands have known bytes, including no operands.
    Literal {
        /// Exact output of the selected byte concatenation law.
        bytes: Vec<u8>,
        /// Actual native compiler construction route.
        allocation: NativeConcatLiteralAllocation,
    },
    /// Evaluate each original parser operand before the native stack operation.
    Operands(Vec<crate::native_compiler_word_projection::NativeCompilerWordOperand>),
}

/// Constant concatenation preserves the compiler's original object route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeConcatLiteralAllocation {
    /// Native registration from output bytes, including C9.1's no-argument form.
    Registered,
    /// C9.1 retains a fresh empty NULL-primary header without global registration.
    PrivateEmpty,
    /// C9.1 retains the original unknown-count String primary without registration.
    PrivateString,
}

impl NativeInstructionPlan {
    /// Borrow a reached original checked expression from its shared recipe.
    /// No parsing, command lookup or runtime evaluation is performed.
    #[must_use]
    pub fn expression_program(
        &self,
        operand: &crate::native_compiler_word_projection::NativeCompilerWordOperand,
    ) -> Option<&crate::native_expression_program::NativeExpressionProgram> {
        use crate::native_control_compilation::NativeControlOutcome;
        match self {
            Self::Expression(expression) => expression
                .program
                .as_ref()
                .filter(|program| &program.operand == operand),
            Self::Control(control) => match &control.outcome {
                NativeControlOutcome::Inline(control) => control.expression_program(operand),
                _ => None,
            },
            _ => None,
        }
    }

    /// Selected native compiler annotation for this exact original script role.
    /// Unrelated or declined operands supply no body-role annotation.
    #[must_use]
    pub fn body_error_context(
        &self,
        operand: &crate::native_compiler_word_projection::NativeCompilerWordOperand,
    ) -> Option<crate::native_compilation::NativeCompiledBodyErrorContext> {
        use crate::native_compilation::NativeCompiledBodyErrorContext as Context;
        use crate::native_control_compilation::NativeControlOutcome;
        match self {
            Self::Control(control) => match &control.outcome {
                NativeControlOutcome::Inline(control) => control.body_error_context(operand),
                _ => None,
            },
            Self::Each(each) => match &each.outcome {
                NativeControlOutcome::Inline(each) if &each.body == operand => Some(
                    if each.collection
                        == crate::native_each_compilation::NativeEachCollection::Foreach
                    {
                        Context::ForeachBody
                    } else {
                        Context::None
                    },
                ),
                _ => None,
            },
            Self::Try(recipe) => match &recipe.outcome {
                NativeControlOutcome::Inline(recipe)
                    if &recipe.body.operand == operand
                        || recipe
                            .finally
                            .as_ref()
                            .is_some_and(|body| &body.operand == operand)
                        || recipe.handlers.iter().any(|handler| {
                            handler
                                .body
                                .as_ref()
                                .is_some_and(|body| &body.operand == operand)
                        }) =>
                {
                    Some(Context::None)
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Selected native test annotation for this original expression role.
    #[must_use]
    pub fn expression_error_context(
        &self,
        operand: &crate::native_compiler_word_projection::NativeCompilerWordOperand,
    ) -> Option<crate::native_compilation::NativeCompiledExpressionErrorContext> {
        use crate::native_compilation::NativeCompiledExpressionErrorContext as Context;
        use crate::native_control_compilation::NativeControlOutcome;
        self.expression_program(operand)?;
        match self {
            Self::Expression(_) => Some(Context::None),
            Self::Control(control) => match &control.outcome {
                NativeControlOutcome::Inline(control) => control.expression_error_context(operand),
                _ => None,
            },
            _ => None,
        }
    }
}

/// One original operand or canonical compiler-selected replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNamedInvocationWord {
    /// Evaluate the retained parser operand, including a literal expansion member.
    Original(crate::native_compiler_word_projection::NativeCompilerWordOperand),
    /// Register the selected canonical ensemble member without reparsing it.
    Replacement(Vec<u8>),
}

/// Pure stack/usage layout; its caller must retain the actual compiler selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeNamedInvocationInstruction {
    /// Actual original compiler visits preceding the selected named fallback.
    pub preparations: Vec<crate::native_control_compilation::NativeControlPreparationStep>,
    /// Name fixed at compilation, not an implementation selected after argv.
    pub name: Vec<u8>,
    /// Native direct invocation or original ensemble usage rewrite.
    pub protocol: crate::native_compilation::NativeNamedInvocationProtocol,
    /// Number of original post-head operands consumed by selection.
    pub arguments_from: usize,
    /// Ordered operands; Direct excludes the selected private head itself.
    pub words: Vec<NativeNamedInvocationWord>,
    /// Expansion flags aligned with `words`; literal parser expansions are false.
    pub expanded: Vec<bool>,
}

/// Original selection of an installed private named-invocation compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeNamedWorkerCompilation {
    /// The selected native compiler declines to the original public invocation.
    Generic,
    /// The selected worker's own compiler captures a direct named invocation.
    Named(NativeNamedInvocationInstruction),
    /// Its actual grammar or original operand geometry is unavailable.
    Unavailable,
}

/// Select an actual private named compiler from retained original operands.
/// `operand_from` addresses the complete original vector after public member
/// selection. No synthetic command head or reconstructed source is introduced.
#[must_use]
pub fn native_named_worker_instruction(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    name: &[u8],
) -> NativeNamedWorkerCompilation {
    use crate::native_compilation::NativeCompilationGrammar;
    let NativeCompilationGrammar::NamedEnsembleInvocation {
        hook_from, arity, ..
    } = spec.grammar
    else {
        return NativeNamedWorkerCompilation::Unavailable;
    };
    let Some(version) = dialect
        .tcl_version
        .filter(|_| dialect.family() == Some(tcl_dialect::model::Family::Tcl))
    else {
        return NativeNamedWorkerCompilation::Unavailable;
    };
    let Ok(projected) =
        crate::native_compiler_word_projection::project_native_compiler_words(words, version)
    else {
        return NativeNamedWorkerCompilation::Unavailable;
    };
    let Some(operands) = projected.get(operand_from..) else {
        return NativeNamedWorkerCompilation::Unavailable;
    };
    if version < hook_from
        || operands.iter().any(|word| {
            word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded
        })
        || !u16::try_from(operands.len()).is_ok_and(|count| arity.accepts(count))
    {
        return NativeNamedWorkerCompilation::Generic;
    }
    let Some(arguments_from) = operand_from.checked_sub(1) else {
        return NativeNamedWorkerCompilation::Unavailable;
    };
    native_named_invocation_instruction(
        words,
        dialect,
        name,
        arguments_from,
        crate::native_compilation::NativeNamedInvocationProtocol::Direct,
        &[],
    )
    .map_or(
        NativeNamedWorkerCompilation::Unavailable,
        NativeNamedWorkerCompilation::Named,
    )
}

/// Project an independently captured named invocation, including mutable
/// ensemble names with no static catalogue lookup. This establishes no identity.
///
/// # Errors
/// Declines unavailable parser geometry or a conflicting replacement layout.
pub fn native_named_invocation_instruction(
    words: &NativeCompilerWords<'_>,
    dialect: InvocationDialect,
    name: &[u8],
    arguments_from: usize,
    protocol: crate::native_compilation::NativeNamedInvocationProtocol,
    replacements: &[Vec<u8>],
) -> Result<NativeNamedInvocationInstruction, NativeInstructionPlanUnavailable> {
    use crate::native_compilation::NativeNamedInvocationProtocol as Protocol;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
        return Err(NativeInstructionPlanUnavailable::CompilerPoint);
    }
    let version = dialect
        .tcl_version
        .ok_or(NativeInstructionPlanUnavailable::CompilerPoint)?;
    let projected =
        crate::native_compiler_word_projection::project_native_compiler_words(words, version)
            .map_err(|_| NativeInstructionPlanUnavailable::OperandGeometry)?;
    if arguments_from >= projected.len()
        || (protocol == Protocol::EnsembleRewrite && replacements.len() != arguments_from)
    {
        return Err(NativeInstructionPlanUnavailable::OperandGeometry);
    }
    let from = if protocol == Protocol::Direct {
        arguments_from + 1
    } else {
        0
    };
    let mut operands = Vec::new();
    let mut expanded = Vec::new();
    for (index, word) in projected.into_iter().enumerate().skip(from) {
        if protocol == Protocol::EnsembleRewrite && index > 0 && index <= arguments_from {
            operands.push(NativeNamedInvocationWord::Replacement(
                replacements[index - 1].clone(),
            ));
            expanded.push(false);
        } else {
            expanded.push(
                word.shape == crate::native_compilation::NativeCompilationWordShape::Expanded,
            );
            operands.push(NativeNamedInvocationWord::Original(word.operand));
        }
    }
    if protocol == Protocol::EnsembleRewrite && expanded.iter().any(|expanded| *expanded) {
        return Err(NativeInstructionPlanUnavailable::OperandGeometry);
    }
    Ok(NativeNamedInvocationInstruction {
        preparations: Vec::new(),
        name: name.to_vec(),
        protocol,
        arguments_from,
        words: operands,
        expanded,
    })
}

/// Ordered original-word append operands. Indices include the command head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAppendInstruction {
    /// String versus List physical mutation.
    pub kind: NativeAppendKind,
    /// Original value words, evaluated before the first variable operation.
    pub values: std::ops::Range<usize>,
    /// Actual compiler's single-object or argument-List instruction shape.
    pub operands: NativeAppendOperands,
}

/// The selected append compiler's stack value recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeAppendOperands {
    /// Each String operand has a separate append/store/observer boundary.
    StringObjects,
    /// One procedure-frame List element, without an intermediate List header.
    ListElement,
    /// Build native expansion segments, then execute one List append.
    ListElements {
        /// Ordered Word/List/Concat steps; no flattened substituted argv.
        steps: Vec<NativeArgumentListStep>,
        /// One expanded operand uses the actual full-range List instruction.
        strip_single_expanded: bool,
    },
}

/// Native argument-List construction, shared by both concrete producers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArgumentListStep {
    /// Evaluate one operand in the calling recipe's retained compiler vector.
    Word(usize),
    /// Construct a List from this many evaluated stack objects.
    List(usize),
    /// Concatenate the two genuine Lists on top of the operand stack.
    Concat,
}

/// Selected List-append instruction handling of an empty evaluated input.
/// This recipe does not authenticate an object or grant compiler admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeListAppendEmptyPublication {
    /// After validating an existing receiver, return it without copying/storing.
    pub skip_existing_store: bool,
    /// A missing receiver receives a fresh empty object instead of the input.
    pub fresh_missing_receiver: bool,
}

/// Project the actual native List-append instruction's empty-input branch.
/// C9.1 changes both branches; earlier instructions always publish the input
/// or the selected appended receiver, including for an empty input.
#[must_use]
pub fn native_list_append_empty_publication(
    protocol: tcl_syntax::native_string::NativeStringProtocol,
) -> Option<NativeListAppendEmptyPublication> {
    use tcl_dialect::TclVersion;
    use tcl_syntax::native_string::NativeStringProtocol;
    match protocol {
        NativeStringProtocol::C(version) if version >= TclVersion::V8_5 => {
            let changed = version == TclVersion::V9_1;
            Some(NativeListAppendEmptyPublication {
                skip_existing_store: changed,
                fresh_missing_receiver: changed,
            })
        }
        _ => None,
    }
}

pub(crate) fn argument_list_steps(
    words: &NativeCompilerWords<'_>,
    values: std::ops::Range<usize>,
    dialect: InvocationDialect,
) -> Vec<NativeArgumentListStep> {
    argument_list_steps_for_expansion(
        values.map(|index| (index, words.original_words()[index].group().expand)),
        dialect,
    )
}

pub(crate) fn argument_list_steps_for_expansion(
    words: impl IntoIterator<Item = (usize, bool)>,
    dialect: InvocationDialect,
) -> Vec<NativeArgumentListStep> {
    argument_list_steps_for_expansion_with_prefix(words, dialect, 0)
}

pub(crate) fn argument_list_steps_for_expansion_with_prefix(
    words: impl IntoIterator<Item = (usize, bool)>,
    dialect: InvocationDialect,
    prefix: usize,
) -> Vec<NativeArgumentListStep> {
    use NativeArgumentListStep as Step;
    let limit =
        crate::native_compilation::NativeTailcallStack::argument_list_segment_limit(dialect);
    let mut steps = Vec::new();
    let mut pending = prefix;
    let mut concatenated = false;
    for (index, expanded) in words {
        if expanded && pending > 0 {
            steps.push(Step::List(pending));
            if concatenated {
                steps.push(Step::Concat);
            }
            pending = 0;
            concatenated = true;
        }
        steps.push(Step::Word(index));
        if expanded {
            if concatenated {
                steps.push(Step::Concat);
            }
            concatenated = true;
        } else {
            pending += 1;
            if limit.is_some_and(|limit| pending > limit) {
                steps.push(Step::List(pending));
                if concatenated {
                    steps.push(Step::Concat);
                }
                pending = 0;
                concatenated = true;
            }
        }
    }
    if pending > 0 {
        steps.push(Step::List(pending));
        if concatenated {
            steps.push(Step::Concat);
        }
    } else if !concatenated {
        steps.push(Step::List(0));
    }
    steps
}

/// Missing portable instruction capability, distinct from a compiler decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInstructionPlanUnavailable {
    /// No independently selected actual C compiler/version point.
    CompilerPoint,
    /// The supplied receipt disagrees with this descriptor and original words.
    Selection,
    /// The complete original vector does not contain the selected operands.
    OperandGeometry,
    /// This descriptor has no portable instruction recipe in this owner.
    Operation,
    /// The original native variable operand could not be retained.
    Variable(NativeVariableWordUnavailable),
    /// The original List construction could not be retained.
    List(NativeCompiledListUnavailable),
    /// The original namespace binding compiler geometry could not be retained.
    NamespaceBindings(
        crate::native_namespace_binding_compilation::NativeNamespaceBindingUnavailable,
    ),
}

/// Project an independently admitted compiler selection onto its ordered
/// original operands. All indices address `words.original_words()`, including
/// the head; no index addresses substituted or expanded argv. Local-slot
/// eligibility and registration are supplied by the concrete compiler owner.
///
/// # Errors
/// Rejects unknown compiler points, conflicting selections, missing geometry
/// and operations without an implemented portable instruction recipe.
pub fn native_instruction_plan(
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    native_instruction_plan_for_purpose(
        spec,
        selection,
        words,
        operand_from,
        dialect,
        context,
        CompilerOperandPurpose::PublicInvocation,
    )
}

/// Project an independently selected original worker compiler onto the same
/// retained operands. The public ensemble selector words remain in `words`,
/// while `operand_from` identifies the worker's operands after those selectors.
/// This validates the registered-worker grammar; it supplies no registration,
/// namespace, compiler-hook, or callable authority.
///
/// # Errors
/// Rejects an unavailable compiler point, a conflicting worker selection,
/// unsupported original geometry, or a missing portable instruction recipe.
pub fn native_registered_worker_instruction_plan(
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    native_instruction_plan_for_purpose(
        spec,
        selection,
        words,
        operand_from,
        dialect,
        context,
        CompilerOperandPurpose::RegisteredWorker,
    )
}

#[derive(Clone, Copy)]
enum CompilerOperandPurpose {
    PublicInvocation,
    RegisteredWorker,
}

impl CompilerOperandPurpose {
    fn select(
        self,
        spec: NativeCompilationSpec,
        words: &NativeCompilerWords<'_>,
        operand_from: usize,
        dialect: InvocationDialect,
        context: NativeCompilationContext,
    ) -> NativeCompilationSelection {
        match self {
            Self::PublicInvocation => {
                spec.select_native_words(words, operand_from, Some(dialect), context)
            }
            Self::RegisteredWorker => spec.select_registered_worker_native_words(
                words,
                operand_from,
                Some(dialect),
                context,
            ),
        }
    }
}

fn original_operand_count(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
) -> Result<usize, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let count = words
        .original_words()
        .len()
        .checked_sub(operand_from)
        .ok_or(Unavailable::OperandGeometry)?;
    if operand_from == 0 {
        return Err(Unavailable::OperandGeometry);
    }
    Ok(count)
}

fn native_instruction_plan_for_purpose(
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
    purpose: CompilerOperandPurpose,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
        return Err(Unavailable::CompilerPoint);
    }
    let version = dialect.tcl_version.ok_or(Unavailable::CompilerPoint)?;
    if purpose.select(spec, words, operand_from, dialect, context) != selection {
        return Err(Unavailable::Selection);
    }
    if selection == NativeCompilationSelection::Generic
        && let Some(preparations) =
            original_dictionary_preparations(spec, words, operand_from, version, context)
    {
        return preparations.map(NativeInstructionPlan::GenericPreparation);
    }
    if matches!(
        selection,
        NativeCompilationSelection::NamedInvocation { .. }
    ) {
        return selected_named_instruction_plan(
            spec,
            selection,
            words,
            operand_from,
            dialect,
            context,
        );
    }
    if !instruction_selection_supported(spec, selection) {
        return Err(Unavailable::Selection);
    }
    if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = spec.grammar {
        return native_instruction_plan_for_purpose(
            *compiler,
            selection,
            words,
            operand_from,
            dialect,
            context,
            purpose,
        );
    }
    if matches!(
        spec.grammar,
        NativeCompilationGrammar::Tailcall
            | NativeCompilationGrammar::CoroutineYield
            | NativeCompilationGrammar::CoroutineRelay
    ) {
        return crate::native_coroutine_compilation::compile_native_coroutine(
            words,
            operand_from,
            spec.grammar,
            dialect,
            context,
        )
        .map(NativeInstructionPlan::Coroutine)
        .map_err(|_| Unavailable::OperandGeometry);
    }
    if let NativeCompilationGrammar::MathOperator(operator) = spec.grammar {
        return crate::native_mathop_compilation::compile_native_mathop(
            words,
            operand_from,
            operator,
            version,
            context,
        )
        .map_err(|_| Unavailable::OperandGeometry)?
        .map(NativeInstructionPlan::MathOperator)
        .ok_or(Unavailable::Selection);
    }
    if spec.grammar == NativeCompilationGrammar::Error {
        return crate::native_error_compilation::compile_native_error(words, operand_from, version)
            .map(NativeInstructionPlan::Error)
            .map_err(|_| Unavailable::OperandGeometry);
    }
    if spec.grammar == NativeCompilationGrammar::LiteralUnset {
        return crate::native_unset_compilation::compile_native_unset(words, operand_from, version)
            .map_err(|_| Unavailable::OperandGeometry)?
            .map(NativeInstructionPlan::Unset)
            .ok_or(Unavailable::Selection);
    }
    let count = original_operand_count(words, operand_from)?;
    if let Some(plan) =
        selected_nonvariable_instruction_plan(spec, words, operand_from, dialect, context)
    {
        return plan;
    }
    if !matches!(
        spec.grammar,
        NativeCompilationGrammar::VariableLoadStore
            | NativeCompilationGrammar::Increment
            | NativeCompilationGrammar::VariableAppend(_)
    ) {
        return Err(Unavailable::Operation);
    }
    selected_variable_instruction_plan(spec, words, operand_from, count, dialect, context)
}

fn instruction_selection_supported(
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
) -> bool {
    matches!(selection, NativeCompilationSelection::Inline { .. })
        || (selection == NativeCompilationSelection::Generic
            && (spec.namespace_binding_kind().is_some()
                || matches!(spec.grammar, NativeCompilationGrammar::Array { .. })))
        || (matches!(
            selection,
            NativeCompilationSelection::Generic | NativeCompilationSelection::CompileError
        ) && matches!(
            spec.grammar,
            NativeCompilationGrammar::Conditional
                | NativeCompilationGrammar::ForLoop
                | NativeCompilationGrammar::Catch
                | NativeCompilationGrammar::Try
                | NativeCompilationGrammar::Foreach
                | NativeCompilationGrammar::WhileLoop
        ))
}

fn selected_named_instruction_plan(
    spec: NativeCompilationSpec,
    selection: NativeCompilationSelection,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let NativeCompilationSelection::NamedInvocation {
        lookup,
        arguments_from,
        protocol,
    } = selection
    else {
        return Err(Unavailable::Selection);
    };
    if operand_from != 1
        || spec.select_native_words(words, operand_from, Some(dialect), context) != selection
    {
        return Err(Unavailable::Selection);
    }
    let replacements = lookup
        .prepended
        .iter()
        .take(arguments_from)
        .map(|value| value.as_bytes().to_vec())
        .collect::<Vec<_>>();
    native_named_invocation_instruction(
        words,
        dialect,
        lookup.slot.as_bytes(),
        arguments_from,
        protocol,
        &replacements,
    )
    .and_then(|mut recipe| {
        if let Some(preparations) = original_dictionary_preparations(
            spec,
            words,
            operand_from,
            dialect.tcl_version.ok_or(Unavailable::CompilerPoint)?,
            context,
        ) {
            recipe.preparations = preparations?;
        }
        Ok(NativeInstructionPlan::NamedInvocation(recipe))
    })
}

fn selected_uplevel_instruction(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
) -> Result<NativeUplevelInstruction, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let projected = words.checked_words().ok_or(Unavailable::OperandGeometry)?;
    let arguments = projected
        .get(operand_from..)
        .ok_or(Unavailable::OperandGeometry)?;
    let invocation =
        crate::InvocationWords::structured(projected[0], arguments).with_dialect(dialect);
    let layout = spec
        .uplevel_operands(invocation.arguments())
        .ok_or(Unavailable::Selection)?;
    let first = operand_from
        .checked_add(layout.script_from)
        .ok_or(Unavailable::OperandGeometry)?;
    if first >= words.original_words().len() {
        return Err(Unavailable::OperandGeometry);
    }
    Ok(NativeUplevelInstruction {
        level_word: layout
            .level
            .and_then(|index| operand_from.checked_add(index)),
        script_words: first..words.original_words().len(),
    })
}

fn selected_projected_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
    context: NativeCompilationContext,
) -> Option<Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>> {
    use NativeInstructionPlanUnavailable as Unavailable;
    if let NativeCompilationGrammar::Array { command, .. } = spec.grammar {
        return Some(
            crate::native_array_compilation::native_array_compilation(
                words,
                operand_from,
                command,
                version,
                context,
            )
            .map(NativeInstructionPlan::Array)
            .ok_or(Unavailable::OperandGeometry),
        );
    }
    if let NativeCompilationGrammar::NamespaceString(operation) = spec.grammar {
        return Some(
            crate::native_namespace_string_compilation::compile_native_namespace_string(
                words,
                operand_from,
                operation,
                version,
            )
            .map(NativeInstructionPlan::NamespaceString)
            .ok_or(Unavailable::OperandGeometry),
        );
    }
    if let Some(kind) = spec.introspection_compilation() {
        return Some(
            crate::native_introspection_compilation::compile_native_introspection(
                words,
                operand_from,
                kind,
                version,
            )
            .map(NativeInstructionPlan::Introspection)
            .ok_or(Unavailable::OperandGeometry),
        );
    }
    if let Some((operation, scope)) = spec.scalar_compilation() {
        return Some(
            crate::native_scalar_compilation::compile_native_scalar(
                words,
                operand_from,
                operation,
                scope,
                version,
            )
            .map(NativeInstructionPlan::Scalar)
            .ok_or(Unavailable::OperandGeometry),
        );
    }
    Some(match spec.grammar {
        NativeCompilationGrammar::Upvar => crate::native_upvar_compilation::compile_native_upvar(
            words,
            operand_from,
            version,
            context,
        )
        .map(NativeInstructionPlan::Upvar)
        .map_err(|_| Unavailable::OperandGeometry),
        NativeCompilationGrammar::InfoExists => {
            crate::native_info_exists_compilation::compile_native_info_exists(
                words,
                operand_from,
                version,
            )
            .map(NativeInstructionPlan::InfoExists)
            .map_err(|_| Unavailable::OperandGeometry)
        }
        _ => return None,
    })
}

fn selected_container_and_string_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
    context: NativeCompilationContext,
) -> Option<Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>> {
    use NativeInstructionPlanUnavailable as Unavailable;
    Some(match spec.grammar {
        NativeCompilationGrammar::Dictionary { command, ensemble } => {
            let from = operand_from + usize::from(ensemble);
            if matches!(
                command,
                crate::native_dictionary::NativeDictionaryCommand::Update
                    | crate::native_dictionary::NativeDictionaryCommand::With
            ) {
                return Some(
                    crate::native_dictionary_scope_compilation::compile_native_dictionary_scope(
                        command, words, from, version, context,
                    )
                    .map(NativeInstructionPlan::DictionaryScope)
                    .map_err(|_| Unavailable::OperandGeometry),
                );
            }
            if matches!(
                command,
                crate::native_dictionary::NativeDictionaryCommand::Set
                    | crate::native_dictionary::NativeDictionaryCommand::Unset
                    | crate::native_dictionary::NativeDictionaryCommand::Append
                    | crate::native_dictionary::NativeDictionaryCommand::Lappend
                    | crate::native_dictionary::NativeDictionaryCommand::Incr
            ) {
                return Some(
                    crate::native_dictionary_compilation::compile_native_dictionary_mutation(
                        command, words, from, version, context,
                    )
                    .map_err(|_| Unavailable::OperandGeometry)
                    .and_then(|recipe| match recipe.outcome {
                        crate::native_control_compilation::NativeControlOutcome::Inline(recipe) => {
                            Ok(NativeInstructionPlan::DictionaryMutation(recipe))
                        }
                        _ => Err(Unavailable::Selection),
                    }),
                );
            }
            crate::native_dictionary_compilation::compile_native_dictionary_lookup(
                command,
                words,
                operand_from + usize::from(ensemble),
                version,
            )
            .map(NativeInstructionPlan::DictionaryLookup)
            .map_err(|_| Unavailable::Operation)
        }
        NativeCompilationGrammar::StringTrim { scope, operation } => {
            crate::native_string_trim_compilation::instruction(
                words,
                operand_from,
                scope,
                version,
                operation,
            )
            .map(NativeInstructionPlan::StringTrim)
            .ok_or(Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::StringMatch(scope) => {
            crate::native_string_compilation::instruction(words, operand_from, scope, version)
                .map(NativeInstructionPlan::StringMatch)
                .ok_or(Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::ListRange | NativeCompilationGrammar::ListAssignment => {
            let kind = if spec.grammar == NativeCompilationGrammar::ListRange {
                crate::native_list_operations_compilation::NativeListOperationKind::Range
            } else {
                crate::native_list_operations_compilation::NativeListOperationKind::Assign
            };
            crate::native_list_operations_compilation::compile_native_list_operation(
                words,
                operand_from,
                version,
                kind,
            )
            .map(NativeInstructionPlan::ListOperations)
            .map_err(|_| Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::ListIndex => {
            crate::native_list_index_compilation::compile_native_list_index(
                words,
                operand_from,
                version,
            )
            .map(NativeInstructionPlan::ListIndex)
            .map_err(|_| Unavailable::OperandGeometry)
        }
        _ => return None,
    })
}

fn selected_nonvariable_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Option<Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let version = dialect.tcl_version?;
    if let Some(plan) =
        selected_projected_instruction_plan(spec, words, operand_from, version, context)
    {
        return Some(plan);
    }
    if let Some(plan) =
        selected_container_and_string_instruction_plan(spec, words, operand_from, version, context)
    {
        return Some(plan);
    }
    Some(match spec.grammar {
        NativeCompilationGrammar::Break => Ok(NativeInstructionPlan::Break),
        NativeCompilationGrammar::Uplevel => {
            selected_uplevel_instruction(spec, words, operand_from, dialect)
                .map(NativeInstructionPlan::Uplevel)
        }
        NativeCompilationGrammar::Continue => Ok(NativeInstructionPlan::Continue),
        NativeCompilationGrammar::Expression => {
            crate::native_expression_program::native_expression_instruction(
                words,
                operand_from,
                dialect,
            )
            .map(NativeInstructionPlan::Expression)
            .map_err(|_| Unavailable::Operation)
        }
        NativeCompilationGrammar::Try
        | NativeCompilationGrammar::Foreach
        | NativeCompilationGrammar::Conditional
        | NativeCompilationGrammar::ForLoop
        | NativeCompilationGrammar::Catch
        | NativeCompilationGrammar::WhileLoop => {
            selected_control_instruction_plan(spec, words, operand_from, dialect, context)
        }
        NativeCompilationGrammar::TclOoHelper(helper) => {
            crate::native_tcloo_compilation::instruction(helper, words, operand_from, dialect)
                .map(NativeInstructionPlan::TclOoHelper)
                .ok_or(Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::NamespaceLegacy
        | NativeCompilationGrammar::NamespaceUpvarBindings
        | NativeCompilationGrammar::GlobalBindings
        | NativeCompilationGrammar::NamespaceVariableBindings => {
            let kind = spec.namespace_binding_kind()?;
            crate::native_namespace_binding_compilation::compile_native_namespace_bindings(
                words,
                operand_from,
                version,
                context,
                kind,
            )
            .map(NativeInstructionPlan::NamespaceBindings)
            .map_err(Unavailable::NamespaceBindings)
        }
        NativeCompilationGrammar::Switch => {
            crate::native_switch_compilation::native_switch_instruction(
                words,
                operand_from,
                version,
            )
            .map(NativeInstructionPlan::Switch)
            .map_err(|_| Unavailable::Operation)
        }
        NativeCompilationGrammar::ArgumentList => words
            .list_recipe(operand_from, version)
            .map(NativeInstructionPlan::List)
            .map_err(Unavailable::List),
        NativeCompilationGrammar::ArgumentConcatFrom(_) => {
            selected_concat_instruction(words, operand_from, version)
                .map(NativeInstructionPlan::Concat)
        }
        NativeCompilationGrammar::Return => {
            crate::native_return_compilation::native_return_instruction(
                words,
                operand_from,
                version,
                context,
            )
            .map(NativeInstructionPlan::ReturnOptions)
            .map_err(|_| Unavailable::Operation)
        }
        _ => return None,
    })
}

fn selected_concat_instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
) -> Result<NativeConcatInstruction, NativeInstructionPlanUnavailable> {
    use crate::native_compiler_word_projection::project_native_compiler_words;
    use NativeInstructionPlanUnavailable as Unavailable;
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Unavailable::OperandGeometry)?;
    let operands = projected
        .get(operand_from..)
        .ok_or(Unavailable::OperandGeometry)?;
    if let Some(literals) = operands
        .iter()
        .map(|word| word.literal.as_deref())
        .collect::<Option<Vec<_>>>()
    {
        let allocation = if version == tcl_dialect::TclVersion::V9_1 && !literals.is_empty() {
            if literals.iter().all(|bytes| bytes.is_empty()) {
                NativeConcatLiteralAllocation::PrivateEmpty
            } else {
                NativeConcatLiteralAllocation::PrivateString
            }
        } else {
            NativeConcatLiteralAllocation::Registered
        };
        Ok(NativeConcatInstruction::Literal {
            bytes: tcl_syntax::list::concat_bytes(literals),
            allocation,
        })
    } else {
        Ok(NativeConcatInstruction::Operands(
            operands.iter().map(|word| word.operand.clone()).collect(),
        ))
    }
}

fn selected_control_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let version = dialect.tcl_version.ok_or(Unavailable::CompilerPoint)?;
    match spec.grammar {
        NativeCompilationGrammar::Try => {
            crate::native_try_compilation::compile_native_try(words, operand_from, version, context)
                .map(NativeInstructionPlan::Try)
                .map_err(|_| Unavailable::Operation)
        }
        NativeCompilationGrammar::Foreach => crate::native_each_compilation::compile_native_each(
            words,
            operand_from,
            version,
            context,
            spec.each_collection().ok_or(Unavailable::Operation)?,
        )
        .map(NativeInstructionPlan::Each)
        .map_err(|_| Unavailable::Operation),
        _ => crate::native_control_instructions::native_control_instruction(
            spec.grammar,
            words,
            operand_from,
            dialect,
            context,
        )
        .map(NativeInstructionPlan::Control)
        .map_err(|_| Unavailable::Operation),
    }
}

fn selected_variable_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    count: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let version = dialect.tcl_version.ok_or(Unavailable::CompilerPoint)?;
    let original = words
        .original_words()
        .get(operand_from)
        .ok_or(Unavailable::OperandGeometry)?;
    let target = native_variable_word(original, version, words.source_protocol())
        .map_err(Unavailable::Variable)?;
    match (spec.grammar, count) {
        (
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::String)
            | NativeCompilationGrammar::VariableLoadStore,
            1,
        ) => Ok(NativeInstructionPlan::Load {
            target_word: operand_from,
            target,
        }),
        (NativeCompilationGrammar::VariableAppend(kind), 1..) => {
            let values = operand_from + 1..words.original_words().len();
            let operands = if kind == NativeAppendKind::String {
                NativeAppendOperands::StringObjects
            } else if values.len() == 1
                && !words.original_words()[values.start].group().expand
                && context.frame == NativeCompilationFrame::ProcedureCode
            {
                NativeAppendOperands::ListElement
            } else {
                NativeAppendOperands::ListElements {
                    steps: argument_list_steps(words, values.clone(), dialect),
                    strip_single_expanded: values.len() == 1
                        && words.original_words()[values.start].group().expand,
                }
            };
            Ok(NativeInstructionPlan::Append {
                target_word: operand_from,
                target,
                recipe: NativeAppendInstruction {
                    kind,
                    values,
                    operands,
                },
            })
        }
        (NativeCompilationGrammar::VariableLoadStore, 2) => Ok(NativeInstructionPlan::Store {
            target_word: operand_from,
            target,
            value_word: operand_from + 1,
        }),
        (NativeCompilationGrammar::Increment, 1 | 2) => {
            let amount_word = (count == 2).then_some(operand_from + 1);
            Ok(NativeInstructionPlan::Increment {
                target_word: operand_from,
                target,
                amount_word,
                immediate: amount_word
                    .map_or(Some(1), |index| words.increment_immediate(index, version)),
            })
        }
        _ => Err(Unavailable::OperandGeometry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SemanticOperationId;
    use crate::hooks::LoweringHookId;
    use crate::native_compilation::{
        NativeBodyCompilation, NativeCompilationFrame, NativeCompilationMode,
    };
    use tcl_dialect::TclVersion;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn concat_result_hex(bytes: &[u8]) -> String {
        use std::fmt::Write;
        bytes.iter().fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
    }

    fn project(
        source: &[u8],
        grammar: NativeCompilationGrammar,
        version: TclVersion,
    ) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
        let profile =
            tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string())).unwrap();
        let image = SourceImage::native(source);
        let end = u32::try_from(image.len()).unwrap();
        let script = native_script_words_in(
            image,
            Span::new(0, end),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &script.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            loop_depth: 0,
            catch_depth: Some(0),
        };
        let spec = NativeCompilationSpec {
            grammar,
            operation: SemanticOperationId::StructuredLowering(match grammar {
                NativeCompilationGrammar::Increment => LoweringHookId::Incr,
                NativeCompilationGrammar::Return => LoweringHookId::Return,
                NativeCompilationGrammar::Uplevel => LoweringHookId::Uplevel,
                NativeCompilationGrammar::Catch => LoweringHookId::Catch,
                NativeCompilationGrammar::Foreach => LoweringHookId::Foreach,
                NativeCompilationGrammar::Try => LoweringHookId::Try,
                NativeCompilationGrammar::VariableAppend(_) => LoweringHookId::AppendOrLappend,
                _ => LoweringHookId::Set,
            }),
            body: NativeBodyCompilation::Inherit,
        };
        let dialect = InvocationDialect::for_version(version);
        let selection = spec.select_native_words(&words, 1, Some(dialect), context);
        native_instruction_plan(spec, selection, &words, 1, dialect, context)
    }

    #[test]
    fn concat_retains_native_constant_allocations_and_original_dynamic_operands() {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
        use NativeConcatLiteralAllocation as Allocation;
        let grammar = NativeCompilationGrammar::ArgumentConcatFrom(TclVersion::V8_6);
        for version in TclVersion::ALL {
            for (source, bytes, private) in [
                (b"concat".as_slice(), b"".as_slice(), Allocation::Registered),
                (b"concat {}", b"", Allocation::PrivateEmpty),
                (b"concat {} {}", b"", Allocation::PrivateEmpty),
                (b"concat { } {\t}", b"", Allocation::PrivateString),
                (b"concat { A B } { C }", b"A B C", Allocation::PrivateString),
            ] {
                let selected = project(source, grammar, version);
                if version < TclVersion::V8_6 {
                    assert!(
                        matches!(selected, Err(NativeInstructionPlanUnavailable::Selection)),
                        "{version:?}: {source:?}: {selected:?}"
                    );
                } else {
                    assert_eq!(
                        selected,
                        Ok(NativeInstructionPlan::Concat(
                            NativeConcatInstruction::Literal {
                                bytes: bytes.to_vec(),
                                allocation: if version == TclVersion::V9_1 {
                                    private
                                } else {
                                    Allocation::Registered
                                },
                            }
                        ))
                    );
                }
            }
            if version >= TclVersion::V8_6 {
                assert_eq!(
                    project(b"concat $a $b", grammar, version),
                    Ok(NativeInstructionPlan::Concat(
                        NativeConcatInstruction::Operands(vec![
                            Operand::Original(1),
                            Operand::Original(2)
                        ])
                    ))
                );
                assert!(matches!(
                    project(b"concat {*}$a", grammar, version),
                    Err(NativeInstructionPlanUnavailable::Selection)
                ));
                assert!(
                    matches!(project(b"concat {*}{A B} $c", grammar, version), Ok(NativeInstructionPlan::Concat(NativeConcatInstruction::Operands(ref operands))) if operands.len() == 3)
                );
            }
        }
    }

    mod concat_expansion_controls {
        use super::*;
        include!("../tests/data/native_concat_expansion/cases.rs");

        #[test]
        fn concat_expansion_matches_original_native_parser_and_instruction_windows() {
            use crate::native_compilation::NativeCompilationWordShape as Shape;
            use crate::native_compiler_word_projection::project_native_compiler_words;
            let grammar = NativeCompilationGrammar::ArgumentConcatFrom(TclVersion::V8_6);
            for version in TclVersion::ALL
                .into_iter()
                .filter(|v| *v >= TclVersion::V8_5)
            {
                let engine = format!("tcl{}", version.version_string());
                let profile = tcl_dialect::DialectProfile::find(&engine).unwrap();
                for (case, source) in EXPANSION_BODIES.iter().enumerate() {
                    let prefix = format!("OP\t{case}\t");
                    let rows = expansion_rows(&engine);
                    let opcodes: Vec<_> = rows
                        .lines()
                        .filter(|line| line.starts_with(&prefix))
                        .map(|line| line.split('\t').nth(3).unwrap())
                        .collect();
                    let image = SourceImage::native(*source);
                    let script = native_script_words_in(
                        image,
                        Span::new(0, u32::try_from(source.len()).unwrap()),
                        LexerConfig::from_grammar(profile.grammar),
                    )
                    .unwrap();
                    let words = NativeCompilerWords::capture(
                        &script.commands[0].words,
                        NativeStringProtocol::C(version),
                    )
                    .unwrap();
                    let projected = project_native_compiler_words(&words, version).unwrap();
                    let prefix = format!("TOKEN\t{case}\t");
                    let native_shapes: Vec<_> = rows
                        .lines()
                        .filter(|line| line.starts_with(&prefix))
                        .filter_map(|line| match line.split('\t').nth(3).unwrap() {
                            "1" => Some(Shape::Substituted),
                            "2" => Some(Shape::Literal),
                            "256" => Some(Shape::Expanded),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(
                        projected.iter().map(|word| word.shape).collect::<Vec<_>>(),
                        native_shapes,
                        "{engine}/{case}"
                    );
                    let selected = project(source, grammar, version);
                    if opcodes.contains(&"concatStk") {
                        let Ok(NativeInstructionPlan::Concat(NativeConcatInstruction::Operands(
                            operands,
                        ))) = selected
                        else {
                            panic!("{engine}/{case}: {selected:?}");
                        };
                        assert_eq!(
                            operands,
                            projected[1..]
                                .iter()
                                .map(|word| word.operand.clone())
                                .collect::<Vec<_>>()
                        );
                    } else if version >= TclVersion::V8_6 && !opcodes.contains(&"invokeExpanded") {
                        let Ok(NativeInstructionPlan::Concat(NativeConcatInstruction::Literal {
                            bytes,
                            ..
                        })) = selected
                        else {
                            panic!("{engine}/{case}: {selected:?}");
                        };
                        assert_eq!(
                            concat_result_hex(&bytes),
                            expansion_result(&engine, case)[6]
                        );
                    } else {
                        assert!(
                            matches!(selected, Err(NativeInstructionPlanUnavailable::Selection)),
                            "{engine}/{case}: {selected:?}"
                        );
                    }
                }
            }
        }
    }

    // Native proof: naming.variable.uplevel-original-compilation-opcode-and-concat-frontier
    // docs/design/analysis/name-resolution-proofs/variable.uplevel-original-compilation-opcode-and-concat-frontier.md
    #[test]
    fn original_uplevel_instruction_keeps_level_and_script_stack_geometry() {
        for row in include_str!("../tests/data/native_uplevel_compilation/instructions.tsv")
            .lines()
            .skip(1)
        {
            let fields = row.split('\t').collect::<Vec<_>>();
            let version = match fields[0] {
                "9.0" => TclVersion::V9_0,
                "9.1" => TclVersion::V9_1,
                _ => unreachable!("measured compiler fixture"),
            };
            let selected = project(
                fields[2].as_bytes(),
                NativeCompilationGrammar::Uplevel,
                version,
            );
            assert_eq!(
                selected.is_ok(),
                fields[3] == "1",
                "{}/{}",
                fields[0],
                fields[1]
            );
            if let Ok(NativeInstructionPlan::Uplevel(recipe)) = selected {
                assert_eq!(recipe.script_words.len() > 1, fields[4] == "1");
            }
        }
        for (source, level, scripts) in [
            (b"uplevel 1 {set x OUTER}".as_slice(), Some(1), 2..3),
            (b"uplevel {set x OUTER}".as_slice(), None, 1..2),
            (b"uplevel 1 set x OUTER".as_slice(), Some(1), 2..5),
        ] {
            assert_eq!(
                project(source, NativeCompilationGrammar::Uplevel, TclVersion::V9_1),
                Ok(NativeInstructionPlan::Uplevel(NativeUplevelInstruction {
                    level_word: level,
                    script_words: scripts,
                })),
            );
            assert!(project(source, NativeCompilationGrammar::Uplevel, TclVersion::V9_0).is_err());
        }
        for source in [
            b"uplevel $level {set x OUTER}".as_slice(),
            b"uplevel -1 {set x OUTER}",
            b"uplevel 1",
        ] {
            assert!(project(source, NativeCompilationGrammar::Uplevel, TclVersion::V9_1).is_err());
        }
    }

    #[test]
    fn installed_named_worker_uses_original_post_selector_operands() {
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
        let registry = crate::CommandRegistry::build_default();
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = InvocationDialect::for_version(version);
            let spec = registry
                .native_compilation_for_registration("::tcl::info::locals", dialect)
                .unwrap();
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            for (source, operand_count, accepted) in [
                (b"info locals".as_slice(), 0, true),
                (b"info loc $pattern".as_slice(), 1, true),
                (b"info locals {*}{a*}".as_slice(), 1, true),
                (b"info locals a b".as_slice(), 2, false),
                (b"info locals {*}$patterns".as_slice(), 1, false),
            ] {
                let image = SourceImage::native(source);
                let script = native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap();
                let words = NativeCompilerWords::capture(
                    &script.commands[0].words,
                    NativeStringProtocol::C(version),
                )
                .unwrap();
                let result =
                    native_named_worker_instruction(spec, &words, 2, dialect, b"::actual:::locals");
                if !accepted || version == TclVersion::V8_5 {
                    assert_eq!(result, NativeNamedWorkerCompilation::Generic);
                    continue;
                }
                let NativeNamedWorkerCompilation::Named(recipe) = result else {
                    panic!("selected installed worker recipe: {source:?}")
                };
                assert_eq!(recipe.name, b"::actual:::locals");
                assert_eq!(recipe.arguments_from, 1);
                assert_eq!(recipe.words.len(), operand_count);
                assert!(recipe.expanded.iter().all(|expanded| !expanded));
                if source == b"info loc $pattern" {
                    assert_eq!(
                        recipe.words,
                        [NativeNamedInvocationWord::Original(Operand::Original(2))]
                    );
                }
                assert_eq!(
                    native_named_worker_instruction(
                        spec,
                        &words,
                        words.shapes().len() + 1,
                        dialect,
                        b"ignored"
                    ),
                    NativeNamedWorkerCompilation::Unavailable
                );
            }
        }
    }

    #[test]
    fn named_layout_preserves_original_expansions_and_canonical_rewrite_words() {
        use crate::native_compilation::NativeNamedInvocationProtocol as Protocol;
        use crate::native_compiler_word_projection::NativeCompilerWordOperand as Operand;
        let version = TclVersion::V8_6;
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let image = SourceImage::native(b"ensemble selected {*}{A B} {*}$tail".as_slice());
        let script = native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &script.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        let direct = native_named_invocation_instruction(
            &words,
            InvocationDialect::for_version(version),
            b"::private",
            1,
            Protocol::Direct,
            &[],
        )
        .unwrap();
        assert_eq!(direct.words.len(), 3);
        assert_eq!(direct.expanded, [false, false, true]);
        assert!(
            matches!(&direct.words[0], NativeNamedInvocationWord::Original(Operand::LiteralExpansion { value, original_word: 2, .. }) if value == b"A")
        );
        assert_eq!(
            direct.words[2],
            NativeNamedInvocationWord::Original(Operand::Original(3))
        );
        assert_eq!(
            native_named_invocation_instruction(
                &words,
                InvocationDialect::for_version(version),
                b"::private",
                1,
                Protocol::EnsembleRewrite,
                &[b"selected".to_vec()]
            ),
            Err(NativeInstructionPlanUnavailable::OperandGeometry)
        );
        let image = SourceImage::native(b"ensemble sel value".as_slice());
        let script = native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &script.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        let rewrite = native_named_invocation_instruction(
            &words,
            InvocationDialect::for_version(version),
            b"::private",
            1,
            Protocol::EnsembleRewrite,
            &[b"selected".to_vec()],
        )
        .unwrap();
        assert_eq!(
            rewrite.words,
            [
                NativeNamedInvocationWord::Original(Operand::Original(0)),
                NativeNamedInvocationWord::Replacement(b"selected".to_vec()),
                NativeNamedInvocationWord::Original(Operand::Original(2))
            ]
        );
        assert!(
            native_named_invocation_instruction(
                &words,
                InvocationDialect::for_version(version),
                b"::private",
                1,
                Protocol::EnsembleRewrite,
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn rejected_each_retains_its_original_list_failure() {
        use crate::native_control_compilation::NativeControlOutcome;
        let source = b"foreach \"{i\" {A} {set reached 1}";
        let NativeInstructionPlan::Each(recipe) =
            project(source, NativeCompilationGrammar::Foreach, TclVersion::V8_4).unwrap()
        else {
            panic!("original iterator recipe");
        };
        let NativeControlOutcome::Rejected(failure) = recipe.outcome else {
            panic!("native compiler rejects malformed variable list");
        };
        assert_eq!(
            failure.message.as_deref(),
            Some("unmatched open brace in list")
        );
        for version in TclVersion::ALL
            .into_iter()
            .filter(|version| *version > TclVersion::V8_4)
        {
            let NativeInstructionPlan::Each(recipe) =
                project(source, NativeCompilationGrammar::Foreach, version).unwrap()
            else {
                panic!("original iterator recipe");
            };
            assert_eq!(recipe.outcome, NativeControlOutcome::Generic);
        }
    }

    #[test]
    fn instruction_operands_keep_original_opaque_names_and_version_geometry() {
        for version in TclVersion::ALL {
            let plan = project(
                b"set x\xff\0tail VALUE",
                NativeCompilationGrammar::VariableLoadStore,
                version,
            )
            .unwrap();
            assert!(matches!(plan, NativeInstructionPlan::Store {
                target_word: 1, target: NativeVariableWordOperand::Literal { ref name, index: None, .. }, value_word: 2
            } if name == b"x\xff\0tail"));
            let braced = project(
                b"set {a(k)} VALUE",
                NativeCompilationGrammar::VariableLoadStore,
                version,
            )
            .unwrap();
            assert_eq!(
                matches!(
                    braced,
                    NativeInstructionPlan::Store {
                        target: NativeVariableWordOperand::DynamicWord,
                        ..
                    }
                ),
                version == TclVersion::V8_4
            );
        }
    }

    #[test]
    fn increment_immediates_use_the_shared_original_amount_recipe() {
        for version in TclVersion::ALL {
            for (source, expected) in [
                (b"incr x -127".as_slice(), Some(-127)),
                (b"incr x -128".as_slice(), None),
            ] {
                assert!(
                    matches!(project(source, NativeCompilationGrammar::Increment, version).unwrap(),
                    NativeInstructionPlan::Increment { amount_word: Some(2), immediate, .. } if immediate == expected)
                );
            }
        }
    }

    #[test]
    fn list_append_empty_publication_retains_selected_native_instruction_branches() {
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let policy = native_list_append_empty_publication(NativeStringProtocol::C(version))
                .expect("actual List-append instruction");
            assert_eq!(policy.skip_existing_store, version == TclVersion::V9_1);
            assert_eq!(policy.fresh_missing_receiver, version == TclVersion::V9_1);
        }
        assert!(
            native_list_append_empty_publication(NativeStringProtocol::C(TclVersion::V8_4))
                .is_none()
        );
    }

    #[test]
    fn append_retains_original_value_order_and_native_expansion_stack_shape() {
        for version in TclVersion::ALL {
            let grammar = NativeCompilationGrammar::VariableAppend(NativeAppendKind::List);
            let one = project(b"lappend x\xff\0tail $value", grammar, version).unwrap();
            assert!(matches!(one, NativeInstructionPlan::Append {
                target_word: 1,
                target: NativeVariableWordOperand::Literal { ref name, index: None, .. },
                recipe: NativeAppendInstruction { ref values, operands: NativeAppendOperands::ListElement, .. }
            } if name == b"x\xff\0tail" && values == &(2..3)));
            let string = project(
                b"append x\xff\0tail $first",
                NativeCompilationGrammar::VariableAppend(NativeAppendKind::String),
                version,
            )
            .unwrap();
            assert!(matches!(
                string,
                NativeInstructionPlan::Append {
                    recipe: NativeAppendInstruction {
                        operands: NativeAppendOperands::StringObjects,
                        ..
                    },
                    ..
                }
            ));
            assert!(matches!(
                project(
                    b"append x",
                    NativeCompilationGrammar::VariableAppend(NativeAppendKind::String),
                    version
                )
                .unwrap(),
                NativeInstructionPlan::Load { .. }
            ));
        }
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let many = project(
                b"lappend x $first $second",
                NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
                version,
            )
            .unwrap();
            assert!(matches!(many, NativeInstructionPlan::Append {
                recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: false }, .. }, ..
            } if steps == &[NativeArgumentListStep::Word(2), NativeArgumentListStep::Word(3), NativeArgumentListStep::List(2)]));
        }
        let expanded = project(
            b"lappend x A {*}$middle Z",
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
            TclVersion::V9_1,
        )
        .unwrap();
        assert!(matches!(expanded, NativeInstructionPlan::Append {
            recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: false }, .. }, ..
        } if steps == &[
            NativeArgumentListStep::Word(2), NativeArgumentListStep::List(1),
            NativeArgumentListStep::Word(3), NativeArgumentListStep::Concat,
            NativeArgumentListStep::Word(4), NativeArgumentListStep::List(1), NativeArgumentListStep::Concat,
        ]));
        let expanded = project(
            b"lappend x {*}$values",
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
            TclVersion::V9_1,
        )
        .unwrap();
        assert!(matches!(expanded, NativeInstructionPlan::Append {
            recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: true }, .. }, ..
        } if steps == &[NativeArgumentListStep::Word(2)]));
        let empty = project(
            b"lappend x",
            NativeCompilationGrammar::VariableAppend(NativeAppendKind::List),
            TclVersion::V9_1,
        )
        .unwrap();
        assert!(matches!(empty, NativeInstructionPlan::Append {
            recipe: NativeAppendInstruction { operands: NativeAppendOperands::ListElements { ref steps, strip_single_expanded: false }, .. }, ..
        } if steps == &[NativeArgumentListStep::List(0)]));
    }

    #[test]
    fn descriptor_without_portable_instruction_cannot_become_generic() {
        assert_eq!(
            project(
                b"linsert value 0 X",
                NativeCompilationGrammar::ListInsertion,
                TclVersion::V9_0
            ),
            Err(NativeInstructionPlanUnavailable::Operation)
        );
    }

    #[test]
    fn original_scalar_descriptor_has_a_retained_native_instruction() {
        use crate::native_scalar_compilation::{NativeScalarOperation, NativeScalarScope};
        let recipe = project(
            b"string length value",
            NativeCompilationGrammar::StringLength(NativeScalarScope::PublicMember),
            TclVersion::V8_5,
        )
        .unwrap();
        assert!(matches!(
            recipe,
            NativeInstructionPlan::Scalar(scalar)
                if scalar.operation == NativeScalarOperation::StringLength
                    && scalar.operands == [crate::native_compiler_word_projection::NativeCompilerWordOperand::Original(2)]
        ));
    }

    #[test]
    fn return_preserves_original_result_operand_and_selected_options_recipe() {
        use crate::native_return_compilation::{
            NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
        };
        for version in TclVersion::ALL {
            for (source, value_word) in [
                (b"return".as_slice(), None),
                (b"return \xff\0tail".as_slice(), Some(1)),
                (b"return $result".as_slice(), Some(1)),
            ] {
                let NativeInstructionPlan::ReturnOptions(recipe) =
                    project(source, NativeCompilationGrammar::Return, version).unwrap()
                else {
                    panic!("original Return plan")
                };
                assert_eq!(recipe.value_word, value_word);
                assert_eq!(
                    recipe.compiler_word_visits().collect::<Vec<_>>(),
                    value_word.into_iter().collect::<Vec<_>>()
                );
                assert_eq!(recipe.exit, Exit::Done);
                let Options::Static(options) = recipe.options else {
                    panic!("native default controls")
                };
                assert_eq!((options.code, options.level, options.size), (0, 1, 0));
            }
        }
        let NativeInstructionPlan::ReturnOptions(recipe) = project(
            b"return -code ok VALUE",
            NativeCompilationGrammar::Return,
            TclVersion::V9_0,
        )
        .unwrap() else {
            panic!("static options")
        };
        assert_eq!(recipe.exit, Exit::Immediate);
        assert_eq!(recipe.value_word, Some(3));
        let Options::Static(options) = recipe.options else {
            panic!("merged private header")
        };
        assert_eq!((options.code, options.level, options.size), (0, 1, 0));
    }
    #[test]
    fn native_return_compilation_retains_static_stack_and_generic_frontiers() {
        use crate::native_return_compilation::{
            NativeReturnExit as Exit, NativeReturnOptionsOperand as Options,
        };
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let NativeInstructionPlan::ReturnOptions(recipe) = project(
                b"return -level 0 -code error -options {-custom kept -errorcode CUSTOM} BODY",
                NativeCompilationGrammar::Return,
                version,
            )
            .unwrap() else {
                panic!("retained Return")
            };
            assert_eq!(recipe.exit, Exit::Immediate);
            assert_eq!(recipe.value_word, Some(7));
            assert_eq!(recipe.compiler_word_visits().collect::<Vec<_>>(), vec![7]);
            let Options::Static(literal) = recipe.options else {
                panic!("private options")
            };
            assert_eq!((literal.code, literal.level, literal.size), (1, 0, 2));
            let NativeInstructionPlan::ReturnOptions(recipe) = project(
                b"return -options $options $result",
                NativeCompilationGrammar::Return,
                version,
            )
            .unwrap() else {
                panic!("stack Return")
            };
            assert_eq!(recipe.options, Options::StackWord(2));
            assert_eq!(recipe.exit, Exit::Stack);
            assert_eq!(
                recipe.compiler_word_visits().collect::<Vec<_>>(),
                vec![2, 3]
            );
            let dynamic = project(
                b"return -code $code $result",
                NativeCompilationGrammar::Return,
                version,
            );
            if version == TclVersion::V8_5 {
                assert_eq!(dynamic, Err(NativeInstructionPlanUnavailable::Selection));
            } else {
                let Ok(NativeInstructionPlan::ReturnOptions(recipe)) = &dynamic else {
                    panic!("original dynamic return recipe");
                };
                assert_eq!(
                    recipe.compiler_word_visits().collect::<Vec<_>>(),
                    vec![1, 2, 3]
                );
                assert!(
                    matches!(dynamic,Ok(NativeInstructionPlan::ReturnOptions(crate::native_return_compilation::NativeReturnInstruction{options:Options::StackPairs(range),..})) if range==(1..3))
                );
            }
            assert_eq!(
                project(
                    b"return -level -1 BODY",
                    NativeCompilationGrammar::Return,
                    version
                ),
                Err(NativeInstructionPlanUnavailable::Selection)
            );
        }
    }
}

#[cfg(test)]
mod registered_worker_tests;

/// Actual original dictionary preparation preceding the selected operation or fallback.
/// This supplies no selected compiler registration or command authority.
#[must_use]
pub fn original_dictionary_preparations(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
    context: NativeCompilationContext,
) -> Option<
    Result<
        Vec<crate::native_control_compilation::NativeControlPreparationStep>,
        NativeInstructionPlanUnavailable,
    >,
> {
    if let NativeCompilationGrammar::WithImplementationPath { compiler, .. } = spec.grammar {
        return original_dictionary_preparations(*compiler, words, operand_from, version, context);
    }
    let NativeCompilationGrammar::Dictionary { command, ensemble } = spec.grammar else {
        return None;
    };
    command
        .original_preparations(
            words,
            operand_from + usize::from(ensemble),
            version,
            context,
        )
        .map(|selected| selected.map_err(|_| NativeInstructionPlanUnavailable::OperandGeometry))
}
