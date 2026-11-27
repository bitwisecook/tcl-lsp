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

//! Portable original-word instruction recipes for admitted C compiler operations.
//!
//! The concrete producer must independently authenticate the selected command
//! token, compiler attachment and implementation prerequisites. This projection
//! does not grant that authority or create a local-variable table.

use crate::native_compilation::{
    NativeAppendKind, NativeCompilationContext, NativeCompilationFrame, NativeCompilationGrammar,
    NativeCompilationSelection, NativeCompilationSpec,
};
use crate::native_compiler_words::{
    NativeCompiledListRecipe, NativeCompiledListUnavailable, NativeCompilerWords,
};
use tcl_native_model::InvocationDialect;
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
    /// `TclCompileStringMapCmd`'s inline two-element literal map.
    StringMap(crate::native_string_map_compilation::NativeStringMapInstruction),
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
    /// `TclCompileStringIsCmd` beyond the scalar `ascii` form.
    StringIs(crate::native_string_is_compilation::NativeStringIsInstruction),
    /// Original `dict create` constant, put or worker construction.
    DictionaryCreate(crate::native_dictionary_compilation::NativeDictionaryCreateInstruction),
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
    /// Original format literal fragments and ordered value-word compiler visits.
    FormatStringConcat(NativeFormatStringInstruction),
    /// `TclCompileSyntaxError`: push the message and return options, then
    /// `SYNTAX 1 0`.
    SyntaxError {
        /// The error message literal.
        message: Vec<u8>,
        /// The return-options literal.
        options: Vec<u8>,
    },
    /// C clockRead selector, as retained by the installed native compiler.
    ClockRead(u8),
    /// Ordered root/current namespace bindings, including declined preparation.
    NamespaceBindings(
        crate::native_namespace_binding_compilation::NativeNamespaceBindingCompilation,
    ),
    /// Actual original helper stack/frame instruction geometry.
    TclOoHelper(crate::native_tcloo_compilation::NativeTclOoInstruction),
    /// Original pattern and subject followed by the selected C string matcher.
    StringMatch(crate::native_string_compilation::NativeStringMatchInstruction),
    /// C9 regexp pattern (original word or converted glob), subject and matcher.
    Regexp(crate::native_regexp_compilation::NativeRegexpInstruction),
    /// `TclSubstCompile` over the original template word.
    Substitution(crate::substitution::NativeSubstitutionInstruction),
    /// Original trim subject and optional original character set.
    StringTrim(crate::native_string_trim_compilation::NativeStringTrimInstruction),
    /// Original fixed String operands followed by the selected instruction.
    StringOperand(crate::native_string_operand_compilation::NativeStringOperandInstruction),
    /// Original subject and compile-known C9 range coordinates.
    StringRange(crate::native_string_range_compilation::NativeStringRangeInstruction),
    /// `TclCompileStringReplaceCmd`'s recipe.
    StringReplace(crate::native_string_replace_compilation::NativeStringReplaceInstruction),
    /// `TclCompileStringInsertCmd`'s recipe.
    StringInsert(crate::native_string_insert_compilation::NativeStringInsertInstruction),
    /// `CompileDictEachCmd`'s `dict for`/`dict map` loop.
    DictionaryEach(
        crate::native_control_compilation::NativeControlCompilation<
            crate::native_dictionary_each_compilation::NativeDictionaryEachInstruction,
        >,
    ),
    /// `TclCompileLreplaceCmd`'s recipe.
    ListReplace(crate::native_list_replace_compilation::NativeListReplaceInstruction),
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
    /// `const`: make the selected scalar a constant holding the value of an
    /// original word, then push the empty result.
    Constant {
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

/// A selected format compiler pushes these pieces in their exact lexical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFormatStringInstruction {
    /// Nonempty format fragments and each original value word, once.
    pub pieces: Vec<NativeFormatStringPiece>,
    /// Literal registration is release-specific even when bytes agree.
    pub literal_allocation: NativeFormatLiteralAllocation,
}

