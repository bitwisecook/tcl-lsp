// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained checked expression syntax for original native compiler operands.
//!
//! This is a syntax/preparation receipt, not command or math-function authority.

use crate::InvocationDialect;
use crate::native_compiler_word_projection::NativeCompilerWordOperand;
use crate::native_compiler_words::NativeCompilerWords;
use tcl_lexer::Span;
use tcl_syntax::expr::NativeExprNode;
use tcl_syntax::expr::parser::{CheckedExprParse, ExprParseContext};

/// Capture an actual expression evaluator without deriving one from vendor
/// compatibility or command availability. Function registrations stay separate.
#[must_use]
pub fn native_expression_evaluation_policy(
    profile: &tcl_dialect::DialectProfile,
    point: tcl_dialect::model::DialectPoint,
) -> Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy> {
    let dialect = InvocationDialect::of_profile(profile);
    if dialect.execution_point() != Some(point) {
        return None;
    }
    let context = dialect.expression_parse_context(Some(profile));
    if context.native_syntax == tcl_syntax::expr::parser::NativeExprSyntax::Unknown {
        return None;
    }
    Some(
        tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy {
            profile: profile.cache_key(),
            origin: tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::Native(point),
            context,
            numeric_simulation: None,
            authored_functions: None,
        },
    )
}

/// Capture the explicitly installed F5 parser/numeric simulation. Neither
/// capability authenticates a fixed math table or native expression emission.
#[must_use]
pub fn authored_expression_evaluation_policy(
    profile: &tcl_dialect::DialectProfile,
    parser: crate::invocation_words::LogicalExpressionParseProvider,
    numeric: Option<tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation>,
) -> Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy> {
    let dialect = InvocationDialect::of_profile(profile);
    let context = dialect.logical_expression_parse_context(parser, profile)?;
    let numeric_simulation = match numeric {
        Some(provider) => Some(dialect.authored_logical_numeric_simulation(provider)?),
        None => None,
    };
    Some(
        tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy {
            profile: profile.cache_key(),
            origin:
                tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::AuthoredTcl84Parser,
            context,
            numeric_simulation,
            authored_functions: None,
        },
    )
}

/// Select authored F5 predicates only from the independently installed parser.
/// Native C/Jim policies and absent F5 word grammar grant no dialect operator.
#[must_use]
pub fn authored_f5_string_predicate_provider(
    policy: Option<&tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
) -> Option<tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider> {
    let policy = policy?;
    if policy.origin
        != tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin::AuthoredTcl84Parser
        || policy.context.f5_word_grammar.is_none()
    {
        return None;
    }
    Some(tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider::F5Trunk)
}

/// Select the function protocol from an independently retained evaluator and
/// actual engine. Authored parsing/numbers and missing policy grant no table.
#[must_use]
pub fn expression_function_dispatch(
    policy: Option<&tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
    physical: InvocationDialect,
) -> Option<crate::mathfunc::NativeMathFunctionDispatch> {
    use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
    let policy = policy?;
    let ExpressionEvaluationOrigin::Native(point) = policy.origin else {
        return None;
    };
    if physical.execution_point() != Some(point)
        || native_expression_evaluation_policy(policy.profile.profile(), point).as_ref()
            != Some(policy)
        || physical.expression_parse_context(None).native_syntax != policy.context.native_syntax
    {
        return None;
    }
    crate::mathfunc::native_function_dispatch(physical)
}

/// Original compilation entry selects evaluation and physical function lookup
/// independently. A missing or foreign logical policy remains unavailable.
#[must_use]
pub fn compilation_expression_function_dispatch(
    entry: &tcl_runtime_api::NativeCompilationEntry,
) -> Option<crate::mathfunc::NativeMathFunctionDispatch> {
    let policy = entry.expression_policy.as_ref()?;
    if entry.invocation_policy != Some(policy.profile) {
        return None;
    }
    expression_function_dispatch(
        Some(policy),
        InvocationDialect::of_point(entry.execution_point?),
    )
}

/// Available expression emission, without pooling logical results as native
/// C headers or inventing authored function registrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionProgramEmission {
    /// Authentic native program with compatible evaluation policy.
    Native,
    /// Evaluate unchanged original bytes using the installed authored parser
    /// and numeric capabilities. Implicit functions additionally require the
    /// independently installed authored fixed-function provider.
    AuthoredSource,
    /// A required expression evaluation capability is absent or conflicts.
    Unavailable,
}

