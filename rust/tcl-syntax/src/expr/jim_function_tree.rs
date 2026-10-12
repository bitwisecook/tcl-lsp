// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Jim's eager function resolution and shared argument-term stack.

use super::ast::{BinOp, ExprNode, ExprText, UnaryOp};
use super::parser::{ExprParseContext, NativeExprSyntax, NativeExprSyntaxDiagnostic};
use std::collections::HashMap;
use tcl_lexer::{ExprToken, ExprTokenType as Kind};

/// Evidence from the actual fixed function table, independently of catalogue data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFunctionArity {
    /// The installed implementation has this exact arity.
    Known(usize),
    /// A complete observed table proves the name absent.
    Absent,
    /// Registration or arity is unknown.
    Unknown,
}

/// A native expression tree, a proved rejection, or an outstanding provider obligation.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeFunctionTree<Text = String> {
    /// Execute this tree: Jim can consume enclosing arguments in an inner call.
    Parsed(ExprNode<Text>),
    /// Rejection precedes every expression substitution, including lazy branches.
    Rejected(NativeExprSyntaxDiagnostic<Text>),
    /// No recovery tree or nominal function roster licenses execution.
    Unknown,
}

#[derive(Debug)]
enum Failure {
    Message(Vec<u8>),
    Unknown,
}
type Result<T> = std::result::Result<T, Failure>;

fn shown(source: &[u8], reason: &str) -> Failure {
    let mut message = format!("{reason} in expression: \"").into_bytes();
    message.extend_from_slice(super::syntax_error::nul_terminated(source));
    message.push(b'"');
    Failure::Message(message)
}

fn function_parentheses_error(source: &[u8]) -> Failure {
    let mut message = b"syntax error in expression: \"".to_vec();
    message.extend_from_slice(super::syntax_error::nul_terminated(source));
    message.extend_from_slice(b"\": function requires parentheses");
    Failure::Message(message)
}

fn lexical_functions<Text: ExprText>(
    source: &[u8],
    tokens: &[ExprToken<Text>],
    lookup: &mut impl FnMut(&str) -> NativeFunctionArity,
) -> Result<HashMap<String, usize>> {
    let mut functions = HashMap::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != Kind::Function {
            continue;
        }
        let name = token.text.try_text().ok_or(Failure::Unknown)?;
        match lookup(name) {
            NativeFunctionArity::Known(arity) => {
                if tokens.get(index + 1).map(|next| next.kind) != Some(Kind::ParenOpen) {
                    return Err(function_parentheses_error(source));
                }
                functions.insert(name.to_owned(), arity);
            }
            NativeFunctionArity::Unknown => return Err(Failure::Unknown),
            NativeFunctionArity::Absent => {
                // Jim matches the longest installed operator prefix before
                // checking its following parenthesis (for example `absx`).
                for (end, _) in name.char_indices().rev().filter(|(end, _)| *end != 0) {
                    match lookup(&name[..end]) {
                        NativeFunctionArity::Known(_) => {
                            return Err(function_parentheses_error(source));
                        }
                        NativeFunctionArity::Unknown => return Err(Failure::Unknown),
                        NativeFunctionArity::Absent => {}
                    }
                }
                return Err(shown(source, "syntax error"));
            }
        }
    }
    Ok(functions)
}

/// Build the Jim 0.84 native tree using actual function registrations.
/// Function names are checked in a complete lexical pass before any arity
/// checks. Tree construction then uses one term stack across nested calls;
/// `abs(pow(1),2)` therefore means `abs(pow(1,2))`. Callers must execute the
/// returned tree and retain the actual function-table identity/generation as
/// a cache prerequisite. Unsupported grammar and incomplete tables decline.
#[must_use]
pub fn prepare_jim_function_tree(
    source: &str,
    context: &ExprParseContext,
    lookup: impl FnMut(&str) -> NativeFunctionArity,
) -> NativeFunctionTree {
    prepare_tree(source.as_bytes(), context, lookup)
}

/// Prepare the same Jim term-stack tree over original native source bytes.
#[must_use]
pub fn prepare_jim_function_tree_bytes(
    source: &[u8],
    context: &ExprParseContext,
    lookup: impl FnMut(&str) -> NativeFunctionArity,
) -> NativeFunctionTree<Vec<u8>> {
    prepare_tree(source, context, lookup)
}