/// A format literal never substitutes; a value retains its original compiler word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeFormatStringPiece {
    /// Decoded nonempty ASCII format bytes, with `%%` replaced by `%`.
    Literal(Vec<u8>),
    /// Exact index in the complete original word vector, including its head.
    Value(usize),
}

/// Original C format compiler literal-header construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFormatLiteralAllocation {
    /// C8.6/C9.0 register bytes in the native literal tables.
    Registered,
    /// C9.1 adds each fresh String object without local or global sharing.
    PrivateString,
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
        hook_from,
        arity,
        later_arity,
        ..
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
        || !u16::try_from(operands.len()).is_ok_and(|count| {
            crate::native_compilation::named_arity_at(arity, later_arity, version).accepts(count)
        })
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

fn original_decline_preparations(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    dialect: InvocationDialect,
    context: NativeCompilationContext,
) -> Option<
    Result<
        Vec<crate::native_control_compilation::NativeControlPreparationStep>,
        NativeInstructionPlanUnavailable,
    >,
> {
    if let NativeCompilationGrammar::StringIsAscii(scope) = spec.grammar
        && context.mode == crate::native_compilation::NativeCompilationMode::BytecodeObject
        && crate::native_string_is_compilation::private_original_declines(
            words,
            operand_from,
            scope,
            dialect.tcl_version?,
            spec.operation,
            spec.body,
            context,
        )
    {
        return Some(Ok(Vec::new()));
    }
    if spec.grammar == NativeCompilationGrammar::FormatStringConcat {
        if crate::native_format_compilation::dynamic_format_word_declines(
            words,
            operand_from,
            dialect.tcl_version?,
            context,
            spec.operation,
            spec.body,
        ) {
            return Some(Ok(Vec::new()));
        }
        let selection = crate::native_format_compilation::select_with_values(
            &|index| words.literal(operand_from + index).map(<[u8]>::to_vec),
            words.shapes().get(operand_from..)?,
            dialect.tcl_version?,
            context,
            spec.operation,
        );
        // The attempted hook emits no instruction, literal registration or
        // local reservation before this decline. The GenericPreparation
        // consumer compiles the enclosing ordinary words once afterwards.
        return (selection == NativeCompilationSelection::Generic).then_some(Ok(Vec::new()));
    }
    original_dictionary_preparations(spec, words, operand_from, dialect.tcl_version?, context)
}

