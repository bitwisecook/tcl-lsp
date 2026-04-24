//! Structured AST for Tcl `[expr]` expressions.
//!
//! Replaces the opaque `String` representation used in `IRAssignExpr.expr`,
//! `IRIfClause.condition`, `IRFor.condition`, and `CFGBranch.condition`.
//! Parsed once at lowering time, then walked by downstream analyses
//! (SSA, SCCP, type inference, shimmer).
//!
//! The [`ExprRaw`] variant is a fallback for any expression the parser
//! cannot handle — every consumer must treat it as "give up, use the
//! string".

use std::collections::HashSet;
use std::fmt;

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
            Self::MatchesGlob => "matches_glob",
            Self::MatchesRegex => "matches_regex",
        }
    }

    /// Operator precedence (higher = tighter binding).
    ///
    /// Used by [`render_expr`] to determine when parentheses are needed.
    ///
    /// [`render_expr`]: super::render_expr
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
/// can produce, plus [`ExprRaw`] as the "give up" fallback. Expression
/// trees are immutable once built (no interior mutability). Recursive
/// children are `Box<ExprNode>` to keep the enum size bounded.
#[derive(Debug, Clone, PartialEq)]
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
    #[must_use]
    pub fn vars(&self) -> HashSet<String> {
        let mut result = HashSet::new();
        self.collect_vars(&mut result);
        result
    }

    /// Recursive variable collection helper.
    fn collect_vars(&self, out: &mut HashSet<String>) {
        match self {
            Self::Var { name, .. } => {
                if !name.is_empty() {
                    out.insert(name.clone());
                }
            }
            Self::Binary { left, right, .. } => {
                left.collect_vars(out);
                right.collect_vars(out);
            }
            Self::Unary { operand, .. } => {
                operand.collect_vars(out);
            }
            Self::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                condition.collect_vars(out);
                true_branch.collect_vars(out);
                false_branch.collect_vars(out);
            }
            Self::Call { args, .. } => {
                for arg in args {
                    arg.collect_vars(out);
                }
            }
            // Command substitutions may contain variable references,
            // but extracting them requires script-level analysis that
            // lives in the SSA module. At the pure-AST level we stop
            // at the command boundary.
            Self::Command { .. }
            | Self::Literal { .. }
            | Self::String { .. }
            | Self::Raw { .. } => {}
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
