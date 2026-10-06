// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime expression preparation from actual fixed-function evidence.

use crate::native_compilation::NativeMathFunctionResolution;
use std::sync::Arc;
use tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite;
use tcl_syntax::expr::ExprNode;
pub use tcl_syntax::expr::jim_function_tree::NativeFunctionTree as RuntimeExpressionPreparation;
use tcl_syntax::expr::parser::{ExprParseContext, NativeExprSyntax};

/// Successful preparation of these exact expression bytes in an observed engine.
/// Reached math calls alone cannot establish this proof: fixed-table engines
/// inspect functions in runtime-lazy branches before any substitutions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PreparedExpressionWitness {
    source: Arc<str>,
    context: ExprParseContext,
    tree: ExprNode,
    fixed_functions: Option<NativeMathFunctionPrerequisite>,
    compiled_scripts: Vec<tcl_lexer::Span>,
}

/// Whether the actual expression compiler precomputes constant operator trees.
/// This selects a native instruction recipe; it says nothing about the current
/// internal representation of any reused literal-pool result object.
#[must_use]
pub const fn native_expression_constant_pooling(
    native: tcl_syntax::expr::parser::NativeExprSyntax,
) -> Option<bool> {
    use tcl_syntax::expr::parser::NativeExprSyntax;
    match native {
        NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4) | NativeExprSyntax::Jim084 => {
            Some(false)
        }
        NativeExprSyntax::Tcl(_) => Some(true),
        NativeExprSyntax::Unknown => None,
    }
}

/// Selected result instruction family, independent of mathematical contents.
/// A pooled value needs actual current pool representation evidence before it
/// can establish a native numeric object at a later source read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExpressionResultRecipe {
    /// C Tcl emits the final numeric conversion attempt on a selected operand.
    NormalizeOperand,
    /// The original expression executes a numeric-producing operator.
    NumericOperation,
    /// A constant-folded result or Jim literal can return a reused pool object.
    PooledValue,
    /// The result protocol or operator topology is not established.
    Unknown,
}

/// Selected runtime operator producing a numeric intrep on normal completion.
/// This proves no concrete value, operand effects, object identity or freshness.
/// Boolean branch results need a separate literal-pool representation proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeExpressionNumericResultProduction {
    native: tcl_syntax::expr::parser::NativeExprSyntax,
    result_type: crate::TclType,
    normalises_string: bool,
}

impl NativeExpressionNumericResultProduction {
    /// The actual runtime recipe resets the result string representation.
    /// This supports sharing disjointness with the native numeric formatter's
    /// output language, independently of current intrep, value and rewrite proof.
    #[must_use]
    pub const fn normalises_result_string(self) -> bool {
        self.normalises_string
    }

    /// Conditional normal result category of this selected runtime operator.
    /// This grants no value, object identity, conversion or rewrite permission.
    #[must_use]
    pub const fn result_type(self) -> crate::TclType {
        self.result_type
    }
}

/// Native operand-cache difference between numeric equality's string fallback
/// and string equality. Value equivalence alone does not close this obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEqualityFallbackProtocol {
    /// Both operations retain an already numeric operand's intrep.
    PreservesNumericRepresentation,
    /// Jim's numeric fallback installs its UTF-8 string cache and retires the
    /// numeric intrep, while string equality compares bytes without that change.
    ConvertsNumericToString,
}

/// Selected numeric-comparison string fallback. This is an operand-cache
/// protocol only; successful values, reached reads and result objects remain
/// separate. Jim's Eq/Ne and ordered comparison fallback shares string coercion.
#[must_use]
pub const fn native_numeric_comparison_fallback(
    native: tcl_syntax::expr::parser::NativeExprSyntax,
) -> Option<NativeEqualityFallbackProtocol> {
    use tcl_syntax::expr::parser::NativeExprSyntax;
    match native {
        NativeExprSyntax::Tcl(_) => {
            Some(NativeEqualityFallbackProtocol::PreservesNumericRepresentation)
        }
        NativeExprSyntax::Jim084 => Some(NativeEqualityFallbackProtocol::ConvertsNumericToString),
        NativeExprSyntax::Unknown => None,
    }
}

/// Original literal conversion retained when arithmetic becomes an increment.
/// Non-unit amounts must retain expression preparation: C Tcl's immediate
/// increment compiler can otherwise leave a shared literal object unconverted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeIncrementAmountConversion<'a> {
    /// The shared native unit constants need no additional literal conversion.
    Unit(i64),
    /// Normalize this exact original literal through the selected expression.
    OriginalLiteral(&'a str),
    /// Normalize the original literal before negating it, preserving its pool
    /// object independently of the resulting increment amount.
    NegatedOriginalLiteral(&'a str),
}

/// Conditional arithmetic schedule; physical reads, stores, observers and
/// prospective native handlers must still be proved by their source owners.
pub struct NativeIncrementExpressionSchedule<'a> {
    operand: &'a ExprNode,
    amount: i64,
    conversion: NativeIncrementAmountConversion<'a>,
    operand_normalisation: Option<&'a str>,
}

