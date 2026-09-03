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

//! Structured AST for Tcl `[expr]` expressions.
//!
//! Replaces the opaque `String` representation used in the compiler's
//! `Statement::AssignExpr.expr`, `IfClause.condition`,
//! `Statement::For.condition`, and `Terminator::Branch.condition`.
//! Parsed once at lowering time, then walked by downstream analyses
//! (SSA, SCCP, type inference, shimmer).
//!
//! The [`ExprNode::Raw`] variant is a fallback for any expression the
//! parser cannot handle — every consumer must treat it as "give up,
//! use the string".

use std::collections::HashSet;
use std::fmt;

use tcl_lexer::{Lexer, SourceMap, TokenType};

use crate::naming::normalise_var_name;

/// Character offset within expression source text.
pub type ExprOffset = u32;

// Binary operators

/// Binary operators in Tcl expressions.
///
/// Covers standard Tcl arithmetic, comparison, logical, bitwise, and
/// string operators, plus iRules-specific extensions (`contains`,
/// `starts_with`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `**`
    Pow,

    // Shift
    /// `<<`
    LShift,
    /// `>>`
    RShift,

    // Bitwise
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,

    // Logical
    /// `&&`
    And,
    /// `||`
    Or,

    // Numeric comparison
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,

    // String comparison
    /// `eq`
    StrEq,
    /// `ne`
    StrNe,
    /// `lt`
    StrLt,
    /// `le`
    StrLe,
    /// `gt`
    StrGt,
    /// `ge`
    StrGe,

    // List membership
    /// `in`
    In,
    /// `ni`
    Ni,

    // iRules word-based logical operators
    /// `and`
    WordAnd,
    /// `or`
    WordOr,

    // iRules string comparison operators
    /// `contains`
    Contains,
    /// `starts_with`
    StartsWith,
    /// `ends_with`
    EndsWith,
    /// `equals`
    StrEquals,
    /// `matches`
    Matches,
    /// `matches_glob`
    MatchesGlob,
    /// `matches_regex`
    MatchesRegex,
}

impl BinOp {
    /// Return the source-text representation of this operator.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::Pow => "**",
            Self::LShift => "<<",
            Self::RShift => ">>",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::And => "&&",
            Self::Or => "||",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::StrEq => "eq",
            Self::StrNe => "ne",
            Self::StrLt => "lt",
            Self::StrLe => "le",
            Self::StrGt => "gt",
            Self::StrGe => "ge",
            Self::In => "in",
            Self::Ni => "ni",
            Self::WordAnd => "and",
            Self::WordOr => "or",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::StrEquals => "equals",
            Self::Matches => "matches",
            Self::MatchesGlob => "matches_glob",
            Self::MatchesRegex => "matches_regex",
        }
    }

    /// Operator precedence (higher = tighter binding).
    ///
    /// Used by [`render_expr`] to
    /// determine when parentheses are needed.
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Or | Self::WordOr => 1,
            Self::And | Self::WordAnd => 2,
            Self::BitOr => 3,
            Self::BitXor => 4,
            Self::BitAnd => 5,
            Self::Eq
            | Self::Ne
            | Self::StrEq
            | Self::StrNe
            | Self::In
            | Self::Ni
            | Self::Contains
            | Self::StartsWith
            | Self::EndsWith
            | Self::StrEquals
            | Self::Matches
            | Self::MatchesGlob
            | Self::MatchesRegex => 6,
            Self::Lt
            | Self::Le
            | Self::Gt
            | Self::Ge
            | Self::StrLt
            | Self::StrLe
            | Self::StrGt
            | Self::StrGe => 7,
            Self::LShift | Self::RShift => 8,
            Self::Add | Self::Sub => 9,
            Self::Mul | Self::Div | Self::Mod => 10,
            Self::Pow => 11,
        }
    }

    /// Whether this operator is right-associative.
    #[must_use]
    pub const fn is_right_assoc(self) -> bool {
        matches!(self, Self::Pow)
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// Unary operators

/// Unary operators in Tcl expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `-` (arithmetic negation)
    Neg,
    /// `+` (arithmetic identity)
    Pos,
    /// `~` (bitwise complement)
    BitNot,
    /// `!` (logical NOT)
    Not,
    /// `not` (iRules word-based NOT)
    WordNot,
}

