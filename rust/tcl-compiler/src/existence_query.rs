// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook/tcl-lsp>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-resolved variable-existence query recognition.
//!
//! The parser owns expression structure, while the registry owns which
//! invocation denotes an existence operation. Rooted spellings such as
//! `::info exists name` therefore follow the same resolved invocation as
//! every other compiler consumer.

use crate::expr_ast::{ExprNode, UnaryOp};

/// The fact an existence query asks about a variable name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExistenceKind {
    /// The name is bound to either a scalar or an array.
    AnyVariable,
    /// The name is bound specifically to an array.
    Array,
}

/// A registry-resolved command-substitution existence query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExistenceQuery {
    /// The queried name, exactly as written.
    pub(crate) var: String,
    /// Whether the containing condition negates the query.
    pub(crate) negated: bool,
    /// The query's registry-owned semantic distinction.
    pub(crate) kind: ExistenceKind,
}

/// Recognise one expression condition as a registry-owned existence query.
#[must_use]
pub(crate) fn in_expr(
    node: &ExprNode,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<ExistenceQuery> {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => in_expr(operand, registry, config).map(|query| ExistenceQuery {
            negated: !query.negated,
            ..query
        }),
        ExprNode::Command { text, .. } => {
            in_text(text, registry, config).map(|(var, kind)| ExistenceQuery {
                var,
                negated: false,
                kind,
            })
        }
        _ => None,
    }
}

/// Recognise one bracketed command substitution as an existence query.
#[must_use]
pub(crate) fn in_text(
    text: &str,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, ExistenceKind)> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial {
        return None;
    }
    let words: Vec<&str> = command.texts.iter().map(String::as_str).collect();
    let (head, args) = words.split_first()?;
    let [_subcommand, variable] = args else {
        return None;
    };
    let operation = registry
        .resolve_invocation(head, args, registry.own_surface_query())?
        .semantics
        .operation;
    let kind = match operation {
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::InfoExists) => {
            ExistenceKind::AnyVariable
        }
        tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ArrayExists) => {
            ExistenceKind::Array
        }
        _ => return None,
    };
    Some(((*variable).to_owned(), kind))
}

#[cfg(test)]
mod tests {
    use super::{ExistenceKind, in_text};

    #[test]
    fn rooted_core_existence_queries_resolve_by_operation() {
        let registry = tcl_registry::CommandRegistry::build_default();
        assert_eq!(
            in_text(
                "[::info exists name]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            Some(("name".to_owned(), ExistenceKind::AnyVariable))
        );
        assert_eq!(
            in_text(
                "[::array exists name]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            Some(("name".to_owned(), ExistenceKind::Array))
        );
    }

    #[test]
    fn existence_words_follow_the_exact_document_grammar() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let irules = tcl_lexer::LexerConfig::for_dialect("f5-irules");
        assert_eq!(
            in_text("[info {exists}{name}]", &registry, irules),
            Some(("name".to_owned(), ExistenceKind::AnyVariable)),
        );
        assert_eq!(
            in_text(
                "[info {exists}{name}]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            None,
        );
        assert_eq!(
            in_text(
                "[info exists {name with spaces}]",
                &registry,
                tcl_lexer::LexerConfig::default(),
            ),
            Some(("name with spaces".to_owned(), ExistenceKind::AnyVariable,)),
        );
    }
}