impl<'a> NativeIncrementExpressionSchedule<'a> {
    /// Original variable occurrence, rather than a base-name dependency label.
    #[must_use]
    pub const fn operand(&self) -> &'a ExprNode {
        self.operand
    }
    /// Signed wide amount. Consumers must reject overflow of the actual input.
    #[must_use]
    pub const fn amount(&self) -> i64 {
        self.amount
    }
    /// Original operand text for the audited C bignum unary-normalisation
    /// recipe. This keeps shared input conversion executed; it grants no
    /// editable source, read success, object identity or handler dependency.
    #[must_use]
    pub const fn operand_normalisation(&self) -> Option<&'a str> {
        self.operand_normalisation
    }
    /// Required original literal normalization, separate from value arithmetic.
    #[must_use]
    pub const fn conversion(&self) -> NativeIncrementAmountConversion<'a> {
        self.conversion
    }
}

pub(crate) fn checked_numeric_reentry_is_idempotent(
    outer: &ExprNode,
    outer_context: &ExprParseContext,
    inner: &ExprNode,
    inner_context: &ExprParseContext,
) -> bool {
    if outer_context != inner_context
        || !matches!(outer_context.native_syntax, NativeExprSyntax::Tcl(_))
        || !matches!(outer, ExprNode::Command { .. })
        || !selected_numeric_result_production(inner, inner_context.native_syntax)
            .is_some_and(NativeExpressionNumericResultProduction::normalises_result_string)
    {
        return false;
    }
    let mut pending = vec![inner];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            _ => return false,
        }
    }
    true
}

pub(crate) fn selected_numeric_result_production(
    tree: &ExprNode,
    native: NativeExprSyntax,
) -> Option<NativeExpressionNumericResultProduction> {
    use tcl_syntax::expr::{BinOp, UnaryOp};
    if selected_numeric_result_recipe(tree, native)
        != NativeExpressionResultRecipe::NumericOperation
    {
        return None;
    }
    let numeric = match tree {
        ExprNode::Unary { op, .. } => {
            matches!(op, UnaryOp::Pos | UnaryOp::Neg | UnaryOp::BitNot)
        }
        ExprNode::Binary { op, .. } => matches!(
            op,
            BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::Pow
                | BinOp::LShift
                | BinOp::RShift
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
        ),
        _ => false,
    };
    numeric.then_some(NativeExpressionNumericResultProduction {
        native,
        // These selected setters/new-object paths reset the string. Other
        // numeric operators can forward an operand or return a pool entry.
        normalises_string: matches!(
            *tree,
            ExprNode::Unary {
                op: UnaryOp::BitNot,
                ..
            } | ExprNode::Binary {
                op: BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div,
                ..
            }
        ),
        result_type: match tree {
            ExprNode::Unary {
                op: UnaryOp::BitNot,
                ..
            }
            | ExprNode::Binary {
                op:
                    BinOp::Mod
                    | BinOp::LShift
                    | BinOp::RShift
                    | BinOp::BitAnd
                    | BinOp::BitOr
                    | BinOp::BitXor,
                ..
            } => crate::TclType::Int,
            _ => crate::TclType::Numeric,
        },
    })
}

impl PreparedExpressionWitness {
    /// Closed single-read integer-addition candidate with its literal-pool
    /// obligation. A selected preparation grants no executable store rewrite.
    #[must_use]
    pub fn increment_expression_schedule(&self) -> Option<NativeIncrementExpressionSchedule<'_>> {
        use tcl_syntax::expr::BinOp;
        self.normal_numeric_result_production()?;
        let ExprNode::Binary { op, left, right } = &self.tree else {
            return None;
        };
        let (operand, literal) = match op {
            BinOp::Add | BinOp::Sub if matches!(left.as_ref(), ExprNode::Var { .. }) => {
                (left.as_ref(), right.as_ref())
            }
            BinOp::Add if matches!(right.as_ref(), ExprNode::Var { .. }) => {
                (right.as_ref(), left.as_ref())
            }
            _ => return None,
        };
        let ExprNode::Literal { text, .. } = literal else {
            return None;
        };
        let tcl_syntax::number::Number::Int(value) = tcl_syntax::number::parse_whole_with(
            text,
            tcl_syntax::number::ParseFlags::for_syntax(self.context.lexer_grammar.numbers),
        )?
        else {
            return None;
        };
        let amount = if *op == BinOp::Sub {
            value.checked_neg()?
        } else {
            value
        };
        if amount == 0 {
            return None;
        }
        let conversion = if text == "1" {
            NativeIncrementAmountConversion::Unit(amount)
        } else if *op == BinOp::Sub {
            NativeIncrementAmountConversion::NegatedOriginalLiteral(text)
        } else {
            NativeIncrementAmountConversion::OriginalLiteral(text)
        };
        Some(NativeIncrementExpressionSchedule {
            operand,
            amount,
            conversion,
            operand_normalisation: match (self.context.native_syntax, operand) {
                (
                    tcl_syntax::expr::parser::NativeExprSyntax::Tcl(
                        tcl_dialect::TclVersion::V8_5
                        | tcl_dialect::TclVersion::V8_6
                        | tcl_dialect::TclVersion::V9_0
                        | tcl_dialect::TclVersion::V9_1,
                    ),
                    ExprNode::Var { text, .. },
                ) => Some(text),
                _ => None,
            },
        })
    }

    /// Selected equality fallback cache protocol. The consumer must separately
    /// prove original operands, a successful numeric read, a fixed nonnumeric
    /// ASCII counterpart and equal normal Boolean result protocols.
    #[must_use]
    pub fn equality_fallback_protocol(&self) -> Option<NativeEqualityFallbackProtocol> {
        use tcl_syntax::expr::BinOp;
        if !matches!(
            self.tree,
            ExprNode::Binary {
                op: BinOp::Eq | BinOp::Ne,
                ..
            }
        ) {
            return None;
        }
        native_numeric_comparison_fallback(self.context.native_syntax)
    }
}

