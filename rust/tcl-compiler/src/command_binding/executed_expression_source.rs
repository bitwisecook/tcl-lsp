// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evaluated native expression bytes and piecewise original operand origins.

use super::{
    CommandAllocationSite, ExecutedScriptMapping, ExecutedScriptSource, MaterialisedSourceKind,
    SourceOriginId,
};
use crate::ir::{Provenance, SourceSite, WordExpr};
use std::sync::Arc;
use tcl_lexer::{LexerConfig, Span};

/// One exact native expression value with truthful operand source mappings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutedExpressionSource {
    /// Bytes received by expression parsing after native argument combination.
    pub text: Arc<str>,
    /// Actual derived expression source; it is never an editable script range.
    pub origin: Arc<SourceOriginId>,
    pieces: Vec<ExpressionSourcePiece>,
    config: LexerConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ExpressionSourcePiece {
    expression_span: Span,
    original: ExecutedScriptSource,
    trimmed: u32,
}

impl ExecutedExpressionSource {
    /// A single unchanged written expression's original parser base. Separate
    /// operands, materialised values and changed slices cannot borrow an
    /// authored expression extent through their equal evaluated bytes.
    #[must_use]
    pub fn original_expression_mapping(&self) -> Option<(&Arc<SourceOriginId>, u32)> {
        let [piece] = self.pieces.as_slice() else {
            return None;
        };
        if piece.expression_span.start() != 0
            || usize::try_from(piece.expression_span.end()).ok()? != self.text.len()
        {
            return None;
        }
        let ExecutedScriptMapping::Contiguous { base } = piece.original.mapping else {
            return None;
        };
        let trimmed = usize::try_from(piece.trimmed).ok()?;
        if piece.original.text.bytes().get(trimmed..)? != self.text.as_bytes() {
            return None;
        }
        Some((&piece.original.origin, base.checked_add(piece.trimmed)?))
    }

    /// Retain native expression concatenation and each unchanged operand slice.
    /// Expanded operands and unknown values do not supply this proof.
    #[must_use]
    pub fn from_arguments(
        parent: CommandAllocationSite,
        words: &[WordExpr],
        values: &[String],
        dialect: tcl_registry::InvocationDialect,
    ) -> Option<Self> {
        let grammar = dialect.expression_arguments()?;
        if words.len() != values.len() || !grammar.accepts_len(values.len()) {
            return None;
        }
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let mut text = String::new();
        let mut pieces = Vec::new();
        for (index, (word, value)) in words.iter().zip(values).enumerate() {
            if matches!(word, WordExpr::Expand { .. }) {
                return None;
            }
            let selected = if values.len() == 1 {
                value.as_str()
            } else {
                tcl_syntax::list::trim_concat_element(value)
            };
            if values.len() > 1 && selected.is_empty() {
                continue;
            }
            if !text.is_empty() && values.len() > 1 {
                text.push(' ');
            }
            let start = u32::try_from(text.len()).ok()?;
            text.push_str(selected);
            let end = u32::try_from(text.len()).ok()?;
            let trimmed =
                u32::try_from(selected.as_ptr() as usize - value.as_ptr() as usize).ok()?;
            pieces.push(ExpressionSourcePiece {
                expression_span: Span::new(start, end),
                original: ExecutedScriptSource::from_word(
                    parent.clone(),
                    index,
                    word,
                    value,
                    config,
                ),
                trimmed,
            });
        }
        if values.len() > 1
            && text != tcl_syntax::list::concat_values(values.iter().map(String::as_str))
        {
            return None;
        }
        let text: Arc<str> = Arc::from(text);
        let origin = Arc::new(SourceOriginId::derived(
            parent,
            (0..values.len()).collect(),
            &Arc::clone(&text),
            MaterialisedSourceKind::Expression,
        ));
        Some(Self {
            text,
            origin,
            pieces,
            config,
        })
    }

    /// Original source site of a parser variable wholly within unchanged bytes.
    /// A decoded, captured or cross-operand reference retains no authored site.
    #[must_use]
    pub fn variable_source(
        &self,
        node: &crate::expr_ast::ExprNode,
    ) -> Option<(&Arc<SourceOriginId>, SourceSite)> {
        let crate::expr_ast::ExprNode::Var {
            text, start, end, ..
        } = node
        else {
            return None;
        };
        if self
            .text
            .get(usize::try_from(*start).ok()?..usize::try_from(end.checked_add(1)?).ok()?)
            != Some(text)
        {
            return None;
        }
        let span = node.variable_source_span(0, self.config)?;
        let piece = self.pieces.iter().find(|piece| {
            piece.expression_span.start() <= span.start()
                && span.end() <= piece.expression_span.end()
        })?;
        let ExecutedScriptMapping::Contiguous { base } = piece.original.mapping else {
            return None;
        };
        let base = base.checked_add(piece.trimmed)?;
        let start = base.checked_add(span.start().checked_sub(piece.expression_span.start())?)?;
        let end = base.checked_add(span.end().checked_sub(piece.expression_span.start())?)?;
        Some((
            &piece.original.origin,
            SourceSite {
                span: Span::new(start, end),
                provenance: Provenance::Source,
            },
        ))
    }
}

