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
    use super::*;
    use crate::native_compilation::{
        NativeExpressionCompilerStep as Step, NativeMathFunctionResolution as Resolution,
    };

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