impl UnaryOp {
    /// Return the source-text representation of this operator.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::BitNot => "~",
            Self::Not => "!",
            Self::WordNot => "not",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// AST nodes

/// An expression AST node.
///
/// This is a sum type covering every expression construct the parser
/// can produce, plus [`Self::Raw`] as the "give up" fallback. Expression
/// trees are immutable once built (no interior mutability). Recursive
/// children are `Box<ExprNode>` to keep the enum size bounded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExprNode {
    /// Integer, float, or boolean literal.
    Literal {
        /// Source text of the literal.
        text: String,
        /// Start offset within expression text.
        start: ExprOffset,
        /// End offset within expression text.
        end: ExprOffset,
    },

    /// Quoted or braced string literal (`"..."` or `{...}`).
    String {
        /// Source text including delimiters.
        text: String,
        /// Start offset within expression text.
        start: ExprOffset,
        /// End offset within expression text.
        end: ExprOffset,
    },

    /// Variable reference (`$var`, `${var}`, `$arr(idx)`).
    Var {
        /// Full text including `$`.
        text: String,
        /// Normalised base name.
        name: String,
        /// Start offset within expression text.
        start: ExprOffset,
        /// End offset within expression text.
        end: ExprOffset,
    },

    /// Command substitution `[cmd ...]` — opaque boundary.
    Command {
        /// Full text including brackets.
        text: String,
        /// Start offset within expression text.
        start: ExprOffset,
        /// End offset within expression text.
        end: ExprOffset,
    },

    /// Binary operator application.
    Binary {
        /// The operator.
        op: BinOp,
        /// Left operand.
        left: Box<ExprNode>,
        /// Right operand.
        right: Box<ExprNode>,
    },

    /// Unary operator application.
    Unary {
        /// The operator.
        op: UnaryOp,
        /// The operand.
        operand: Box<ExprNode>,
    },

    /// Ternary conditional `cond ? true_val : false_val`.
    Ternary {
        /// Condition expression.
        condition: Box<ExprNode>,
        /// Value when condition is true.
        true_branch: Box<ExprNode>,
        /// Value when condition is false.
        false_branch: Box<ExprNode>,
    },

    /// Math function call: `sin($x)`, `int($y)`, `max($a, $b)`.
    Call {
        /// Function name.
        function: String,
        /// Arguments.
        args: Vec<ExprNode>,
        /// Start offset within expression text.
        start: ExprOffset,
        /// End offset within expression text.
        end: ExprOffset,
    },

    /// Fallback: unparseable expression preserved as raw text.
    ///
    /// Every consumer must handle this as "give up" — returning the
    /// same result as the old string-based analysis.
    Raw {
        /// Original expression text.
        text: String,
    },
}

impl ExprNode {
    /// Recursively extract variable names from this expression AST.
    ///
    /// This is the structured replacement for the scattered
    /// `tokenise_expr()` -> scan-for-variables patterns.
    ///
    /// The `Raw` fallback text is re-lexed under the **default** grammar:
    /// this form is for callers that genuinely carry no document grammar.
    /// A caller that has one must use [`Self::vars_with_grammar`].
    /// Test-only: the default-grammar form. Production callers hold the
    /// document's grammar and use [`Self::vars_with_grammar`].
    #[cfg(test)]
    #[must_use]
    pub fn vars(&self) -> HashSet<String> {
        self.vars_with_grammar(tcl_dialect::LexerGrammar::default()) // dialect-drift-ok: #[cfg(test)] convenience
    }

    /// [`Self::vars`] with the document's grammar, used to re-lex the `Raw`
    /// fallback text (unparsed Tcl preserved verbatim) the way the rest of
    /// the document is read.
    #[must_use]
    pub fn vars_with_grammar(&self, grammar: tcl_dialect::LexerGrammar) -> HashSet<String> {
        let mut result = HashSet::new();
        self.collect_vars(grammar, &mut result);
        result
    }