impl super::SourceCommandBindings {
    pub(super) fn walk_mapped_expression(
        &mut self,
        offset: usize,
        values: &[&str],
        operands: super::SourceScriptOperands<'_>,
        state: &mut super::ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
    ) -> super::SourceOutcomes {
        let Some(origin) = state.current_source_origin.clone() else {
            return super::opaque_source_invocation(state);
        };
        let parent = CommandAllocationSite {
            source: origin,
            offset: context.invocation_offset,
        };
        let words = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                operands
                    .written_word(offset + index)
                    .cloned()
                    .unwrap_or_else(|| WordExpr::Literal {
                        text: (*value).to_owned(),
                        source: SourceSite {
                            span: Span::empty(0),
                            provenance: Provenance::Opaque,
                        },
                    })
            })
            .collect::<Vec<_>>();
        let Some(expression) = operands.arguments.dialect().and_then(|dialect| {
            ExecutedExpressionSource::from_arguments(
                parent.clone(),
                &words,
                &values
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect::<Vec<_>>(),
                dialect,
            )
        }) else {
            return super::opaque_source_invocation(state);
        };
        let owner = super::SourceVariableEvaluationOwner::NativeExpression {
            invocation: parent.clone(),
            parent: context.variable_read_owner.cloned().map(Arc::new),
        };
        let previous = state
            .current_source_origin
            .replace(Arc::clone(&expression.origin));
        let mut outcomes = self.walk_prepared_expression(
            parent,
            ExecutedScriptSource {
                text: tcl_lexer::SourceImage::native(Arc::<[u8]>::from(expression.text.as_bytes())),
                origin: Arc::clone(&expression.origin),
                mapping: ExecutedScriptMapping::Materialised,
            },
            state,
            super::SourceExecutionContext {
                variable_read_owner: Some(&owner),
                expression_source: Some(&expression),
                ..context
            },
        );
        outcomes.restore_source_origin(previous.as_ref());
        outcomes.publish(state);
        outcomes
    }

    pub(super) fn walk_expression_variable(
        &mut self,
        node: &crate::expr_ast::ExprNode,
        text: &str,
        start: u32,
        base: u32,
        state: &mut super::ModuleCommandBindings,
        context: super::SourceExecutionContext<'_>,
    ) -> super::SourceOutcomes {
        if let Some((origin, source, original)) = context
            .expression_source
            .and_then(|proof| proof.variable_source(node))
            .and_then(|(origin, source)| {
                origin
                    .try_text()
                    .ok()
                    .map(|original| (origin, source, original))
            })
        {
            let previous = state.current_source_origin.replace(Arc::clone(origin));
            let mut outcomes =
                self.observe_variable_substitution(text, &source, original, 0, state, context);
            outcomes.restore_source_origin(previous.as_ref());
            outcomes.publish(state);
            return outcomes;
        }
        self.observe_variable_substitution(
            text,
            &SourceSite::source(Span::new(
                base.saturating_add(start),
                base.saturating_add(start).saturating_add(
                    u32::try_from(text.len().saturating_sub(1)).unwrap_or(u32::MAX),
                ),
            )),
            text,
            base.saturating_add(start),
            state,
            context,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expression(source: &str, values: &[&str]) -> ExecutedExpressionSource {
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let command = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .next()
            .unwrap();
        let words = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &command,
        );
        let parent = CommandAllocationSite {
            source: Arc::new(SourceOriginId::authored(&Arc::from(source))),
            offset: 0,
        };
        ExecutedExpressionSource::from_arguments(
            parent,
            &words.words()[1..],
            &values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>(),
            dialect,
        )
        .unwrap()
    }

    #[test]
    fn original_expression_mapping_requires_one_unchanged_written_operand() {
        let single = expression("expr {$left + 1}", &["$left + 1"]);
        let (_, base) = single.original_expression_mapping().unwrap();
        assert_eq!(base, 6);
        let concatenated = expression("expr {$left} + 1", &["$left", "+", "1"]);
        assert!(concatenated.original_expression_mapping().is_none());
        let captured = expression("expr $stored", &["$left + 1"]);
        assert!(captured.original_expression_mapping().is_none());
    }

    #[test]
    fn concatenation_keeps_each_variables_original_source_site() {
        let source = "expr {  $left  } + { $right }";
        let expression = expression(source, &["  $left  ", "+", " $right "]);
        assert_eq!(&*expression.text, "$left + $right");
        let tree = tcl_syntax::expr::parser::parse_expr(&expression.text, None);
        let crate::expr_ast::ExprNode::Binary { left, right, .. } = tree else {
            panic!("binary expression");
        };
        for (node, spelling) in [(&*left, "$left"), (&*right, "$right")] {
            let (origin, site) = expression.variable_source(node).unwrap();
            assert_ne!(origin, &expression.origin);
            assert_eq!(&source[site.span.as_range()], spelling);
        }
    }

    #[test]
    fn decoded_and_cross_operand_references_have_no_authored_read_site() {
        let decoded = expression(r#"expr "\x24x" + 1"#, &["$x", "+", "1"]);
        let tree = tcl_syntax::expr::parser::parse_expr(&decoded.text, None);
        let crate::expr_ast::ExprNode::Binary { left, .. } = tree else {
            panic!("binary expression");
        };
        assert!(decoded.variable_source(&left).is_none());
        let split = expression("expr {$} {x}", &["$", "x"]);
        let forged = crate::expr_ast::ExprNode::Var {
            text: "$x".to_owned(),
            name: "x".to_owned(),
            start: 0,
            end: 1,
        };
        assert!(split.variable_source(&forged).is_none());
        let empty = expression("expr {} {1}", &["", "1"]);
        assert_eq!(&*empty.text, "1");
    }
}