fn authored_source_policy(
    policy: &tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy,
) -> bool {
    use tcl_runtime_api::expression_policy::ExpressionEvaluationOrigin;
    if policy.origin != ExpressionEvaluationOrigin::AuthoredTcl84Parser {
        return false;
    }
    let Some(numeric) = policy.numeric_simulation else {
        return false;
    };
    let mut expected = authored_expression_evaluation_policy(
        policy.profile.profile(),
        crate::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
        Some(numeric),
    );
    if let Some(expected) = &mut expected {
        expected.authored_functions = policy.authored_functions;
    }
    expected.as_ref() == Some(policy)
}

/// Preserve a native pruning decision only when the separately installed
/// evaluator accepts the same complete original predicate as the same Boolean.
/// This compares pure policy recipes and grants no physical getter/header.
#[must_use]
pub fn expression_boolean_probe_matches(
    probe: &crate::native_control_compilation::NativeControlBooleanProbe,
    policy: Option<&tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
    physical: InvocationDialect,
) -> bool {
    use tcl_syntax::scalar_getter::{NativeScalarGetterKind, NativeScalarGetterValue};
    let Some(native) = physical
        .native_scalar_getter_protocol()
        .and_then(|protocol| {
            protocol.fresh_conversion(NativeScalarGetterKind::Boolean, &probe.literal)
        })
    else {
        return false;
    };
    if !matches!(native.outcome(), Ok(NativeScalarGetterValue::Boolean(value)) if value == probe.value)
    {
        return false;
    }
    if expression_function_dispatch(policy, physical).is_some() {
        return true;
    }
    policy
        .filter(|policy| authored_source_policy(policy))
        .and_then(|policy| policy.numeric_simulation)
        .and_then(|provider| {
            provider
                .parse_boolean(
                    &probe.literal,
                    tcl_syntax::logical_numeric_simulation::LogicalBooleanInputStage::BooleanValue,
                )
                .ok()
        })
        .is_some_and(|logical| logical.value() == probe.value)
}

/// Check every reached original pruning predicate, including false clauses
/// absent from the instruction. No probe grants function or object authority.
#[must_use]
pub fn control_boolean_probes_match(
    preparations: &[crate::native_control_compilation::NativeControlPreparationStep],
    words: &NativeCompilerWords<'_>,
    policy: Option<&tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
    physical: InvocationDialect,
) -> bool {
    preparations.iter().all(|step| {
        let crate::native_control_compilation::NativeControlPreparationStep::BooleanProbe(probe) =
            step
        else {
            return true;
        };
        physical
            .tcl_version
            .is_some_and(|version| probe.matches_original(words, version))
            && expression_boolean_probe_matches(probe, policy, physical)
    })
}

/// A dynamic original source can use an installed evaluator. Function-table
/// validation remains a separate reached obligation after source preparation.
#[must_use]
pub fn compilation_expression_source_evaluation_supported(
    entry: &tcl_runtime_api::NativeCompilationEntry,
) -> bool {
    entry.expression_policy.as_ref().is_some_and(|policy| {
        entry.execution_point.is_some()
            && entry.invocation_policy == Some(policy.profile)
            && (compilation_expression_function_dispatch(entry).is_some()
                || authored_source_policy(policy))
    })
}

/// Checked topology has no implicit function lookup or unrepresented raw node.
/// This does not authenticate variables, command substitutions or evaluation.
#[must_use]
pub fn expression_tree_is_call_free<Text>(tree: &tcl_syntax::expr::ExprNode<Text>) -> bool {
    use tcl_syntax::expr::ExprNode;
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Call { .. } | ExprNode::Raw { .. } => return false,
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => {}
        }
    }
    true
}

/// Select emission from original source and independently retained evaluation
/// policy. Physical compiler visits still use the original native program.
#[must_use]
pub fn expression_program_emission(
    program: &NativeExpressionProgram,
    entry: &tcl_runtime_api::NativeCompilationEntry,
) -> ExpressionProgramEmission {
    let Some(policy) = entry
        .expression_policy
        .as_ref()
        .filter(|policy| entry.invocation_policy == Some(policy.profile))
    else {
        return ExpressionProgramEmission::Unavailable;
    };
    let Some(point) = entry.execution_point else {
        return ExpressionProgramEmission::Unavailable;
    };
    expression_program_emission_for_policy(
        program,
        Some(policy),
        InvocationDialect::of_point(point),
    )
}