impl PreparedExpressionWitness {
    /// The actual native tree, including Jim's shared argument-stack grouping.
    #[must_use]
    pub const fn tree(&self) -> &ExprNode {
        &self.tree
    }

    /// Retain the selected compiler's result recipe without treating a
    /// mathematical constant as a fresh native numeric object. This does not
    /// prove operand conversions, result bytes or pool representation.
    #[must_use]
    pub fn numeric_result_recipe(&self) -> NativeExpressionResultRecipe {
        selected_numeric_result_recipe(&self.tree, self.context.native_syntax)
    }

    /// C expression normalization of a fresh arithmetic result is idempotent
    /// on the inner normal path. Inner errors and operand evaluation remain
    /// obligations of the original nested invocation. This supplies neither
    /// total evaluation nor a concrete numeric value.
    #[must_use]
    pub fn numeric_reentry_is_idempotent(&self, inner: &Self) -> bool {
        inner.compiled_scripts.is_empty()
            && checked_numeric_reentry_is_idempotent(
                &self.tree,
                &self.context,
                &inner.tree,
                &inner.context,
            )
    }

    /// Numeric representation of an actually completed runtime arithmetic
    /// operator. Preparation alone does not prove that its normal path occurs.
    #[must_use]
    pub fn normal_numeric_result_production(
        &self,
    ) -> Option<NativeExpressionNumericResultProduction> {
        selected_numeric_result_production(&self.tree, self.context.native_syntax)
    }

    /// Exact immutable expression bytes whose preparation succeeded.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Complete preparation axes, independently of assistance profile names.
    #[must_use]
    pub const fn context(&self) -> &ExprParseContext {
        &self.context
    }

    /// Actual interpreter/table prerequisite, separate from reached call bindings.
    #[must_use]
    pub const fn fixed_functions(&self) -> Option<&NativeMathFunctionPrerequisite> {
        self.fixed_functions.as_ref()
    }

    /// Ordered nested script visits closed by the native compiler owner.
    /// Spans are expression-relative authored command-substitution extents,
    /// including their brackets. The enclosing compiler proof retains the
    /// actual snapshot, selected recipes and dependencies for these visits.
    #[must_use]
    pub fn compiled_scripts(&self) -> &[tcl_lexer::Span] {
        &self.compiled_scripts
    }

    /// Whether the same entry proof applies to the proposed expression and engine.
    #[must_use]
    pub fn applies_to(
        &self,
        source: &str,
        context: &ExprParseContext,
        fixed_functions: Option<&NativeMathFunctionPrerequisite>,
    ) -> bool {
        self.compiled_scripts.is_empty()
            && self.source() == source
            && self.context == *context
            && self
                .fixed_functions
                .as_ref()
                .is_none_or(|required| fixed_functions == Some(required))
    }
}

/// Maximal constant operator trees precomputed by the selected compiler.
/// The inventory is linear in AST size and refers only to the original tree;
/// callers still need the selected evaluator to obtain their exact result bytes.
#[must_use]
pub fn native_expression_pooled_subtrees<Text: tcl_syntax::expr::ExprText>(
    tree: &ExprNode<Text>,
    native: tcl_syntax::expr::parser::NativeExprSyntax,
) -> Option<Vec<&ExprNode<Text>>> {
    if !native_expression_constant_pooling(native)? {
        return Some(Vec::new());
    }
    let constants = constant_expression_nodes(tree);
    let mut pending = vec![tree];
    let mut result = Vec::new();
    while let Some(node) = pending.pop() {
        if matches!(
            node,
            ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. }
        ) && constants.get(&std::ptr::from_ref(node)) == Some(&true)
        {
            result.push(node);
        } else {
            pending.extend(expression_children(node));
        }
    }
    Some(result)
}

fn expression_children<Text>(node: &ExprNode<Text>) -> Vec<&ExprNode<Text>> {
    match node {
        ExprNode::Unary { operand, .. } => vec![operand],
        ExprNode::Binary { left, right, .. } => vec![left, right],
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => vec![condition, true_branch, false_branch],
        ExprNode::Call { args, .. } => args.iter().collect(),
        _ => Vec::new(),
    }
}