    /// Every direct variable-reference range in this parsed expression.
    ///
    /// Offsets retain the expression lexer's inclusive-end convention. A
    /// command substitution remains an opaque Tcl-script boundary: variables
    /// inside it belong to the script walker that executes that command, not
    /// to the surrounding expression. Quoted-string substitutions are exposed
    /// separately by [`Self::quoted_string_spans`], because their Tcl word
    /// grammar needs the resolved lexer profile.
    #[must_use]
    pub fn variable_spans(&self) -> Vec<(ExprOffset, ExprOffset)> {
        let mut out = Vec::new();
        self.collect_variable_spans(&mut out);
        out
    }

    /// Every double-quoted string operand's inclusive source range.
    ///
    /// A quoted operand is evaluated as a Tcl substitution context by
    /// `expr`; a braced operand is not. The expression lexer intentionally
    /// keeps both forms as opaque strings, so the profile-aware substitution
    /// owner uses these raw source ranges to re-enter only quoted interiors
    /// through the shared Tcl lexer.
    #[must_use]
    pub fn quoted_string_spans(&self) -> Vec<(ExprOffset, ExprOffset)> {
        let mut out = Vec::new();
        self.collect_quoted_string_spans(&mut out);
        out
    }

    /// Collect the raw text of every command substitution in this expr AST
    /// (each an `[cmd …]` form, brackets included).  Used to recover the side
    /// effects / reads of command substitutions evaluated inside an
    /// expression — they run in the current scope.
    #[must_use]
    pub fn command_texts(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_command_texts(&mut out);
        out
    }

    /// Every math-function application in this expression, innermost included,
    /// as `(name, name_start_offset, arg_count)`.  A `sin($x)` dispatches to
    /// the command `::tcl::mathfunc::sin`, so a consumer maps the offset back
    /// to source to reach that command (definition, references, arity) and to
    /// gate the function against the dialect's available set.  The offset is
    /// the function-name token's start within the expression text this node
    /// was parsed from; the name is returned verbatim because mathfunc lookup
    /// is case-sensitive.
    #[must_use]
    pub fn function_calls(&self) -> Vec<(&str, ExprOffset, usize)> {
        let mut out = Vec::new();
        self.collect_function_calls(&mut out);
        out
    }