/// Share emission selection with a runtime retaining its actual engine and
/// policy directly. Missing engine or function capability remains unavailable.
#[must_use]
pub fn expression_program_emission_for_policy(
    program: &NativeExpressionProgram,
    policy: Option<&tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
    physical: InvocationDialect,
) -> ExpressionProgramEmission {
    if physical.execution_point().is_none() {
        return ExpressionProgramEmission::Unavailable;
    }
    if program.evaluation_policy_matches(policy, physical) {
        return ExpressionProgramEmission::Native;
    }
    if let Some(policy) = policy.filter(|policy| authored_source_policy(policy))
        && let CheckedExprParse::Parsed(tree) =
            tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(
                &program.source,
                &policy.context,
            )
        && (expression_tree_is_call_free(&tree)
            || crate::authored_math_functions::provider(policy).is_some())
    {
        return ExpressionProgramEmission::AuthoredSource;
    }
    ExpressionProgramEmission::Unavailable
}

/// Original expression program or selected executable syntax failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionTree {
    /// The single checked byte tree, with original inclusive leaf offsets.
    Parsed(NativeExprNode),
    /// A proved native rejection; no recovery tree becomes executable.
    Rejected {
        /// Original native diagnostic bytes.
        message: Vec<u8>,
        /// Selected native error-code spelling, absent when unspecified.
        error_code: Option<Vec<u8>>,
    },
}

/// Checked static expression belonging to one original compiler operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExpressionProgram {
    /// The unchanged word or parser-produced literal member.
    pub operand: NativeCompilerWordOperand,
    /// Counted expression bytes used by the sole checked parser.
    pub source: Vec<u8>,
    /// Corresponding body extent in the retained source image.
    pub span: Span,
    /// Independently selected grammar and native diagnostic axes.
    pub context: ExprParseContext,
    /// Complete tree or proved rejection.
    pub tree: NativeExpressionTree,
}

/// Selected native compilation of a short-circuit logical operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLogicalExpressionCompilation {
    /// C8.4 registers `0`, then `1`, after the left operand and before
    /// compiling the right operand. The selected original normalisation
    /// literal survives the short-circuit test or feeds eager LAND/LOR.
    NormalizedLeft84,
    /// Later C compilers branch around the right operand and register their
    /// result literals after both operand programs have been compiled.
    BranchResult,
}

/// Obtain logical compilation from the independently selected native syntax.
/// Unknown and Jim syntax do not issue a C bytecode recipe.
#[must_use]
pub fn native_logical_expression_compilation(
    syntax: tcl_syntax::expr::parser::NativeExprSyntax,
) -> Option<NativeLogicalExpressionCompilation> {
    use tcl_syntax::expr::parser::NativeExprSyntax;
    match syntax {
        NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4) => {
            Some(NativeLogicalExpressionCompilation::NormalizedLeft84)
        }
        NativeExprSyntax::Tcl(_) => Some(NativeLogicalExpressionCompilation::BranchResult),
        NativeExprSyntax::Jim084 | NativeExprSyntax::Unknown => None,
    }
}

/// A C8.4 parser Boolean word that needs its reached Boolean getter.
/// Numeric `0` and `1` remain registered native-long literals; they do not
/// acquire the parser's word-Boolean primary.
#[must_use]
pub fn native_expression_boolean_word84(bytes: &[u8]) -> bool {
    !matches!(bytes, b"0" | b"1")
        && core::str::from_utf8(bytes)
            .ok()
            .and_then(tcl_syntax::boolean::parse_boolean_word)
            .is_some()
}

impl NativeExpressionProgram {
    /// Native emission requires a separately retained evaluator whose grammar
    /// and numeric issuer agree with this physical preparation. A logical parser
    /// alone cannot authorize C literal pooling, function lookup or body code.
    #[must_use]
    pub fn evaluation_policy_matches(
        &self,
        policy: Option<&tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
        physical: InvocationDialect,
    ) -> bool {
        policy.is_some_and(|policy| {
            expression_function_dispatch(Some(policy), physical).is_some()
                && policy.context.native_syntax == self.context.native_syntax
                && policy.context.expr_grammar_base == self.context.expr_grammar_base
                && policy.context.f5_word_grammar == self.context.f5_word_grammar
        })
    }