/// Shared selected native result recipe, without actual preparation authority.
pub(crate) fn selected_numeric_result_recipe(
    tree: &ExprNode,
    native: tcl_syntax::expr::parser::NativeExprSyntax,
) -> NativeExpressionResultRecipe {
    use NativeExpressionResultRecipe as Recipe;
    use tcl_syntax::expr::parser::NativeExprSyntax;
    if native == NativeExprSyntax::Unknown {
        return Recipe::Unknown;
    }
    match tree {
        ExprNode::Literal { .. } | ExprNode::Var { .. } => match native {
            NativeExprSyntax::Tcl(_) => Recipe::NormalizeOperand,
            _ => Recipe::PooledValue,
        },
        ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. } => {
            if native_expression_constant_pooling(native) == Some(true)
                && constant_operator_tree(tree)
            {
                Recipe::PooledValue
            } else if matches!(tree, ExprNode::Ternary { .. }) {
                match native {
                    NativeExprSyntax::Tcl(_) => Recipe::NormalizeOperand,
                    _ => Recipe::PooledValue,
                }
            } else {
                Recipe::NumericOperation
            }
        }
        _ => Recipe::Unknown,
    }
}

fn constant_expression_nodes<Text: tcl_syntax::expr::ExprText>(
    tree: &ExprNode<Text>,
) -> std::collections::HashMap<*const ExprNode<Text>, bool> {
    let mut pending = vec![(tree, false)];
    // Scratch AST addresses identify traversal nodes, never runtime objects.
    let mut constants = std::collections::HashMap::new();
    while let Some((node, complete)) = pending.pop() {
        let children = expression_children(node);
        if !complete {
            pending.push((node, true));
            pending.extend(children.into_iter().map(|child| (child, false)));
            continue;
        }
        let constant = match node {
            ExprNode::Literal { .. } => true,
            ExprNode::String { text, .. } => text
                .try_text()
                .and_then(tcl_syntax::expr::fixed_string_operand)
                .is_some(),
            ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. } => children
                .iter()
                .all(|child| constants.get(&std::ptr::from_ref(*child)) == Some(&true)),
            _ => false,
        };
        constants.insert(std::ptr::from_ref(node), constant);
    }
    constants
}

fn constant_operator_tree(tree: &ExprNode) -> bool {
    constant_expression_nodes(tree).get(&std::ptr::from_ref(tree)) == Some(&true)
}

/// Preparation is separate from expression execution and its reached read/call ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionPreparationProof {
    /// Entry validation succeeded; evaluate this witness's prepared tree.
    Prepared(Box<PreparedExpressionWitness>),
    /// A presented native rejection occurs before evaluation starts.
    Rejected(tcl_syntax::expr::parser::NativeExprSyntaxDiagnostic),
    /// Grammar, registration or nested compiler obligations are unresolved.
    Unknown,
}

/// Prepare exact bytes using an actual native registration prerequisite.
/// This does not validate nested C 8.4 script compiler visits: such expressions
/// retain an obligation until their compiler traversal owner closes it.
#[must_use]
pub fn prepare_expression_witness(
    source: &str,
    context: &ExprParseContext,
    fixed_functions: Option<&NativeMathFunctionPrerequisite>,
) -> ExpressionPreparationProof {
    prepare_expression_witness_with_script_compiler(source, context, fixed_functions, |_| {
        NativeExpressionScriptPreparation::Unknown
    })
}

/// Result of one actual nested-script compiler visit. Syntax or catalogue
/// inspection alone cannot establish a completed compiler traversal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionScriptPreparation {
    /// The actual compiler owner closed this script's complete traversal.
    /// Its enclosing source proof retains the snapshot, recipes and guards.
    Compiled,
    /// The compiler stops at this visit with a retained native rejection.
    Rejected(Box<crate::native_compilation::NativeCompilationFailure>),
    /// A compiler lookup, recipe, dependency or completion remains unresolved.
    Unknown,
}