/// Native fixed-table tree and independent original primary preparation facts.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedNativeFunctionTree<Text = String> {
    /// Tree execution or native rejection from the actual fixed function table.
    pub tree: NativeFunctionTree<Text>,
    /// Native preparation action and original term roster; absent if unavailable.
    pub jim: Option<super::native_objects::JimExpressionPreparation>,
}

/// Prepare original byte terms and representation action in the same lexical pass.
#[must_use]
pub fn prepare_jim_function_tree_bytes_with_preparation(
    source: &[u8],
    context: &ExprParseContext,
    lookup: impl FnMut(&str) -> NativeFunctionArity,
) -> PreparedNativeFunctionTree<Vec<u8>> {
    prepare_tree_receipt(source, context, lookup)
}

fn prepare_tree<Text: ExprText>(
    source: &[u8],
    context: &ExprParseContext,
    lookup: impl FnMut(&str) -> NativeFunctionArity,
) -> NativeFunctionTree<Text> {
    prepare_tree_receipt(source, context, lookup).tree
}

fn prepare_tree_receipt<Text: ExprText>(
    source: &[u8],
    context: &ExprParseContext,
    mut lookup: impl FnMut(&str) -> NativeFunctionArity,
) -> PreparedNativeFunctionTree<Text> {
    if context.native_syntax != NativeExprSyntax::Jim084 {
        return PreparedNativeFunctionTree {
            tree: NativeFunctionTree::Unknown,
            jim: None,
        };
    }
    let (raw, failures) = tcl_lexer::tokenise_expr_bytes_with_failures(
        source,
        &context.lexer_grammar,
        context.expr_grammar_base,
        context.f5_word_grammar,
    );
    let tokens: Vec<ExprToken<Text>> = raw
        .iter()
        .filter(|token| !token.kind.is_skipped())
        .map(|token| ExprToken {
            kind: token.kind,
            text: Text::from_source_bytes(&token.text),
            start: token.start,
            end: token.end,
        })
        .collect();
    let lexical = lexical_functions(source, &tokens, &mut lookup);
    let lexical_rejection = matches!(&lexical, Err(Failure::Message(_)));
    let tree = if let Err(failure) = lexical.as_ref() {
        native_tree_failure(failure)
    } else if !failures.is_empty() || tokens.is_empty() {
        original_syntax_failure(source, context, raw.clone(), failures.clone())
    } else {
        let functions = lexical.unwrap_or_default();
        build_tree(source, context, &tokens, functions)
    };
    let jim = if matches!(tree, NativeFunctionTree::Unknown) {
        None
    } else {
        super::native_objects::jim_preparation_for_tree(
            source,
            context,
            &raw,
            &failures,
            matches!(tree, NativeFunctionTree::Parsed(_)),
            lexical_rejection,
        )
    };
    PreparedNativeFunctionTree { tree, jim }
}

fn original_syntax_failure<Text: ExprText>(
    source: &[u8],
    context: &ExprParseContext,
    mut tokens: Vec<ExprToken<Vec<u8>>>,
    failures: Vec<tcl_lexer::ExprLexicalFailure>,
) -> NativeFunctionTree<Text> {
    let super::parser::CheckedExprParse::ProvedSyntaxFailure(failure) =
        super::parser::parse_native_token_stream(source, context, &mut tokens, failures)
    else {
        return NativeFunctionTree::Unknown;
    };
    failure
        .native_diagnostic_bytes_with_context(source, context)
        .map_or(NativeFunctionTree::Unknown, |diagnostic| {
            NativeFunctionTree::Rejected(NativeExprSyntaxDiagnostic {
                message: Text::from_source_bytes(&diagnostic.message),
                error_code: diagnostic
                    .error_code
                    .map(|code| Text::from_source_bytes(&code)),
            })
        })
}

fn native_tree_failure<Text: ExprText>(failure: &Failure) -> NativeFunctionTree<Text> {
    match failure {
        Failure::Message(message) => NativeFunctionTree::Rejected(NativeExprSyntaxDiagnostic {
            message: Text::from_source_bytes(message),
            error_code: None,
        }),
        Failure::Unknown => NativeFunctionTree::Unknown,
    }
}