    fn collect_function_calls<'a>(&'a self, out: &mut Vec<(&'a str, ExprOffset, usize)>) {
        match self {
            Self::Call {
                function,
                args,
                start,
                ..
            } => {
                out.push((function.as_str(), *start, args.len()));
                for arg in args {
                    arg.collect_function_calls(out);
                }
            }
            Self::Binary { left, right, .. } => {
                left.collect_function_calls(out);
                right.collect_function_calls(out);
            }
            Self::Unary { operand, .. } => operand.collect_function_calls(out),
            Self::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                condition.collect_function_calls(out);
                true_branch.collect_function_calls(out);
                false_branch.collect_function_calls(out);
            }
            // A `[cmd …]` substitution is an opaque boundary (its own commands
            // are walked at the script level); literals, strings, variables,
            // and unparsed fallback text hold no function calls.
            Self::Command { .. }
            | Self::Literal { .. }
            | Self::String { .. }
            | Self::Var { .. }
            | Self::Raw { .. } => {}
        }
    }

    fn collect_command_texts(&self, out: &mut Vec<String>) {
        match self {
            Self::Command { text, .. } => {
                if !text.is_empty() {
                    out.push(text.clone());
                }
            }
            Self::Binary { left, right, .. } => {
                left.collect_command_texts(out);
                right.collect_command_texts(out);
            }
            Self::Unary { operand, .. } => operand.collect_command_texts(out),
            Self::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                condition.collect_command_texts(out);
                true_branch.collect_command_texts(out);
                false_branch.collect_command_texts(out);
            }
            Self::Call { args, .. } => {
                for arg in args {
                    arg.collect_command_texts(out);
                }
            }
            Self::Var { .. } | Self::Literal { .. } | Self::String { .. } | Self::Raw { .. } => {}
        }
    }

    fn collect_variable_spans(&self, out: &mut Vec<(ExprOffset, ExprOffset)>) {
        match self {
            Self::Var { start, end, .. } => out.push((*start, *end)),
            Self::Binary { left, right, .. } => {
                left.collect_variable_spans(out);
                right.collect_variable_spans(out);
            }
            Self::Unary { operand, .. } => operand.collect_variable_spans(out),
            Self::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                condition.collect_variable_spans(out);
                true_branch.collect_variable_spans(out);
                false_branch.collect_variable_spans(out);
            }
            Self::Call { args, .. } => {
                for arg in args {
                    arg.collect_variable_spans(out);
                }
            }
            Self::Command { .. }
            | Self::Literal { .. }
            | Self::String { .. }
            | Self::Raw { .. } => {}
        }
    }

    fn collect_quoted_string_spans(&self, out: &mut Vec<(ExprOffset, ExprOffset)>) {
        match self {
            Self::String { text, start, end } if text.starts_with('"') => {
                out.push((*start, *end));
            }
            Self::Binary { left, right, .. } => {
                left.collect_quoted_string_spans(out);
                right.collect_quoted_string_spans(out);
            }
            Self::Unary { operand, .. } => operand.collect_quoted_string_spans(out),
            Self::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                condition.collect_quoted_string_spans(out);
                true_branch.collect_quoted_string_spans(out);
                false_branch.collect_quoted_string_spans(out);
            }
            Self::Call { args, .. } => {
                for arg in args {
                    arg.collect_quoted_string_spans(out);
                }
            }
            Self::Command { .. }
            | Self::Literal { .. }
            | Self::String { .. }
            | Self::Var { .. }
            | Self::Raw { .. } => {}
        }
    }

    /// Recursive variable collection helper.
    fn collect_vars(&self, grammar: tcl_dialect::LexerGrammar, out: &mut HashSet<String>) {
        self.collect_vars_with(grammar, &|_text, name| name.to_owned(), out);
    }

    /// Test-only: the default-grammar form. Production callers hold the
    /// document's grammar and use the `_with_grammar` sibling.
    #[cfg(test)]
    /// Every variable read in this expression, **element-qualified**: a
    /// constant-keyed array element reports as its own variable `base(key)`,
    /// a dynamic one as the bare base — the SSA-side counterpart of
    /// [`Self::vars`], which normalises every reference to its base.
    /// The `Raw` fallback text is re-lexed under the **default** grammar:
    /// this form is for callers that genuinely carry no document grammar.
    /// A caller that has one must use
    /// [`Self::vars_element_qualified_with_grammar`].
    #[must_use]
    pub fn vars_element_qualified(&self) -> HashSet<String> {
        self.vars_element_qualified_with_grammar(tcl_dialect::LexerGrammar::default()) // dialect-drift-ok: #[cfg(test)] convenience
    }

    /// [`Self::vars_element_qualified`] with the document's grammar, used to
    /// re-lex the `Raw` fallback text.
    #[must_use]
    pub fn vars_element_qualified_with_grammar(
        &self,
        grammar: tcl_dialect::LexerGrammar,
    ) -> HashSet<String> {
        let mut result = HashSet::new();
        self.collect_vars_with(
            grammar,
            &|text, _name| crate::naming::element_var_name(text).to_owned(),
            &mut result,
        );
        result
    }

    /// The shared walk under [`Self::vars`] / [`Self::vars_element_qualified`]:
    /// `pick` maps a `Var` node's `(text, name)` to the reported name.
    fn collect_vars_with(
        &self,
        grammar: tcl_dialect::LexerGrammar,
        pick: &dyn Fn(&str, &str) -> String,
        out: &mut HashSet<String>,
    ) {
        match self {
            Self::Var { text, name, .. } => {
                let picked = pick(text, name);
                if !picked.is_empty() {
                    out.insert(picked);
                }
            }
            Self::Binary { left, right, .. } => {
                left.collect_vars_with(grammar, pick, out);
                right.collect_vars_with(grammar, pick, out);
            }
            Self::Unary { operand, .. } => {
                operand.collect_vars_with(grammar, pick, out);
            }
            Self::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                condition.collect_vars_with(grammar, pick, out);
                true_branch.collect_vars_with(grammar, pick, out);
                false_branch.collect_vars_with(grammar, pick, out);
            }
            Self::Call { args, .. } => {
                for arg in args {
                    arg.collect_vars_with(grammar, pick, out);
                }
            }
            // Command substitutions may contain variable references,
            // but extracting them requires script-level analysis that
            // lives in the SSA module. At the pure-AST level we stop
            // at the command boundary.
            Self::Command { .. } | Self::Literal { .. } | Self::String { .. } => {}
            // Raw fallback text is unparsed Tcl (e.g. a `switch -- $col`
            // subject preserved verbatim). Re-lex it and collect
            // top-level `$var` references so liveness / unused-parameter
            // detection (W214) sees the read. Nested vars inside command
            // substitutions are left to the SSA layer, matching the
            // `Self::Command` policy above.
            Self::Raw { text } => collect_raw_vars_with(text, grammar, pick, out),
        }
    }
}