    /// Visit original compiler syntax without parsing or rendering its tree.
    /// C8.4 fixed-function lookup precedes argument visits; later C compilers
    /// defer script/syntax failures to execution. The lookup must describe the
    /// actual retained fixed-function table, independently of command lookup.
    #[must_use]
    pub fn compiler_steps(
        &self,
        mut lookup: impl FnMut(&str) -> crate::native_compilation::NativeMathFunctionResolution,
    ) -> Vec<crate::native_compilation::NativeExpressionCompilerStep> {
        use crate::native_compilation::NativeExpressionCompilerStep as Step;
        use tcl_syntax::expr::parser::NativeExprSyntax;
        match self.context.native_syntax {
            NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V8_4) => {}
            NativeExprSyntax::Unknown => return vec![Step::Unknown],
            NativeExprSyntax::Tcl(_) | NativeExprSyntax::Jim084 => return Vec::new(),
        }
        let node = match &self.tree {
            NativeExpressionTree::Parsed(node) => node,
            NativeExpressionTree::Rejected {
                message,
                error_code,
            } => {
                return vec![Step::Failure(
                    crate::native_compilation::NativeCompilationFailure {
                        message: String::from_utf8(message.clone()).ok(),
                        error_code: error_code
                            .as_ref()
                            .and_then(|code| String::from_utf8(code.clone()).ok()),
                        error_info: None,
                    },
                )];
            }
        };
        let Some(scripts) = original_expression_scripts(node, &self.source, &self.context) else {
            return vec![Step::Unknown];
        };
        let mut steps = Vec::new();
        crate::native_compilation::expression_compiler_visits(
            node,
            &scripts,
            &mut lookup,
            &mut steps,
        );
        steps
    }
}

fn original_expression_scripts(
    root: &NativeExprNode,
    source: &[u8],
    context: &ExprParseContext,
) -> Option<Vec<Span>> {
    use tcl_syntax::expr::ExprNode;
    let image = tcl_lexer::SourceImage::native(source);
    let config = tcl_lexer::LexerConfig::from_grammar(context.lexer_grammar);
    let mut nodes = vec![root];
    let mut scripts = Vec::new();
    while let Some(node) = nodes.pop() {
        let span = match node {
            ExprNode::Command { start, end, .. } => {
                scripts.push(Span::new(*start, end.checked_add(1)?));
                continue;
            }
            ExprNode::String { start, end, .. } => {
                if source.get(*start as usize) == Some(&b'{') {
                    continue;
                }
                Span::new(start.checked_add(1)?, *end)
            }
            ExprNode::Var { start, end, .. } => Span::new(*start, end.checked_add(1)?),
            ExprNode::Call { args, .. } => {
                nodes.extend(args.iter().rev());
                continue;
            }
            ExprNode::Unary { operand, .. } => {
                nodes.push(operand);
                continue;
            }
            ExprNode::Binary { left, right, .. } => {
                nodes.push(right);
                nodes.push(left);
                continue;
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                nodes.push(false_branch);
                nodes.push(true_branch);
                nodes.push(condition);
                continue;
            }
            ExprNode::Literal { .. } => continue,
            ExprNode::CompiledWord { .. } | ExprNode::Raw { .. } => return None,
        };
        let arena = tcl_lexer::ExecutablePartArena::decompose(
            image.clone(),
            span,
            tcl_lexer::SubstFlags::default(),
            config,
        )
        .ok()?;
        let mut lists = vec![arena.root()];
        while let Some(list) = lists.pop() {
            for component in arena.list(list).iter().rev() {
                match &component.part {
                    tcl_lexer::ExecutablePart::Command { .. } => scripts.push(component.span),
                    tcl_lexer::ExecutablePart::Variable {
                        index: Some(index), ..
                    } => lists.push(*index),
                    tcl_lexer::ExecutablePart::Text(_)
                    | tcl_lexer::ExecutablePart::Variable { index: None, .. } => {}
                    tcl_lexer::ExecutablePart::Expression { .. }
                    | tcl_lexer::ExecutablePart::ParseError(_) => return None,
                }
            }
        }
    }
    scripts.sort_unstable_by_key(|span| (span.start(), span.end()));
    scripts.dedup();
    Some(scripts)
}

/// Missing original expression syntax or source geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExpressionProgramUnavailable {
    /// No static original expression value is present.
    Dynamic,
    /// Source transformations prevent a direct original-offset mapping.
    SourceGeometry,
    /// The checked parser cannot prove either a tree or native rejection.
    Syntax,
}

/// Native single `SIMPLE_WORD` inline tree or original dynamic `EXPR_STK` inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExpressionInstruction {
    /// Ordered original operands, with spaces emitted between separate words.
    pub operands: Vec<NativeCompilerWordOperand>,
    /// Static tree when the original native compiler reaches `TclCompileExpr`.
    pub program: Option<NativeExpressionProgram>,
}