/// Prepare an expression with the native compiler owner's ordered script visits.
/// The callback receives expression-relative command-substitution spans,
/// including their brackets, exactly as emitted by `compiler_steps`.
/// A completed witness with scripts remains dependent on its enclosing compiler
/// carrier; `applies_to` cannot reuse it as a standalone table-only proof.
#[must_use]
pub fn prepare_expression_witness_with_script_compiler(
    source: &str,
    context: &ExprParseContext,
    fixed_functions: Option<&NativeMathFunctionPrerequisite>,
    mut compile_script: impl FnMut(tcl_lexer::Span) -> NativeExpressionScriptPreparation,
) -> ExpressionPreparationProof {
    use tcl_syntax::expr::parser::{CheckedExprParse, NativeExprSyntax};
    let mut used_table = false;
    let mut compiled_scripts = Vec::new();
    let mut lookup = |name: &str| {
        used_table = true;
        lookup_function(fixed_functions, name)
    };
    let tree = if requires_fixed_function_preparation(context) {
        match prepare_fixed_function_expression(source, context, &mut lookup) {
            RuntimeExpressionPreparation::Parsed(tree) => tree,
            RuntimeExpressionPreparation::Rejected(error) => {
                return ExpressionPreparationProof::Rejected(error);
            }
            RuntimeExpressionPreparation::Unknown => return ExpressionPreparationProof::Unknown,
        }
    } else {
        let tree = match tcl_syntax::expr::parser::parse_expr_checked_with_context(source, context)
        {
            CheckedExprParse::Parsed(tree) => tree,
            CheckedExprParse::ProvedSyntaxFailure(error) => {
                return error
                    .native_diagnostic_with_context(source, context)
                    .map_or(
                        ExpressionPreparationProof::Unknown,
                        ExpressionPreparationProof::Rejected,
                    );
            }
            CheckedExprParse::Unsupported(_) => return ExpressionPreparationProof::Unknown,
        };
        if context.native_syntax == NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4)
            && let Some(failure) = fixed_compiler_failure(
                source,
                context,
                &mut lookup,
                &mut compile_script,
                &mut compiled_scripts,
            )
        {
            return failure;
        }
        tree
    };
    ExpressionPreparationProof::Prepared(Box::new(PreparedExpressionWitness {
        source: Arc::from(source),
        context: *context,
        tree,
        fixed_functions: used_table.then(|| fixed_functions.cloned()).flatten(),
        compiled_scripts,
    }))
}

fn lookup_function(
    prerequisite: Option<&NativeMathFunctionPrerequisite>,
    name: &str,
) -> NativeMathFunctionResolution {
    use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Lookup;
    match prerequisite.map(|proof| proof.table.lookup(name)) {
        Some(Lookup::Present(row)) => row
            .arity
            .map_or(NativeMathFunctionResolution::Unknown, |arity| {
                NativeMathFunctionResolution::Known { arity }
            }),
        Some(Lookup::Absent) => NativeMathFunctionResolution::Absent,
        Some(Lookup::Unknown) | None => NativeMathFunctionResolution::Unknown,
    }
}

fn fixed_compiler_failure(
    source: &str,
    context: &ExprParseContext,
    lookup: &mut impl FnMut(&str) -> NativeMathFunctionResolution,
    compile_script: &mut impl FnMut(tcl_lexer::Span) -> NativeExpressionScriptPreparation,
    compiled_scripts: &mut Vec<tcl_lexer::Span>,
) -> Option<ExpressionPreparationProof> {
    use crate::native_compilation::{
        NativeCompiledExpressionErrorContext, NativeCompiledExpressionOperand,
        NativeExpressionCompilerStep,
    };
    let operand = NativeCompiledExpressionOperand {
        argument: 0,
        error_context: NativeCompiledExpressionErrorContext::None,
    };
    for step in operand.compiler_steps(source, context, lookup) {
        match step {
            NativeExpressionCompilerStep::Failure(error) => {
                return Some(present_compiler_rejection(error));
            }
            NativeExpressionCompilerStep::Unknown => {
                return Some(ExpressionPreparationProof::Unknown);
            }
            NativeExpressionCompilerStep::Script(span) => match compile_script(span) {
                NativeExpressionScriptPreparation::Compiled => compiled_scripts.push(span),
                NativeExpressionScriptPreparation::Rejected(error) => {
                    return Some(present_compiler_rejection(*error));
                }
                NativeExpressionScriptPreparation::Unknown => {
                    return Some(ExpressionPreparationProof::Unknown);
                }
            },
        }
    }
    None
}

fn present_compiler_rejection(
    error: crate::native_compilation::NativeCompilationFailure,
) -> ExpressionPreparationProof {
    let Some(message) = error.message else {
        return ExpressionPreparationProof::Unknown;
    };
    ExpressionPreparationProof::Rejected(tcl_syntax::expr::parser::NativeExprSyntaxDiagnostic {
        message,
        error_code: error.error_code,
    })
}

/// Whether the selected native policy requires eager fixed-function preparation.
/// The lexical grammar and assistance profile cannot substitute for this axis.
#[must_use]
pub const fn requires_fixed_function_preparation(context: &ExprParseContext) -> bool {
    matches!(
        context.native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Jim084
    )
}

/// Whether warm expression preparation depends on the actual fixed table.
/// This is independent of whether Jim's reassociated tree is required.
#[must_use]
pub const fn preparation_needs_actual_table(context: &ExprParseContext) -> bool {
    matches!(
        context.native_syntax,
        tcl_syntax::expr::parser::NativeExprSyntax::Jim084
            | tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4)
    )
}