/// Re-lex raw fallback text and collect top-level `$var` references through
/// `pick` (the same `(text, base-name)` mapping as the AST walk).
fn collect_raw_vars_with(
    text: &str,
    grammar: tcl_dialect::LexerGrammar,
    pick: &dyn Fn(&str, &str) -> String,
    out: &mut HashSet<String>,
) {
    let source_map = SourceMap::new(text);
    let Ok(tokens) =
        Lexer::with_config(text, tcl_lexer::LexerConfig::from_grammar(grammar)).tokenise_all()
    else {
        return;
    };
    for tok in &tokens {
        if tok.kind == TokenType::Var {
            let raw = source_map.token_text(*tok);
            let picked = pick(raw, normalise_var_name(raw));
            if !picked.is_empty() {
                out.insert(picked);
            }
        }
    }
}

// Rendering

/// Return `true` when a unary operator's operand needs wrapping in parens.
fn needs_parens_for_unary(operand: &ExprNode) -> bool {
    matches!(operand, ExprNode::Binary { .. } | ExprNode::Ternary { .. })
}

/// Return `true` when a binary child needs parentheses to preserve semantics.
fn needs_parens_for_binary_child(parent_op: BinOp, child: &ExprNode, is_right: bool) -> bool {
    // A ternary (`?:`) has lower precedence than every binary operator, so as a
    // child of a binary op it ALWAYS needs parentheses — on either side.
    // `($a ? 1 : 2) + 3` must not render as `$a ? 1 : 2 + 3` (re-parses as
    // `$a ? 1 : (2 + 3)`); `1 + ($a ? 2 : 3)` must not render as
    // `1 + $a ? 2 : 3` (re-parses as `(1 + $a) ? 2 : 3`).
    if matches!(child, ExprNode::Ternary { .. }) {
        return true;
    }
    let ExprNode::Binary { op: child_op, .. } = child else {
        return false;
    };

    let parent_prec = parent_op.precedence();
    let child_prec = child_op.precedence();

    if child_prec < parent_prec {
        return true;
    }
    if child_prec == parent_prec {
        if parent_op.is_right_assoc() {
            // Right-associative: parenthesise the left child, not the right.
            return !is_right;
        }
        // Left-associative (default): parenthesise the right child.
        return is_right;
    }
    false
}