/// `error` and `throw` share the Error stack recipe.
fn error_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
) -> Option<Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>> {
    let compiled = match spec.grammar {
        NativeCompilationGrammar::Error => {
            crate::native_error_compilation::compile_native_error(words, operand_from, version)
        }
        NativeCompilationGrammar::Throw => {
            crate::native_error_compilation::compile_native_throw(words, operand_from, version)
        }
        _ => return None,
    };
    Some(
        compiled
            .map(NativeInstructionPlan::Error)
            .map_err(|_| NativeInstructionPlanUnavailable::OperandGeometry),
    )
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
            original_decline_preparations(spec, words, operand_from, dialect, context)
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
            | NativeCompilationGrammar::InfoCoroutine
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
    if let Some(plan) = error_instruction_plan(spec, words, operand_from, version) {
        return plan;
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
            | NativeCompilationGrammar::Constant
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

/// Non-strict `ascii` keeps its scalar form; other classes use the full recipe.
fn string_is_instruction_plan(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    scope: crate::native_scalar_compilation::NativeScalarScope,
    version: tcl_dialect::TclVersion,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    if let Some((message, options)) = crate::native_string_is_compilation::bad_class_syntax_error(
        words,
        operand_from,
        scope,
        version,
    ) {
        return Ok(NativeInstructionPlan::SyntaxError { message, options });
    }
    crate::native_string_is_compilation::instruction(words, operand_from, scope, version)
        .map(NativeInstructionPlan::Scalar)
        .or_else(|| {
            crate::native_string_is_compilation::string_is_instruction(
                words,
                operand_from,
                scope,
                version,
            )
            .map(NativeInstructionPlan::StringIs)
        })
        .ok_or(NativeInstructionPlanUnavailable::OperandGeometry)
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
    if let NativeCompilationGrammar::StringIsAscii(scope) = spec.grammar {
        return Some(string_is_instruction_plan(
            words,
            operand_from,
            scope,
            version,
        ));
    }
    if let Some(plan) =
        selected_string_slice_instruction_plan(spec.grammar, words, operand_from, version)
    {
        return Some(plan);
    }
    if let NativeCompilationGrammar::StringOperand { scope, operation } = spec.grammar {
        return Some(
            crate::native_string_operand_compilation::compile_native_string_operand(
                words,
                operand_from,
                operation,
                scope,
                version,
            )
            .map(NativeInstructionPlan::StringOperand)
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

fn selected_string_slice_instruction_plan(
    grammar: NativeCompilationGrammar,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
) -> Option<Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>> {
    let plan = match grammar {
        NativeCompilationGrammar::StringInsert(scope) => {
            crate::native_string_insert_compilation::compile_native_string_insert(
                words,
                operand_from,
                scope,
                version,
            )
            .map(NativeInstructionPlan::StringInsert)
        }
        NativeCompilationGrammar::StringReplace(scope) => {
            crate::native_string_replace_compilation::compile_native_string_replace(
                words,
                operand_from,
                scope,
                version,
            )
            .map(NativeInstructionPlan::StringReplace)
        }
        NativeCompilationGrammar::StringRange(scope) => {
            crate::native_string_range_compilation::compile_native_string_range(
                words,
                operand_from,
                scope,
                version,
            )
            .map(NativeInstructionPlan::StringRange)
        }
        NativeCompilationGrammar::ListReplacement => {
            crate::native_list_replace_compilation::compile_native_list_replace(
                words,
                operand_from,
                version,
            )
            .map(NativeInstructionPlan::ListReplace)
        }
        _ => return None,
    };
    Some(plan.ok_or(NativeInstructionPlanUnavailable::OperandGeometry))
}

fn selected_dictionary_instruction_plan(
    command: crate::native_dictionary::NativeDictionaryCommand,
    ensemble: bool,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
    context: NativeCompilationContext,
) -> Result<NativeInstructionPlan, NativeInstructionPlanUnavailable> {
    use NativeInstructionPlanUnavailable as Unavailable;
    let from = operand_from + usize::from(ensemble);
    if matches!(
        command,
        crate::native_dictionary::NativeDictionaryCommand::Update
            | crate::native_dictionary::NativeDictionaryCommand::With
    ) {
        return crate::native_dictionary_scope_compilation::compile_native_dictionary_scope(
            command, words, from, version, context,
        )
        .map(NativeInstructionPlan::DictionaryScope)
        .map_err(|_| Unavailable::OperandGeometry);
    }
    if matches!(
        command,
        crate::native_dictionary::NativeDictionaryCommand::For
            | crate::native_dictionary::NativeDictionaryCommand::Map
    ) {
        return crate::native_dictionary_each_compilation::compile_native_dictionary_each(
            words,
            from,
            version,
            context,
            command == crate::native_dictionary::NativeDictionaryCommand::Map,
        )
        .filter(|recipe| {
            matches!(
                recipe.outcome,
                crate::native_control_compilation::NativeControlOutcome::Inline(_)
            )
        })
        .map(NativeInstructionPlan::DictionaryEach)
        .ok_or(Unavailable::OperandGeometry);
    }
    if command == crate::native_dictionary::NativeDictionaryCommand::Create {
        return crate::native_dictionary_compilation::compile_native_dictionary_create(
            words, from, version, context,
        )
        .map(NativeInstructionPlan::DictionaryCreate)
        .map_err(|_| Unavailable::OperandGeometry);
    }
    if matches!(
        command,
        crate::native_dictionary::NativeDictionaryCommand::Set
            | crate::native_dictionary::NativeDictionaryCommand::Unset
            | crate::native_dictionary::NativeDictionaryCommand::Append
            | crate::native_dictionary::NativeDictionaryCommand::Lappend
            | crate::native_dictionary::NativeDictionaryCommand::Incr
    ) {
        return crate::native_dictionary_compilation::compile_native_dictionary_mutation(
            command, words, from, version, context,
        )
        .map_err(|_| Unavailable::OperandGeometry)
        .and_then(|recipe| match recipe.outcome {
            crate::native_control_compilation::NativeControlOutcome::Inline(recipe) => {
                Ok(NativeInstructionPlan::DictionaryMutation(recipe))
            }
            _ => Err(Unavailable::Selection),
        });
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

fn selected_container_and_string_instruction_plan(
    spec: NativeCompilationSpec,
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: tcl_dialect::TclVersion,
    context: NativeCompilationContext,
) -> Option<Result<NativeInstructionPlan, NativeInstructionPlanUnavailable>> {
    use NativeInstructionPlanUnavailable as Unavailable;
    Some(match spec.grammar {
        NativeCompilationGrammar::StringMap(scope) => {
            crate::native_string_map_compilation::instruction(words, operand_from, scope, version)
                .map(NativeInstructionPlan::StringMap)
                .ok_or(Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::FormatStringConcat => {
            if let Some((message, options)) = crate::native_format_compilation::static_syntax_error(
                &|index| words.literal(operand_from + index).map(<[u8]>::to_vec),
                words.shapes().get(operand_from..).unwrap_or_default(),
                version,
                context,
            ) {
                return Some(Ok(NativeInstructionPlan::SyntaxError { message, options }));
            }
            crate::native_format_compilation::instruction(words, operand_from, version, context)
                .map(NativeInstructionPlan::FormatStringConcat)
                .ok_or(Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::Dictionary { command, ensemble } => {
            selected_dictionary_instruction_plan(
                command,
                ensemble,
                words,
                operand_from,
                version,
                context,
            )
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
        NativeCompilationGrammar::Regexp => {
            crate::native_regexp_compilation::instruction(words, operand_from, version)
                .map(NativeInstructionPlan::Regexp)
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
        // TclCompileLsetCmd: every release from 8.4, the variable word first.
        NativeCompilationGrammar::ArityFrom { .. }
            if spec.operation
                == crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ListSet) =>
        {
            crate::native_list_operations_compilation::compile_native_list_operation(
                words,
                operand_from,
                version,
                crate::native_list_operations_compilation::NativeListOperationKind::Set,
            )
            .map(NativeInstructionPlan::ListOperations)
            .map_err(|_| Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::ListInsertion if version >= tcl_dialect::TclVersion::V8_6 => {
            crate::native_list_operations_compilation::compile_native_list_operation(
                words,
                operand_from,
                version,
                crate::native_list_operations_compilation::NativeListOperationKind::Insert,
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
        NativeCompilationGrammar::SubstitutionTemplate => {
            crate::substitution::native_substitution_instruction(words, operand_from, version)
                .map(NativeInstructionPlan::Substitution)
                .ok_or(Unavailable::OperandGeometry)
        }
        NativeCompilationGrammar::ClockReading(reading) => {
            crate::native_compilation::clock_reading_value(
                reading,
                words.literal(operand_from),
                words.shapes().get(operand_from..)?,
                version,
            )
            .map(NativeInstructionPlan::ClockRead)
            .ok_or(Unavailable::Selection)
        }
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
        (NativeCompilationGrammar::Constant, 2) => match target {
            NativeVariableWordOperand::Literal { index: None, .. }
            | NativeVariableWordOperand::DynamicWord => Ok(NativeInstructionPlan::Constant {
                target_word: operand_from,
                target,
                value_word: operand_from + 1,
            }),
            _ => Err(Unavailable::Selection),
        },
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
mod tests;

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