fn build_tree<Text: ExprText>(
    source: &[u8],
    context: &ExprParseContext,
    tokens: &[ExprToken<Text>],
    functions: HashMap<String, usize>,
) -> NativeFunctionTree<Text> {
    let mut builder = Builder {
        source,
        variable_config: tcl_lexer::LexerConfig::default().with_grammar(context.lexer_grammar),
        tokens,
        functions,
        position: 0,
        depth: 0,
        parentheses: 0,
        terms: Vec::new(),
    };
    let result = builder.build(0, Scope::Expression, 1).and_then(|()| {
        if builder.parentheses != 0 {
            return Err(Failure::Message(b"missing close parenthesis".to_vec()));
        }
        if builder.terms.len() != 1 {
            return Err(Failure::Unknown);
        }
        builder.terms.pop().ok_or(Failure::Unknown)
    });
    match result {
        Ok(tree) => NativeFunctionTree::Parsed(tree),
        Err(failure) => native_tree_failure(&failure),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Expression,
    Parentheses,
    Function,
    Ternary,
}

struct Builder<'a, Text: ExprText> {
    source: &'a [u8],
    variable_config: tcl_lexer::LexerConfig,
    tokens: &'a [ExprToken<Text>],
    functions: HashMap<String, usize>,
    position: usize,
    depth: usize,
    parentheses: i32,
    terms: Vec<ExprNode<Text>>,
}

impl<Text: ExprText> Builder<'_, Text> {
    fn build(&mut self, precedence: u16, scope: Scope, count: usize) -> Result<()> {
        self.depth += 1;
        if self.depth > 200 {
            return Err(Failure::Unknown);
        }
        let expected = self
            .terms
            .len()
            .checked_add(count)
            .ok_or(Failure::Unknown)?;
        while self
            .tokens
            .get(self.position)
            .is_some_and(|token| token.kind != Kind::Eof)
        {
            let token = &self.tokens[self.position];
            let previous = self
                .position
                .checked_sub(1)
                .map(|index| self.tokens[index].kind);
            self.position += 1;
            match token.kind {
                Kind::ParenOpen => {
                    if self.terms.len() == expected {
                        return Err(shown(self.source, "unexpected open parenthesis"));
                    }
                    self.parentheses += 1;
                    self.build(0, Scope::Parentheses, 1)?;
                }
                Kind::ParenClose => {
                    if !matches!(scope, Scope::Parentheses | Scope::Function) {
                        if self.terms.len() == expected && self.depth > 1 {
                            self.position -= 1;
                            self.depth -= 1;
                            return Ok(());
                        }
                        return Err(shown(self.source, "unexpected closing parenthesis"));
                    }
                    self.parentheses -= 1;
                    if self.terms.len() == expected {
                        break;
                    }
                }
                Kind::Comma => {
                    if scope != Scope::Function {
                        if self.terms.len() == expected {
                            self.position -= 1;
                            self.depth -= 1;
                            return Ok(());
                        }
                        return Err(shown(self.source, "unexpected comma"));
                    }
                    if self.terms.len() > expected {
                        return Err(Failure::Message(
                            b"too many arguments to math function".to_vec(),
                        ));
                    }
                }
                Kind::TernaryC => {
                    if scope != Scope::Ternary && self.depth == 1 {
                        return Err(shown(self.source, ": without ?"));
                    }
                    if scope != Scope::Ternary || self.terms.len() == expected {
                        self.position -= 1;
                        self.depth -= 1;
                        return Ok(());
                    }
                }
                Kind::Function | Kind::Operator | Kind::TernaryQ => {
                    let prefix = previous.is_none_or(is_expression_start);
                    if !self.apply_operator(token, prefix, precedence)? {
                        break;
                    }
                }
                Kind::Number | Kind::Bool | Kind::String | Kind::Variable | Kind::Command => {
                    if !previous.is_none_or(is_expression_start) {
                        return Err(shown(self.source, "missing operator"));
                    }
                    self.terms.push(leaf(token, self.variable_config)?);
                }
                _ => return Err(Failure::Unknown),
            }
        }
        self.finish(scope, expected)
    }

    fn finish(&mut self, scope: Scope, expected: usize) -> Result<()> {
        if self.terms.len() == expected {
            self.depth -= 1;
            return Ok(());
        }
        let message = if scope == Scope::Function {
            format!(
                "too {} arguments for math function",
                if self.terms.len() < expected {
                    "few"
                } else {
                    "many"
                }
            )
            .into_bytes()
        } else if self.terms.len() < expected {
            let mut message = b"syntax error in expression \"".to_vec();
            message.extend_from_slice(super::syntax_error::nul_terminated(self.source));
            message.extend_from_slice(b"\": premature end of expression");
            message
        } else {
            b"extra terms after expression".to_vec()
        };
        Err(Failure::Message(message))
    }

    fn apply_operator(
        &mut self,
        token: &ExprToken<Text>,
        prefix: bool,
        precedence: u16,
    ) -> Result<bool> {
        let operator = if token.kind == Kind::Function {
            Operator::Function(
                *self
                    .functions
                    .get(token.text.try_text().ok_or(Failure::Unknown)?)
                    .ok_or(Failure::Unknown)?,
            )
        } else if token.kind == Kind::TernaryQ {
            Operator::Ternary
        } else {
            operator(token.text.try_text().ok_or(Failure::Unknown)?, prefix)
                .ok_or(Failure::Unknown)?
        };
        let (binding, right_associative) =
            operator.binding(token.text.try_text().ok_or(Failure::Unknown)?)?;
        if binding < precedence || (!right_associative && binding == precedence) {
            self.position -= 1;
            return Ok(false);
        }
        match operator {
            Operator::Function(arity) => self.function_arguments(arity)?,
            Operator::Ternary => self.build(binding, Scope::Ternary, 2)?,
            Operator::Unary(_) | Operator::Binary(_) => {
                self.build(binding, Scope::Expression, 1)?;
            }
        }
        let arity = operator.arity();
        if self.terms.len() < arity {
            return Err(shown(
                self.source,
                &format!(
                    "missing operand to {}",
                    token.text.try_text().ok_or(Failure::Unknown)?
                ),
            ));
        }
        let arguments = self.terms.split_off(self.terms.len() - arity);
        let end = self
            .tokens
            .get(self.position.saturating_sub(1))
            .map_or(token.end, |last| last.end);
        self.terms.push(operator.node(token, end, arguments));
        Ok(true)
    }

    fn function_arguments(&mut self, arity: usize) -> Result<()> {
        if self.tokens.get(self.position).map(|token| token.kind) != Some(Kind::ParenOpen) {
            return Err(Failure::Message(
                b"missing arguments for math function".to_vec(),
            ));
        }
        self.position += 1;
        if arity == 0 {
            if self.tokens.get(self.position).map(|token| token.kind) != Some(Kind::ParenClose) {
                return Err(Failure::Message(
                    b"too many arguments for math function".to_vec(),
                ));
            }
            self.position += 1;
            Ok(())
        } else {
            self.parentheses += 1;
            self.build(0, Scope::Function, arity)
        }
    }
}

