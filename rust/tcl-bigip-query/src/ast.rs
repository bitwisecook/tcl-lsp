//! AST node types for the query DSL.
//!
//! The tree is deliberately simple: one variant per syntactic form, and every
//! node carries the byte offset where it began so error messages can underline
//! the right span.
//!
//! The evaluator walks this tree top-down, producing either a plain value or
//! a stream. Path nodes also produce a *location trail* used by the edit
//! planner when the user assigns to them.

/// A literal / token value: the parsed payload of a `STRING` / `NUMBER` /
/// `true` / `false` / `null` token, or the textual name carried by `IDENT`
/// and `$IDENT`. This is a heterogeneous `token.value` field, where the
/// absence of a value is spelled `None`.
#[derive(Debug, Clone, PartialEq)]
pub enum LitValue {
    /// `None` / `null`.
    Null,
    /// A boolean literal.
    Bool(bool),
    /// An integer literal.
    Int(i64),
    /// A floating-point literal.
    Float(f64),
    /// A string literal, identifier name, or `$name` payload.
    Str(String),
}

/// A single path step: `.field` (`Field`) or `[...]` (`Subscript`).
#[derive(Debug, Clone, PartialEq)]
pub enum PathStep {
    /// `.foo` or `."foo-bar"` — a single named step.
    Field {
        name: String,
        /// The `?` suffix — reserved, not used by the evaluator yet.
        optional: bool,
        offset: usize,
    },
    /// `[expr]` / `["~regex"]` / `[number]` / `[]`.
    Subscript {
        /// `true` for the bare `[]` (iterate all).
        stream: bool,
        index: Option<Box<Expr>>,
        /// Set when the subscript was `["~..."]`.
        regex: Option<String>,
        offset: usize,
        /// The `?` suffix makes the step error-suppressing.
        optional: bool,
    },
}

impl PathStep {
    /// The byte offset where this step began.
    #[must_use]
    pub fn offset(&self) -> usize {
        match self {
            PathStep::Field { offset, .. } | PathStep::Subscript { offset, .. } => *offset,
        }
    }
}

/// An expression node — one variant per syntactic form.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal scalar value.
    Literal { value: LitValue, offset: usize },
    /// The bare `.` — yields the current input.
    Identity { offset: usize },
    /// `$name` — yields the root container of the named source.
    Variable { name: String, offset: usize },
    /// `{ key: expr, ... }` — jq's object constructor.
    ObjectLiteral {
        entries: Vec<(String, Expr)>,
        offset: usize,
    },
    /// `[ inner ]` — collect *inner*'s output into a list. `None` is `[]`.
    ListLiteral {
        inner: Option<Box<Expr>>,
        offset: usize,
    },
    /// A chain of `Field` / `Subscript` steps starting from input.
    ///
    /// `head` is the optional rooting expression — when set (`$x.foo`,
    /// `f(.).y[0]`), the steps walk from `head`'s value rather than from the
    /// current input.
    PathExpr {
        steps: Vec<PathStep>,
        offset: usize,
        head: Option<Box<Expr>>,
    },
    /// `name(args...)` — a builtin / function call.
    Call {
        name: String,
        args: Vec<Expr>,
        offset: usize,
    },
    /// A binary operator: `== != < <= > >= + - * / and or`.
    BinOp {
        op: String,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        offset: usize,
    },
    /// A unary operator: `not` or `-`.
    UnaryOp {
        op: String,
        operand: Box<Expr>,
        offset: usize,
    },
    /// `lhs | rhs` — evaluate lhs, feed each value through rhs.
    Pipe {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        offset: usize,
    },
    /// `source as $name | body` — jq's let-binding form.
    LetBinding {
        source: Box<Expr>,
        name: String,
        body: Box<Expr>,
        offset: usize,
    },
    /// `path = expr` / `path |= expr` / `path += expr` / `path -= expr`.
    ///
    /// `target` is always a `PathExpr`. `source` is set (a `Variable`) when
    /// the LHS was `$name.path`.
    Assignment {
        target: Box<Expr>,
        op: String,
        rhs: Box<Expr>,
        offset: usize,
        source: Option<Box<Expr>>,
    },
    /// `a, b, c` — jq's stream-concatenation operator.
    CommaStream { parts: Vec<Expr>, offset: usize },
    /// `if COND then BODY [elif COND then BODY]* [else BODY] end`.
    IfThenElse {
        cond: Box<Expr>,
        then_body: Box<Expr>,
        elifs: Vec<(Expr, Expr)>,
        else_body: Option<Box<Expr>>,
        offset: usize,
    },
}

impl Expr {
    /// The byte offset where this expression began. Every node carries one.
    #[must_use]
    pub fn offset(&self) -> usize {
        match self {
            Expr::Literal { offset, .. }
            | Expr::Identity { offset }
            | Expr::Variable { offset, .. }
            | Expr::ObjectLiteral { offset, .. }
            | Expr::ListLiteral { offset, .. }
            | Expr::PathExpr { offset, .. }
            | Expr::Call { offset, .. }
            | Expr::BinOp { offset, .. }
            | Expr::UnaryOp { offset, .. }
            | Expr::Pipe { offset, .. }
            | Expr::LetBinding { offset, .. }
            | Expr::Assignment { offset, .. }
            | Expr::CommaStream { offset, .. }
            | Expr::IfThenElse { offset, .. } => *offset,
        }
    }
}

/// One or more semicolon-separated statements — the top-level program.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Expr>,
}