/// Round-trip an [`ExprNode`] back to source text.
///
/// Inserts parentheses where needed to preserve operator precedence
/// and associativity.
#[must_use]
pub fn render_expr(node: &ExprNode) -> String {
    match node {
        ExprNode::Literal { text, .. }
        | ExprNode::String { text, .. }
        | ExprNode::Var { text, .. }
        | ExprNode::Command { text, .. }
        | ExprNode::Raw { text } => text.clone(),

        ExprNode::Binary {
            op, left, right, ..
        } => {
            let mut left_text = render_expr(left);
            let mut right_text = render_expr(right);
            if needs_parens_for_binary_child(*op, left, false) {
                left_text = format!("({left_text})");
            }
            if needs_parens_for_binary_child(*op, right, true) {
                right_text = format!("({right_text})");
            }
            format!("{left_text} {op} {right_text}")
        }

        ExprNode::Unary { op, operand } => {
            let prefix = op.as_str();
            let mut inner = render_expr(operand);
            if needs_parens_for_unary(operand) {
                inner = format!("({inner})");
            }
            // Word operators like `not` need a space before the operand.
            if prefix
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
            {
                format!("{prefix} {inner}")
            } else {
                format!("{prefix}{inner}")
            }
        }

        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            format!(
                "{} ? {} : {}",
                render_expr(condition),
                render_expr(true_branch),
                render_expr(false_branch)
            )
        }

        ExprNode::Call { function, args, .. } => {
            let arg_text: Vec<String> = args.iter().map(render_expr).collect();
            format!("{function}({})", arg_text.join(", "))
        }
    }
}

/// Get the string form of an expression.
///
/// For [`ExprNode::Raw`] returns the original text directly; for structured
/// nodes, renders them via [`render_expr`].
#[must_use]
pub fn expr_text(node: &ExprNode) -> String {
    match node {
        ExprNode::Raw { text } => text.clone(),
        _ => render_expr(node),
    }
}

/// Which command asked an existence question.
///
/// The two spellings are *not* interchangeable when the queried name is
/// known to be a scalar: `info exists` answers "is this name bound",
/// `array exists` answers "is this name bound **to an array**".  A consumer
/// that folds the query to a constant has to know which one it is looking
/// at (issue #1239).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceCommand {
    /// `info exists NAME` — true for any bound name, scalar or array.
    Info,
    /// `array exists NAME` — true only when `NAME` is bound to an array.
    Array,
}

/// A recognised `[info exists X]` / `[array exists X]` existence query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistenceQuery {
    /// The queried variable name, exactly as written.
    pub var: String,
    /// True for `![info exists X]` — the condition is the query's negation.
    pub negated: bool,
    /// Which of the two spellings asked.
    pub command: ExistenceCommand,
}

/// Recognise an `[info exists X]` / `[array exists X]` existence-query
/// condition.  Used to inject guarded-region read narrowing
/// (`analyser::diagnostics::helpers::collect_existence_guards`), suppress
/// the existence-query word's W210 read,
/// and fold the predicate to a constant (analyser I230).
///
/// Only the simple two-/three-word command-substitution form is matched
/// (e.g. `[info exists name]`); anything embedded in a larger expression
/// returns `None`.
#[must_use]
pub fn existence_query_var(node: &ExprNode) -> Option<ExistenceQuery> {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => existence_query_var(operand).map(|q| ExistenceQuery {
            negated: !q.negated,
            ..q
        }),
        ExprNode::Command { text, .. } => {
            existence_query_in_text(text).map(|(var, command)| ExistenceQuery {
                var,
                negated: false,
                command,
            })
        }
        _ => None,
    }
}