/// Whether a constant operator returns a resident Boolean spelling rather
/// than an absent-string arithmetic header.
#[must_use]
pub fn native_expression_boolean_operator(node: &NativeExprNode) -> bool {
    use tcl_syntax::expr::{BinOp, ExprNode, UnaryOp};
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not | UnaryOp::WordNot,
            ..
        } => true,
        ExprNode::Binary { op, .. } => matches!(
            op,
            BinOp::And
                | BinOp::Or
                | BinOp::WordAnd
                | BinOp::WordOr
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe
                | BinOp::In
                | BinOp::Ni
        ),
        _ => false,
    }
}

/// Whether the selected folded Boolean result is retained from C8.5's
/// temporary logical-expression compiler, rather than registered in the
/// receiving compiler. C8.6+ registers resident constant results directly.
#[must_use]
pub fn native_expression_private_logical_boolean85(
    node: &NativeExprNode,
    version: tcl_dialect::TclVersion,
) -> bool {
    use tcl_syntax::expr::{BinOp, ExprNode};
    version == tcl_dialect::TclVersion::V8_5
        && matches!(
            node,
            ExprNode::Binary {
                op: BinOp::And | BinOp::Or | BinOp::WordAnd | BinOp::WordOr,
                ..
            }
        )
}

/// Select `TclCompileExprWords`'s static versus substituted expression path.
///
/// # Errors
/// Returns absent original parser/checked-tree evidence, not handler fallback.
pub fn native_expression_instruction(
    words: &NativeCompilerWords<'_>,
    from: usize,
    dialect: InvocationDialect,
) -> Result<NativeExpressionInstruction, NativeExpressionProgramUnavailable> {
    use crate::native_compilation::NativeCompilationWordShape as Shape;
    let projected = crate::native_compiler_word_projection::project_native_compiler_words(
        words,
        dialect
            .tcl_version
            .ok_or(NativeExpressionProgramUnavailable::SourceGeometry)?,
    )
    .map_err(|_| NativeExpressionProgramUnavailable::SourceGeometry)?;
    let args = projected
        .get(from..)
        .filter(|args| !args.is_empty())
        .ok_or(NativeExpressionProgramUnavailable::SourceGeometry)?;
    let program = if args.len() == 1
        && matches!(
            args[0].shape,
            Shape::Literal | Shape::QuotedLiteral | Shape::BracedLiteral
        ) {
        Some(prepare_native_expression_program(
            words,
            &args[0].operand,
            dialect,
        )?)
    } else {
        None
    };
    Ok(NativeExpressionInstruction {
        operands: args.iter().map(|word| word.operand.clone()).collect(),
        program,
    })
}