/// Resolve Jim's whole-expression function pass and native term-stack tree.
/// This is a runtime preparation boundary, independently of C Tcl 8.4's
/// script compiler traversal. No command catalogue establishes registration
/// evidence. Every execution, including a warm AST-cache hit, must retain or
/// revalidate the supplied actual function-table identity and generation.
/// Execute the returned tree rather than a separately parsed conventional AST.
#[must_use]
pub fn prepare_fixed_function_expression(
    source: &str,
    context: &ExprParseContext,
    mut lookup: impl FnMut(&str) -> NativeMathFunctionResolution,
) -> RuntimeExpressionPreparation {
    use tcl_syntax::expr::jim_function_tree::{NativeFunctionArity, prepare_jim_function_tree};
    prepare_jim_function_tree(source, context, |name| match lookup(name) {
        NativeMathFunctionResolution::Known { arity } => NativeFunctionArity::Known(arity),
        NativeMathFunctionResolution::Absent => NativeFunctionArity::Absent,
        NativeMathFunctionResolution::Unknown => NativeFunctionArity::Unknown,
    })
}

/// Resolve the same actual fixed-function table over original byte source.
/// Byte leaves remain owned by the native tree; no textual source replacement
/// is used for function resolution or substitution.
#[must_use]
pub fn prepare_fixed_function_expression_bytes(
    source: &[u8],
    context: &ExprParseContext,
    mut lookup: impl FnMut(&str) -> NativeMathFunctionResolution,
) -> RuntimeExpressionPreparation<Vec<u8>> {
    use tcl_syntax::expr::jim_function_tree::{
        NativeFunctionArity, prepare_jim_function_tree_bytes,
    };
    prepare_jim_function_tree_bytes(source, context, |name| match lookup(name) {
        NativeMathFunctionResolution::Known { arity } => NativeFunctionArity::Known(arity),
        NativeMathFunctionResolution::Absent => NativeFunctionArity::Absent,
        NativeMathFunctionResolution::Unknown => NativeFunctionArity::Unknown,
    })
}
/// Prepare the fixed native tree together with its original primary action.
/// Function registration, lexical failure and term ownership come from the
/// same preparation pass; catalogue metadata cannot issue these facts.
#[must_use]
pub fn prepare_fixed_function_expression_bytes_with_preparation(
    source: &[u8],
    context: &ExprParseContext,
    mut lookup: impl FnMut(&str) -> NativeMathFunctionResolution,
) -> tcl_syntax::expr::jim_function_tree::PreparedNativeFunctionTree<Vec<u8>> {
    use tcl_syntax::expr::jim_function_tree::{
        NativeFunctionArity, prepare_jim_function_tree_bytes_with_preparation,
    };
    prepare_jim_function_tree_bytes_with_preparation(source, context, |name| match lookup(name) {
        NativeMathFunctionResolution::Known { arity } => NativeFunctionArity::Known(arity),
        NativeMathFunctionResolution::Absent => NativeFunctionArity::Absent,
        NativeMathFunctionResolution::Unknown => NativeFunctionArity::Unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_runtime_api::native_compilation::{
        NativeInterpreterIdentity, NativeMathFunctionBinding, NativeMathFunctionTable,
    };

    #[test]
    fn native_nested_expression_results_and_inner_error_metadata_match() {
        for rows in [
            include_str!("../tests/data/native_nested_expression_normalization/8.4.txt"),
            include_str!("../tests/data/native_nested_expression_normalization/8.5.txt"),
            include_str!("../tests/data/native_nested_expression_normalization/8.6.txt"),
            include_str!("../tests/data/native_nested_expression_normalization/9.0.txt"),
            include_str!("../tests/data/native_nested_expression_normalization/9.1.txt"),
            include_str!("../tests/data/native_nested_expression_normalization/jim.txt"),
        ] {
            let outcomes = rows.split("ROW ").skip(1).collect::<Vec<_>>();
            assert_eq!(outcomes.len(), 10);
            for pair in outcomes.chunks_exact(2) {
                assert_eq!(
                    pair[0].replace("original", "procedure"),
                    pair[1].replace("rewritten", "procedure")
                );
            }
        }
    }

    #[test]
    fn nested_numeric_reentry_preserves_conditional_results_without_totality() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let parser = context(profile);
            let ExpressionPreparationProof::Prepared(outer) =
                prepare_expression_witness_with_script_compiler(
                    "[expr {$x+1}]",
                    &parser,
                    None,
                    |_| NativeExpressionScriptPreparation::Compiled,
                )
            else {
                panic!("outer {profile}");
            };
            for (source, allowed) in [
                ("$x+1", true),
                ("$x*2", true),
                ("$x", false),
                (r#""$x""#, false),
                ("$x+[other]", false),
                ("sqrt($x)+1", false),
            ] {
                let inner =
                    prepare_expression_witness_with_script_compiler(source, &parser, None, |_| {
                        NativeExpressionScriptPreparation::Compiled
                    });
                let accepted = match inner {
                    ExpressionPreparationProof::Prepared(inner) => {
                        outer.numeric_reentry_is_idempotent(&inner)
                    }
                    _ => false,
                };
                assert_eq!(accepted, allowed, "{profile}: {source}");
            }
        }
    }

    fn context(profile: &str) -> ExprParseContext {
        ExprParseContext::for_profile(
            crate::model::ingress::static_context_for(profile)
                .commands()
                .profile()
                .unwrap(),
        )
    }

    #[test]
    fn increment_operand_normalisation_uses_the_selected_compiler_recipe() {
        for (profile, normalises) in [
            ("tcl8.4", false),
            ("tcl8.5", true),
            ("tcl8.6", true),
            ("tcl9.0", true),
            ("tcl9.1", true),
            ("jim", false),
        ] {
            let ExpressionPreparationProof::Prepared(witness) =
                prepare_expression_witness("${x}+1", &context(profile), None)
            else {
                panic!("actual single-read native preparation: {profile}");
            };
            let schedule = witness
                .increment_expression_schedule()
                .expect("native addition recipe");
            assert_eq!(
                schedule.operand_normalisation(),
                normalises.then_some("${x}"),
                "{profile}"
            );
        }
    }

    #[test]
    fn selected_constant_pooling_and_result_recipes_follow_native_compilers() {
        use NativeExpressionResultRecipe as Recipe;
        for (profile, pooling, arithmetic) in [
            ("tcl8.4", false, Recipe::NumericOperation),
            ("tcl8.5", true, Recipe::PooledValue),
            ("tcl8.6", true, Recipe::PooledValue),
            ("tcl9.0", true, Recipe::PooledValue),
            ("tcl9.1", true, Recipe::PooledValue),
            ("jim", false, Recipe::NumericOperation),
        ] {
            let parser = context(profile);
            assert_eq!(
                native_expression_constant_pooling(parser.native_syntax),
                Some(pooling)
            );
            for (source, expected) in [
                ("3+4", arithmetic),
                ("$x+1", Recipe::NumericOperation),
                (
                    "3",
                    if profile == "jim" {
                        Recipe::PooledValue
                    } else {
                        Recipe::NormalizeOperand
                    },
                ),
            ] {
                let ExpressionPreparationProof::Prepared(witness) =
                    prepare_expression_witness(source, &parser, None)
                else {
                    panic!("closed literal/operator preparation: {profile}");
                };
                assert_eq!(
                    witness.numeric_result_recipe(),
                    expected,
                    "{profile}: {source}"
                );
            }
        }
        assert_eq!(
            native_expression_constant_pooling(tcl_syntax::expr::parser::NativeExprSyntax::Unknown),
            None
        );
    }

    #[test]
    fn normal_numeric_production_excludes_forwarding_and_pooled_boolean_results() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let parser = context(profile);
            for (source, numeric) in [
                ("$x+0", true),
                ("+$x", true),
                ("$x<<1", true),
                ("$x||1", false),
                ("$x==7", false),
                ("!$x", false),
                ("$x", false),
                ("$x?1:2", false),
                (
                    "3+4",
                    native_expression_constant_pooling(parser.native_syntax) == Some(false),
                ),
            ] {
                let ExpressionPreparationProof::Prepared(witness) =
                    prepare_expression_witness(source, &parser, None)
                else {
                    panic!("closed native preparation: {profile}: {source}");
                };
                assert_eq!(
                    witness.normal_numeric_result_production().is_some(),
                    numeric,
                    "{profile}: {source}"
                );
                if source == "$x<<1" {
                    assert_eq!(
                        witness
                            .normal_numeric_result_production()
                            .unwrap()
                            .result_type(),
                        crate::TclType::Int,
                        "{profile}: {source}"
                    );
                }
            }
        }
    }

    #[test]
    fn result_string_normalisation_is_separate_from_numeric_category() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let parser = context(profile);
            for (source, normalises) in [
                ("$x+0", true),
                ("$x-0", true),
                ("$x*1", true),
                ("$x/1", true),
                ("~$x", true),
                ("+$x", false),
                ("-$x", false),
                ("$x**1", false),
                ("$x%1", false),
                ("$x<<1", false),
            ] {
                if profile == "tcl8.4" && source == "$x**1" {
                    let unsupported = prepare_expression_witness(source, &parser, None);
                    assert!(
                        !matches!(unsupported, ExpressionPreparationProof::Prepared(_)),
                        "C8.4 exponentiation cannot establish native preparation"
                    );
                    continue;
                }
                let ExpressionPreparationProof::Prepared(witness) =
                    prepare_expression_witness(source, &parser, None)
                else {
                    panic!("closed native preparation: {profile}: {source}")
                };
                assert_eq!(
                    witness
                        .normal_numeric_result_production()
                        .unwrap()
                        .normalises_result_string(),
                    normalises,
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn equality_fallback_keeps_the_selected_cache_obligation() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let ExpressionPreparationProof::Prepared(witness) =
                prepare_expression_witness("$x == \"hello\"", &context(profile), None)
            else {
                panic!("original comparison must prepare");
            };
            assert_eq!(
                witness.equality_fallback_protocol(),
                Some(if profile == "jim" {
                    NativeEqualityFallbackProtocol::ConvertsNumericToString
                } else {
                    NativeEqualityFallbackProtocol::PreservesNumericRepresentation
                })
            );
        }
    }

    fn table() -> NativeMathFunctionPrerequisite {
        NativeMathFunctionPrerequisite {
            interpreter: NativeInterpreterIdentity {
                owner: 7,
                interpreter: 2,
            },
            table: NativeMathFunctionTable {
                closed: true,
                generation: 3,
                functions: [("abs", 1), ("pow", 2)]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (name, arity))| NativeMathFunctionBinding {
                        name: name.into(),
                        token: u64::try_from(index).unwrap(),
                        implementation_generation: 1,
                        registry_identity: Some(name.into()),
                        arity: Some(arity),
                    })
                    .collect(),
            },
        }
    }

    #[test]
    fn skipped_fixed_function_still_requires_expression_entry_validation() {
        let table = table();
        for profile in ["tcl8.4", "jim"] {
            assert!(matches!(
                prepare_expression_witness(
                    "0 && future_function(1)",
                    &context(profile),
                    Some(&table)
                ),
                ExpressionPreparationProof::Rejected(_)
            ));
            assert!(matches!(
                prepare_expression_witness("0 && abs(1)", &context(profile), None),
                ExpressionPreparationProof::Unknown
            ));
        }
        assert!(matches!(
            prepare_expression_witness("0 && future_function(1)", &context("tcl8.6"), None),
            ExpressionPreparationProof::Prepared(_)
        ));
    }

    #[test]
    fn prepared_jim_tree_and_table_incarnation_are_retained() {
        let mut table = table();
        let context = context("jim");
        let source = "abs(pow(1),2)";
        let ExpressionPreparationProof::Prepared(witness) =
            prepare_expression_witness(source, &context, Some(&table))
        else {
            panic!("native Jim shared argument stack must prepare");
        };
        let ExprNode::Call { args, .. } = witness.tree() else {
            panic!("outer abs");
        };
        assert_eq!(args.len(), 1);
        assert!(matches!(&args[0], ExprNode::Call { args, .. } if args.len() == 2));
        assert!(witness.applies_to(source, &context, Some(&table)));
        table.table.generation += 1;
        assert!(!witness.applies_to(source, &context, Some(&table)));
        assert!(!witness.applies_to("abs(pow(1,2))", &context, witness.fixed_functions()));
    }

    #[test]
    fn unclosed_native_table_and_nested_compiler_obligations_do_not_prepare() {
        let mut table = table();
        table.table.closed = false;
        assert!(matches!(
            prepare_expression_witness("0 && future_function(1)", &context("jim"), Some(&table)),
            ExpressionPreparationProof::Unknown
        ));
        assert!(matches!(
            prepare_expression_witness("abs([opaque])", &context("tcl8.4"), Some(&table)),
            ExpressionPreparationProof::Unknown
        ));
    }
    #[test]
    fn nested_compiler_visits_require_each_actual_script_closure_in_order() {
        let source = "[first] + [second]";
        let parser = context("tcl8.4");
        let mut visited = Vec::new();
        let proof =
            prepare_expression_witness_with_script_compiler(source, &parser, None, |span| {
                visited.push(source[span.as_range()].to_owned());
                NativeExpressionScriptPreparation::Compiled
            });
        let ExpressionPreparationProof::Prepared(witness) = proof else {
            panic!("closed compiler visits");
        };
        assert_eq!(visited, ["[first]", "[second]"]);
        assert_eq!(witness.compiled_scripts().len(), 2);
        assert!(!witness.applies_to(source, &parser, None));
        assert!(matches!(
            prepare_expression_witness(source, &parser, None),
            ExpressionPreparationProof::Unknown
        ));
        let mut visited = Vec::new();
        let proof =
            prepare_expression_witness_with_script_compiler(source, &parser, None, |span| {
                visited.push(span);
                if visited.len() == 1 {
                    NativeExpressionScriptPreparation::Compiled
                } else {
                    NativeExpressionScriptPreparation::Unknown
                }
            });
        assert_eq!(visited.len(), 2);
        assert!(matches!(proof, ExpressionPreparationProof::Unknown));
    }

    #[test]
    fn nested_visit_closure_does_not_hide_later_fixed_function_rejection() {
        let source = "abs([first],2)";
        let parser = context("tcl8.4");
        let table = table();
        let mut visited = Vec::new();
        let proof = prepare_expression_witness_with_script_compiler(
            source,
            &parser,
            Some(&table),
            |span| {
                visited.push(source[span.as_range()].to_owned());
                NativeExpressionScriptPreparation::Compiled
            },
        );
        assert_eq!(visited, ["[first]"]);
        assert!(matches!(proof, ExpressionPreparationProof::Rejected(_)));
        let mut visited = Vec::new();
        let proof =
            prepare_expression_witness_with_script_compiler(source, &parser, None, |span| {
                visited.push(span);
                NativeExpressionScriptPreparation::Compiled
            });
        assert!(visited.is_empty());
        assert!(matches!(proof, ExpressionPreparationProof::Unknown));
    }
}