fn is_expression_start(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::ParenOpen
            | Kind::Comma
            | Kind::Operator
            | Kind::TernaryQ
            | Kind::TernaryC
            | Kind::Function
    )
}

fn leaf<Text: ExprText>(
    token: &ExprToken<Text>,
    config: tcl_lexer::LexerConfig,
) -> Result<ExprNode<Text>> {
    let text = token.text.clone();
    let (start, end) = (token.start, token.end);
    Ok(match token.kind {
        Kind::Variable => ExprNode::Var {
            name: Text::from_source_bytes(
                crate::naming::variable_reference_root_bytes(text.bytes(), config)
                    .map_err(|_| Failure::Unknown)?
                    .ok_or(Failure::Unknown)?,
            ),
            text,
            start,
            end,
        },
        Kind::Command => ExprNode::Command { text, start, end },
        Kind::String => ExprNode::String { text, start, end },
        _ => ExprNode::Literal { text, start, end },
    })
}

#[derive(Clone, Copy)]
enum Operator {
    Function(usize),
    Ternary,
    Unary(UnaryOp),
    Binary(BinOp),
}

impl Operator {
    fn arity(self) -> usize {
        match self {
            Self::Function(arity) => arity,
            Self::Ternary => 3,
            Self::Unary(_) => 1,
            Self::Binary(_) => 2,
        }
    }
    fn binding(self, spelling: &str) -> Result<(u16, bool)> {
        match self {
            Self::Function(_) => Ok((400, false)),
            Self::Ternary => Ok((10, true)),
            Self::Unary(_) => Ok((300, true)),
            Self::Binary(_) => {
                let grammar = tcl_dialect::model::expr_grammar::expr(
                    tcl_dialect::model::Family::Jim,
                    tcl_dialect::model::Release::JIM_0_84,
                );
                let (left, right) = grammar
                    .precedence
                    .lookup(spelling)
                    .ok_or(Failure::Unknown)?;
                Ok((left, left == right))
            }
        }
    }
    fn node<Text: ExprText>(
        self,
        token: &ExprToken<Text>,
        end: u32,
        mut args: Vec<ExprNode<Text>>,
    ) -> ExprNode<Text> {
        match self {
            Self::Function(_) => ExprNode::Call {
                function: token.text.clone(),
                args,
                start: token.start,
                end,
            },
            Self::Unary(op) => ExprNode::Unary {
                op,
                operand: Box::new(args.remove(0)),
            },
            Self::Binary(op) => ExprNode::Binary {
                op,
                left: Box::new(args.remove(0)),
                right: Box::new(args.remove(0)),
            },
            Self::Ternary => ExprNode::Ternary {
                condition: Box::new(args.remove(0)),
                true_branch: Box::new(args.remove(0)),
                false_branch: Box::new(args.remove(0)),
            },
        }
    }
}