/// Parse a bracketed command-substitution `text` (e.g. `"[info exists
/// x]"`) and return the queried variable plus the spelling that asked, when
/// it is exactly `info exists NAME` or `array exists NAME`.
#[must_use]
pub fn existence_query_in_text(text: &str) -> Option<(String, ExistenceCommand)> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let words: Vec<&str> = inner.split_whitespace().collect();
    if words.len() != 3 || words[1] != "exists" {
        return None;
    }
    let command = match words[0] {
        "info" => ExistenceCommand::Info,
        "array" => ExistenceCommand::Array,
        _ => return None,
    };
    Some((words[2].to_owned(), command))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_construction() {
        let node = ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        };
        assert_eq!(render_expr(&node), "42");
    }

    #[test]
    fn var_construction_and_vars() {
        let node = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        let vars = node.vars();
        assert_eq!(vars.len(), 1);
        assert!(vars.contains("x"));
    }

    #[test]
    fn empty_var_name_not_collected() {
        let node = ExprNode::Var {
            text: "$".into(),
            name: String::new(),
            start: 0,
            end: 1,
        };
        assert!(node.vars().is_empty());
    }

    #[test]
    fn binary_render() {
        let node = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 4,
                end: 5,
            }),
        };
        assert_eq!(render_expr(&node), "1 + 2");
    }

    #[test]
    fn ternary_child_of_binary_is_parenthesised() {
        use crate::expr::parser::parse_expr;
        // A ternary child of a binary op must round-trip with
        // parens so it re-parses identically.
        for src in [
            "($a ? 1 : 2) + 3",
            "3 + ($a ? 1 : 2)",
            "($a ? 1 : 2) * ($b ? 3 : 4)",
        ] {
            let once = render_expr(&parse_expr(src, None));
            let twice = render_expr(&parse_expr(&once, None));
            assert_eq!(once, twice, "render must be stable for {src:?}");
            assert!(
                once.contains('('),
                "ternary child must keep parens: {src:?} → {once:?}",
            );
        }
        // Structural check: the rendered form must re-parse to a Binary at the
        // top (not a Ternary), proving the ternary stayed grouped.
        let node = parse_expr(&render_expr(&parse_expr("($a ? 1 : 2) + 3", None)), None);
        assert!(
            matches!(node, ExprNode::Binary { op: BinOp::Add, .. }),
            "top node must remain `+`, got {node:?}",
        );
    }

    #[test]
    fn binary_precedence_parens() {
        // (1 + 2) * 3 — the addition must be parenthesised
        let add = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 4,
                end: 5,
            }),
        };
        let mul = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(add),
            right: Box::new(ExprNode::Literal {
                text: "3".into(),
                start: 9,
                end: 10,
            }),
        };
        assert_eq!(render_expr(&mul), "(1 + 2) * 3");
    }

    #[test]
    fn right_assoc_pow() {
        // 2 ** 3 ** 4 → 2 ** 3 ** 4 (right-associative, no parens on right)
        let inner_pow = ExprNode::Binary {
            op: BinOp::Pow,
            left: Box::new(ExprNode::Literal {
                text: "3".into(),
                start: 5,
                end: 6,
            }),
            right: Box::new(ExprNode::Literal {
                text: "4".into(),
                start: 10,
                end: 11,
            }),
        };
        let outer_pow = ExprNode::Binary {
            op: BinOp::Pow,
            left: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(inner_pow),
        };
        assert_eq!(render_expr(&outer_pow), "2 ** 3 ** 4");
    }

    #[test]
    fn unary_render() {
        let node = ExprNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(ExprNode::Literal {
                text: "5".into(),
                start: 1,
                end: 2,
            }),
        };
        assert_eq!(render_expr(&node), "-5");
    }

    #[test]
    fn unary_parens_for_binary_operand() {
        let add = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            }),
            right: Box::new(ExprNode::Literal {
                text: "2".into(),
                start: 4,
                end: 5,
            }),
        };
        let neg = ExprNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(add),
        };
        assert_eq!(render_expr(&neg), "-(1 + 2)");
    }

    #[test]
    fn word_not_has_space() {
        let node = ExprNode::Unary {
            op: UnaryOp::WordNot,
            operand: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 4,
                end: 5,
            }),
        };
        assert_eq!(render_expr(&node), "not 1");
    }

    #[test]
    fn ternary_render() {
        let node = ExprNode::Ternary {
            condition: Box::new(ExprNode::Var {
                text: "$x".into(),
                name: "x".into(),
                start: 0,
                end: 2,
            }),
            true_branch: Box::new(ExprNode::Literal {
                text: "1".into(),
                start: 5,
                end: 6,
            }),
            false_branch: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 9,
                end: 10,
            }),
        };
        assert_eq!(render_expr(&node), "$x ? 1 : 0");
    }

    #[test]
    fn call_render() {
        let node = ExprNode::Call {
            function: "max".into(),
            args: vec![
                ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 4,
                    end: 6,
                },
                ExprNode::Var {
                    text: "$b".into(),
                    name: "b".into(),
                    start: 8,
                    end: 10,
                },
            ],
            start: 0,
            end: 11,
        };
        assert_eq!(render_expr(&node), "max($a, $b)");
    }

    #[test]
    fn raw_fallback() {
        let node = ExprNode::Raw {
            text: "some complex thing".into(),
        };
        assert_eq!(expr_text(&node), "some complex thing");
    }

    #[test]
    fn raw_node_collects_top_level_vars() {
        // A `switch -- $col` subject is preserved as Raw text; the
        // var scan must recover `$col` (and array / braced forms) so
        // W214 doesn't flag the parameter as unused.
        let node = ExprNode::Raw {
            text: "$col $arr(idx) ${ns::name}".into(),
        };
        let vars = node.vars();
        assert!(vars.contains("col"), "got {vars:?}");
        assert!(vars.contains("arr"), "got {vars:?}");
        assert!(vars.contains("ns::name"), "got {vars:?}");
    }

    #[test]
    fn raw_node_stops_at_command_substitution() {
        // Vars nested inside a command substitution belong to the SSA
        // layer, not the pure-AST scan — matching `Self::Command`.
        let node = ExprNode::Raw {
            text: "[incr counter]".into(),
        };
        assert!(node.vars().is_empty(), "got {:?}", node.vars());
    }

    #[test]
    fn vars_in_nested_expr() {
        // $x + sin($y) ? $z : 0
        let node = ExprNode::Ternary {
            condition: Box::new(ExprNode::Binary {
                op: BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$x".into(),
                    name: "x".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Call {
                    function: "sin".into(),
                    args: vec![ExprNode::Var {
                        text: "$y".into(),
                        name: "y".into(),
                        start: 8,
                        end: 10,
                    }],
                    start: 4,
                    end: 11,
                }),
            }),
            true_branch: Box::new(ExprNode::Var {
                text: "$z".into(),
                name: "z".into(),
                start: 14,
                end: 16,
            }),
            false_branch: Box::new(ExprNode::Literal {
                text: "0".into(),
                start: 19,
                end: 20,
            }),
        };
        let vars = node.vars();
        assert_eq!(vars.len(), 3);
        assert!(vars.contains("x"));
        assert!(vars.contains("y"));
        assert!(vars.contains("z"));
    }

    #[test]
    fn command_node_stops_var_collection() {
        let node = ExprNode::Command {
            text: "[set x 1]".into(),
            start: 0,
            end: 9,
        };
        // At the AST level, command substitution is opaque.
        assert!(node.vars().is_empty());
    }

    #[test]
    fn binop_display() {
        assert_eq!(BinOp::Add.to_string(), "+");
        assert_eq!(BinOp::StrEq.to_string(), "eq");
        assert_eq!(BinOp::Contains.to_string(), "contains");
    }

    #[test]
    fn unaryop_display() {
        assert_eq!(UnaryOp::Neg.to_string(), "-");
        assert_eq!(UnaryOp::WordNot.to_string(), "not");
    }

    #[test]
    fn expr_text_renders_structured() {
        let node = ExprNode::Literal {
            text: "42".into(),
            start: 0,
            end: 2,
        };
        assert_eq!(expr_text(&node), "42");
    }

    #[test]
    fn clone_and_eq() {
        let a = ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn string_node() {
        let node = ExprNode::String {
            text: r#""hello""#.into(),
            start: 0,
            end: 7,
        };
        assert_eq!(render_expr(&node), r#""hello""#);
    }

    #[test]
    fn irules_binary_ops() {
        let node = ExprNode::Binary {
            op: BinOp::Contains,
            left: Box::new(ExprNode::Var {
                text: "$uri".into(),
                name: "uri".into(),
                start: 0,
                end: 4,
            }),
            right: Box::new(ExprNode::String {
                text: r#""admin""#.into(),
                start: 15,
                end: 22,
            }),
        };
        assert_eq!(render_expr(&node), r#"$uri contains "admin""#);
    }
}