/// Prepare a static expression once, without reparsing a rendered tree.
///
/// # Errors
/// Returns unavailable transformations or parser evidence independently of a
/// native rejection. Consumers retain the original operand for dynamic paths.
pub fn prepare_native_expression_program(
    words: &NativeCompilerWords<'_>,
    operand: &NativeCompilerWordOperand,
    dialect: InvocationDialect,
) -> Result<NativeExpressionProgram, NativeExpressionProgramUnavailable> {
    use NativeExpressionProgramUnavailable as Unavailable;
    let span = crate::native_control_compilation::native_control_body_span(words, operand)
        .map_err(|_| Unavailable::SourceGeometry)?;
    let source = match operand {
        NativeCompilerWordOperand::Original(index) => words.literal(*index),
        NativeCompilerWordOperand::LiteralExpansion { value, .. } => Some(value.as_slice()),
    }
    .ok_or(Unavailable::Dynamic)?;
    let image = words
        .original_words()
        .first()
        .ok_or(Unavailable::SourceGeometry)?
        .image();
    if image.bytes().get(span.as_range()) != Some(source) {
        return Err(Unavailable::SourceGeometry);
    }
    let context = dialect.expression_parse_context(None);
    let tree =
        match tcl_syntax::expr::parser::parse_expr_bytes_checked_with_context(source, &context) {
            CheckedExprParse::Parsed(tree) => NativeExpressionTree::Parsed(tree),
            CheckedExprParse::ProvedSyntaxFailure(failure) => {
                let diagnostic = failure
                    .native_diagnostic_bytes_with_context(source, &context)
                    .ok_or(Unavailable::Syntax)?;
                NativeExpressionTree::Rejected {
                    message: diagnostic.message,
                    error_code: diagnostic.error_code,
                }
            }
            CheckedExprParse::Unsupported(_) => return Err(Unavailable::Syntax),
        };
    Ok(NativeExpressionProgram {
        operand: operand.clone(),
        source: source.to_vec(),
        span,
        context,
        tree,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn authored_f5_predicates_require_installed_policy_and_reject_native_c_and_jim() {
        use crate::invocation_words::LogicalExpressionParseProvider;
        use tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider;
        let mut policy = authored_expression_evaluation_policy(
            tcl_dialect::DialectProfile::irules(),
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            None,
        )
        .unwrap();
        assert_eq!(
            authored_f5_string_predicate_provider(Some(&policy)),
            Some(AuthoredF5StringPredicateProvider::F5Trunk)
        );
        assert_eq!(authored_f5_string_predicate_provider(None), None);
        policy.context.f5_word_grammar = None;
        assert_eq!(authored_f5_string_predicate_provider(Some(&policy)), None);
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let profile = crate::model::resolve_environment(name).unit_profile();
            let dialect = InvocationDialect::of_profile(profile);
            let policy =
                native_expression_evaluation_policy(profile, dialect.execution_point().unwrap())
                    .unwrap();
            assert_eq!(
                authored_f5_string_predicate_provider(Some(&policy)),
                None,
                "{name}"
            );
        }
    }

    use super::*;
    use crate::native_compilation::{
        NativeExpressionCompilerStep as Step, NativeMathFunctionResolution as Resolution,
    };

    #[test]
    fn expression_evaluator_selects_actual_function_protocol_and_withdraws_foreign_policy() {
        use crate::mathfunc::NativeMathFunctionDispatch;
        use tcl_dialect::TclVersion;
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let dialect = InvocationDialect::for_version(version);
            let profile =
                tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
            let point = dialect.execution_point().unwrap();
            let policy = native_expression_evaluation_policy(profile, point).unwrap();
            assert_eq!(
                expression_function_dispatch(Some(&policy), dialect),
                Some(if version == TclVersion::V8_4 {
                    NativeMathFunctionDispatch::FixedTable
                } else {
                    NativeMathFunctionDispatch::CommandTable
                })
            );
            assert_eq!(
                expression_program_emission_for_policy(
                    &prepare("077+1", version),
                    Some(&policy),
                    dialect
                ),
                ExpressionProgramEmission::Native
            );
            assert_eq!(expression_function_dispatch(None, dialect), None);
        }
        let modern = InvocationDialect::for_version(TclVersion::V9_0);
        let old = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let policy = native_expression_evaluation_policy(
            old,
            InvocationDialect::for_version(TclVersion::V8_4)
                .execution_point()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(expression_function_dispatch(Some(&policy), modern), None);
        assert!(
            native_expression_evaluation_policy(
                tcl_dialect::DialectProfile::irules(),
                modern.execution_point().unwrap()
            )
            .is_none()
        );
    }

    #[test]
    fn authored_expression_source_requires_installed_scalar_policy_and_no_implicit_calls() {
        use crate::invocation_words::LogicalExpressionParseProvider;
        use tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation;
        let physical = InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let profile = tcl_dialect::DialectProfile::irules();
        let policy = authored_expression_evaluation_policy(
            profile,
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            Some(AuthoredLogicalNumericSimulation::Tcl84Core),
        )
        .unwrap();
        for source in ["077+1", "$x+1", "0 && [set lazy 1]"] {
            let program = prepare(source, tcl_dialect::TclVersion::V9_0);
            assert_eq!(
                expression_program_emission_for_policy(&program, Some(&policy), physical),
                ExpressionProgramEmission::AuthoredSource
            );
            assert_eq!(
                program.context.native_syntax,
                tcl_syntax::expr::parser::NativeExprSyntax::Tcl(tcl_dialect::TclVersion::V9_0)
            );
            assert_eq!(program.source, source.as_bytes());
        }
        for source in ["abs(077)", "0 && abs([set skipped 1])", "\"a\" in {a b}"] {
            assert_eq!(
                expression_program_emission_for_policy(
                    &prepare(source, tcl_dialect::TclVersion::V9_0),
                    Some(&policy),
                    physical
                ),
                ExpressionProgramEmission::Unavailable
            );
        }
        let parser_only = authored_expression_evaluation_policy(
            profile,
            LogicalExpressionParseProvider::Tcl84CoreSimulation,
            None,
        )
        .unwrap();
        let program = prepare("077+1", tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            expression_program_emission_for_policy(&program, Some(&parser_only), physical),
            ExpressionProgramEmission::Unavailable
        );
        assert_eq!(expression_function_dispatch(Some(&policy), physical), None);
        assert_eq!(
            expression_program_emission_for_policy(&program, None, physical),
            ExpressionProgramEmission::Unavailable
        );
    }

    fn control_probe_recipe(
        source: &str,
        grammar: crate::native_compilation::NativeCompilationGrammar,
        version: tcl_dialect::TclVersion,
    ) -> (
        tcl_lexer::NativeScriptCommandWords,
        crate::native_control_compilation::NativeControlCompilation<
            crate::native_control_instructions::NativeControlInstruction,
        >,
    ) {
        let dialect = InvocationDialect::for_version(version);
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let command = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, image.len().try_into().unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
        .commands
        .remove(0);
        let captured = NativeCompilerWords::capture(
            &command.words,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        let recipe = crate::native_control_instructions::native_control_instruction(
            grammar,
            &captured,
            1,
            dialect,
            crate::native_compilation::NativeCompilationContext::default(),
        )
        .unwrap();
        (command, recipe)
    }

    #[test]
    fn original_boolean_probes_retain_omitted_false_clauses_and_masked_predicates() {
        use crate::native_compilation::NativeCompilationGrammar as Grammar;
        use crate::native_control_compilation::NativeControlPreparationStep as Visit;
        let physical = InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let policy = authored_expression_evaluation_policy(
            tcl_dialect::DialectProfile::irules(),
            crate::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
            Some(
                tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
            ),
        )
        .unwrap();
        for (source, grammar, expected) in [
            (
                "if {0} {set omitted 1} elseif {1} {set reached 1}",
                Grammar::Conditional,
                vec![false, true],
            ),
            (
                "if {1} {set reached 1} elseif {09} {set masked 1}",
                Grammar::Conditional,
                vec![true],
            ),
            ("while {1} {break}", Grammar::WhileLoop, vec![true]),
            ("while {0} {set omitted 1}", Grammar::WhileLoop, vec![false]),
        ] {
            let (command, recipe) =
                control_probe_recipe(source, grammar, tcl_dialect::TclVersion::V9_0);
            let captured = NativeCompilerWords::capture(
                &command.words,
                physical.native_source_string_protocol().unwrap(),
            )
            .unwrap();
            let values: Vec<_> = recipe
                .preparations
                .iter()
                .filter_map(|visit| match visit {
                    Visit::BooleanProbe(probe) => Some(probe.value),
                    _ => None,
                })
                .collect();
            assert_eq!(values, expected, "{source}");
            assert!(
                control_boolean_probes_match(
                    &recipe.preparations,
                    &captured,
                    Some(&policy),
                    physical
                ),
                "{source}"
            );
            assert_eq!(
                recipe
                    .preparations
                    .iter()
                    .filter(|visit| matches!(visit, Visit::Script { .. }))
                    .count(),
                usize::from(!source.starts_with("while {0}"))
            );
        }
    }

    #[test]
    fn original_boolean_probes_refuse_changed_source_missing_policy_and_logical_number_mismatch() {
        use crate::native_compilation::NativeCompilationGrammar as Grammar;
        let physical = InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let profile = tcl_dialect::DialectProfile::irules();
        let policy = authored_expression_evaluation_policy(
            profile,
            crate::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
            Some(
                tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
            ),
        )
        .unwrap();
        for source in ["if {09} {set reached 1}", "if {0o10} {set reached 1}"] {
            let (command, recipe) =
                control_probe_recipe(source, Grammar::Conditional, tcl_dialect::TclVersion::V9_0);
            let captured = NativeCompilerWords::capture(
                &command.words,
                physical.native_source_string_protocol().unwrap(),
            )
            .unwrap();
            assert!(
                !control_boolean_probes_match(
                    &recipe.preparations,
                    &captured,
                    Some(&policy),
                    physical
                ),
                "{source}"
            );
        }
        let (command, recipe) = control_probe_recipe(
            "if {1} {set x 1}",
            Grammar::Conditional,
            tcl_dialect::TclVersion::V9_0,
        );
        let captured = NativeCompilerWords::capture(
            &command.words,
            physical.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        assert!(!control_boolean_probes_match(
            &recipe.preparations,
            &captured,
            None,
            physical
        ));
        let parser_only = authored_expression_evaluation_policy(
            profile,
            crate::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation,
            None,
        )
        .unwrap();
        assert!(!control_boolean_probes_match(
            &recipe.preparations,
            &captured,
            Some(&parser_only),
            physical
        ));
        let (changed, _) = control_probe_recipe(
            "if {0} {set x 1}",
            Grammar::Conditional,
            tcl_dialect::TclVersion::V9_0,
        );
        let changed = NativeCompilerWords::capture(
            &changed.words,
            physical.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        assert!(!control_boolean_probes_match(
            &recipe.preparations,
            &changed,
            Some(&policy),
            physical
        ));
    }

    #[test]
    fn logical_recipe_requires_selected_c_syntax_and_preserves_numeric_literals() {
        use tcl_syntax::expr::parser::NativeExprSyntax;
        assert_eq!(
            native_logical_expression_compilation(NativeExprSyntax::Tcl(
                tcl_dialect::TclVersion::V8_4
            )),
            Some(NativeLogicalExpressionCompilation::NormalizedLeft84)
        );
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            assert_eq!(
                native_logical_expression_compilation(NativeExprSyntax::Tcl(version)),
                Some(NativeLogicalExpressionCompilation::BranchResult)
            );
        }
        for syntax in [NativeExprSyntax::Unknown, NativeExprSyntax::Jim084] {
            assert_eq!(native_logical_expression_compilation(syntax), None);
        }
        for literal in [b"0".as_slice(), b"1", b"2", b"0x10"] {
            assert!(!native_expression_boolean_word84(literal));
        }
        for literal in [b"false".as_slice(), b"true", b"yes", b"off"] {
            assert!(native_expression_boolean_word84(literal));
        }
    }

    fn prepare(expression: &str, version: tcl_dialect::TclVersion) -> NativeExpressionProgram {
        let source = format!("expr {{{expression}}}");
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let dialect = crate::InvocationDialect::for_version(version);
        let command = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, image.len().try_into().unwrap()),
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
        .unwrap()
        .commands
        .remove(0);
        let words = NativeCompilerWords::capture(
            &command.words,
            dialect.native_source_string_protocol().unwrap(),
        )
        .unwrap();
        native_expression_instruction(&words, 1, dialect)
            .unwrap()
            .program
            .unwrap()
    }

    #[test]
    fn retained_checked_expression_visits_fixed_lookup_before_original_argument_scripts() {
        let program = prepare(
            "abs([set first 1], [set skipped 2])",
            tcl_dialect::TclVersion::V8_4,
        );
        let mut looked_up = Vec::new();
        let steps = program.compiler_steps(|name| {
            looked_up.push(name.to_owned());
            Resolution::Known { arity: 1 }
        });
        assert_eq!(looked_up, vec!["abs"]);
        assert_eq!(steps.len(), 2);
        assert!(
            matches!(steps.first(), Some(Step::Script(span)) if &program.source[span.as_range()] == b"[set first 1]")
        );
        assert!(
            matches!(steps.last(), Some(Step::Failure(failure)) if failure.message.as_deref() == Some("too many arguments for math function"))
        );
        assert!(matches!(
            program.compiler_steps(|_| Resolution::Absent).as_slice(),
            [Step::Failure(_)]
        ));
        assert_eq!(
            program.compiler_steps(|_| Resolution::Unknown),
            vec![Step::Unknown]
        );
    }

    #[test]
    fn retained_checked_expression_uses_original_quoted_and_index_substitution_geometry() {
        for (expression, scripts) in [
            (
                "\"prefix [set x 1] $a([set k 2])\"",
                vec!["[set x 1]", "[set k 2]"],
            ),
            ("{[set inert 1]}", Vec::new()),
            ("0 && [set lazy 1]", vec!["[set lazy 1]"]),
        ] {
            let program = prepare(expression, tcl_dialect::TclVersion::V8_4);
            let retained: Vec<_> = program
                .compiler_steps(|_| Resolution::Unknown)
                .into_iter()
                .map(|step| {
                    let Step::Script(span) = step else {
                        panic!("closed original leaf geometry")
                    };
                    String::from_utf8(program.source[span.as_range()].to_vec()).unwrap()
                })
                .collect();
            assert_eq!(retained, scripts);
        }
        let modern = prepare("0 && [set lazy 1]", tcl_dialect::TclVersion::V9_0);
        assert!(
            modern
                .compiler_steps(|_| panic!("modern compiler has no fixed table lookup"))
                .is_empty()
        );
    }
}