fn operator(text: &str, prefix: bool) -> Option<Operator> {
    if (prefix || matches!(text, "!" | "~"))
        && let Some(op) = super::parser::unaryop_from_text(text)
    {
        return Some(Operator::Unary(op));
    }
    let op = match text {
        "=*" => BinOp::MatchesGlob,
        "=~" => BinOp::MatchesRegex,
        _ => super::parser::binop_from_text(text)?,
    };
    Some(Operator::Binary(op))
}

#[cfg(test)]
mod tests {
    #[test]
    fn jim_reference_tree_shares_selected_utf8_and_opaque_roots() {
        // Implementation contract: naming.expression.selected-reference-root
        // docs/design/analysis/name-resolution-proofs/expression-selected-reference-root.md
        for (source, expected) in [
            (b"${scalar(open}".as_slice(), b"scalar(open".as_slice()),
            (b"${scalar(open)tail}", b"scalar(open)tail"),
            (b"${arr(key)}", b"arr"),
            (b"$arr(\xff)", b"arr"),
            (b"${\xff(key)}", b"\xff"),
            (b"${\xff(open}", b"\xff(open"),
            (b"${nul\0tail(key)}", b"nul\0tail"),
        ] {
            let NativeFunctionTree::Parsed(ExprNode::Var { text, name, .. }) =
                prepare_jim_function_tree_bytes(source, &context(), |_| {
                    NativeFunctionArity::Absent
                })
            else {
                panic!("whole Jim reference declined");
            };
            assert_eq!(text, source);
            assert_eq!(name, expected);
            if let Ok(source) = std::str::from_utf8(source) {
                let NativeFunctionTree::Parsed(ExprNode::Var { text, name, .. }) = prepare(source)
                else {
                    panic!("UTF-8 Jim reference declined");
                };
                assert_eq!(text, source);
                assert_eq!(name.as_bytes(), expected);
            }
        }
        let mut selected = context();
        selected.lexer_grammar.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
        assert!(
            matches!(prepare_jim_function_tree("${a{b}c}", &selected, |_| NativeFunctionArity::Absent), NativeFunctionTree::Parsed(ExprNode::Var { name, .. }) if name == "a{b}c")
        );
        assert!(!matches!(
            prepare_jim_function_tree("${a{b}", &selected, |_| NativeFunctionArity::Absent),
            NativeFunctionTree::Parsed(_)
        ));
    }

    use super::*;
    fn context() -> ExprParseContext {
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        ExprParseContext::for_profile(&profile)
    }
    fn prepare(source: &str) -> NativeFunctionTree {
        prepare_jim_function_tree(source, &context(), |name| match name {
            "abs" => NativeFunctionArity::Known(1),
            "pow" => NativeFunctionArity::Known(2),
            "rand" => NativeFunctionArity::Known(0),
            _ => NativeFunctionArity::Absent,
        })
    }
    #[test]
    fn fixed_tree_receipt_retains_native_action_and_original_terms() {
        use super::super::native_objects::{
            JimExpressionCacheAction as Action, JimExpressionPreparationStage as Stage,
        };
        let lookup = |name: &str| match name {
            "abs" => NativeFunctionArity::Known(1),
            "pow" => NativeFunctionArity::Known(2),
            _ => NativeFunctionArity::Absent,
        };
        let prepared =
            prepare_jim_function_tree_bytes_with_preparation(b"abs(pow(1),2)", &context(), lookup);
        assert!(matches!(prepared.tree, NativeFunctionTree::Parsed(_)));
        let primary = prepared.jim.unwrap();
        assert_eq!(
            (primary.stage, primary.action),
            (Stage::Tree, Action::Prepared)
        );
        assert_eq!(
            primary
                .terms
                .iter()
                .map(|term| term.source.clone())
                .collect::<Vec<_>>(),
            vec![8..9, 11..12]
        );
        for (source, stage, action) in [
            (
                b"".as_slice(),
                Stage::Completeness,
                Action::PreserveOriginal,
            ),
            (b"{", Stage::Completeness, Action::PreserveOriginal),
            (b"1+", Stage::Tree, Action::Rejected),
            (b"future(1)", Stage::Tokenization, Action::Rejected),
        ] {
            let result =
                prepare_jim_function_tree_bytes_with_preparation(source, &context(), lookup);
            assert!(
                matches!(result.tree, NativeFunctionTree::Rejected(_)),
                "{source:?}"
            );
            let primary = result.jim.unwrap();
            assert_eq!(
                (primary.stage, primary.action),
                (stage, action),
                "{source:?}"
            );
            assert_eq!(primary.terms, [] as [tcl_lexer::ExprTerm; 0]);
        }
        let unavailable =
            prepare_jim_function_tree_bytes_with_preparation(b"future(1)", &context(), |_| {
                NativeFunctionArity::Unknown
            });
        assert!(matches!(unavailable.tree, NativeFunctionTree::Unknown));
        assert_eq!(unavailable.jim, None);
    }

    #[test]
    fn names_are_resolved_eagerly_before_arity_and_lazy_evaluation() {
        for source in [
            "0 && future_function(1)",
            "1 || future_function(1)",
            "1 ? 7 : future_function(1)",
            "abs(future_function([mark]),2)",
        ] {
            assert!(
                matches!(prepare(source), NativeFunctionTree::Rejected(error)
                if error.message == format!("syntax error in expression: \"{source}\""))
            );
        }
        for (source, expected) in [
            ("0 && abs([mark],2)", "too many arguments for math function"),
            ("1 || pow([mark])", "too few arguments for math function"),
        ] {
            assert!(
                matches!(prepare(source), NativeFunctionTree::Rejected(error) if error.message==expected)
            );
        }
    }
    #[test]
    fn inner_calls_can_consume_enclosing_comma_arguments() {
        let NativeFunctionTree::Parsed(ExprNode::Call { function, args, .. }) =
            prepare("abs(pow([mark]),2)")
        else {
            panic!("native reassociation must parse");
        };
        assert_eq!(function, "abs");
        assert_eq!(args.len(), 1);
        assert!(
            matches!(&args[0], ExprNode::Call {function,args,..} if function=="pow" && args.len()==2)
        );
        assert!(
            matches!(prepare("pow(abs([mark],2))"), NativeFunctionTree::Rejected(error) if error.message=="too many arguments for math function")
        );
    }
    #[test]
    fn empty_segments_and_prefix_function_rejections_match_native_tree_builder() {
        for source in ["abs(,1)", "abs(1,,)", "pow(1,,2)", "0 && abs(1,)"] {
            assert!(
                matches!(prepare(source), NativeFunctionTree::Parsed(_)),
                "{source}"
            );
        }
        assert!(
            matches!(prepare("rand(,)"),NativeFunctionTree::Rejected(error) if error.message=="too many arguments for math function")
        );
        assert!(
            matches!(prepare("absx(1)"),NativeFunctionTree::Rejected(error) if error.message.ends_with(": function requires parentheses"))
        );
        assert!(
            matches!(prepare("abs(1+)"),NativeFunctionTree::Rejected(error) if error.message=="unexpected closing parenthesis in expression: \"abs(1+)\"")
        );
    }
    #[test]
    fn actual_registration_and_native_policy_are_required() {
        assert!(matches!(
            prepare_jim_function_tree("abs(1)", &context(), |_| NativeFunctionArity::Unknown),
            NativeFunctionTree::Unknown
        ));
        let c = ExprParseContext::for_profile(tcl_dialect::DialectProfile::find("tcl9.0").unwrap());
        assert!(matches!(
            prepare_jim_function_tree("abs(1)", &c, |_| NativeFunctionArity::Known(1)),
            NativeFunctionTree::Unknown
        ));
    }
}
