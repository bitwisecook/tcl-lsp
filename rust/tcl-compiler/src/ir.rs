//! Intermediate representation (IR) for Tcl analysis.
//!
//! This is a structured IR front-end that keeps source [`Span`]s on all
//! nodes. Later passes lower this to CFG + SSA.
//!
//! **Architectural note:** every IR node carries a [`Span`] (two `u32`
//! byte offsets) rather than inline `(line, character, offset)` pairs.
//! Text and positions are resolved on demand via a
//! [`SourceMap`](tcl_lexer::SourceMap). This mirrors the span-first
//! design established in the lexer crate.

use tcl_lexer::Span;

use crate::expr_ast::ExprNode;

// Command tokens

/// Original parsed tokens for a command invocation.
///
/// Carried on [`Statement::Call`] and [`Statement::Barrier`] so
/// downstream passes (optimiser, compiler checks) can inspect tokens
/// without re-lexing.
///
/// In the Rust pipeline, token references use [`Span`]s into the
/// source buffer. The `argv_texts` and `single_token_word` fields
/// preserve the Python-era per-word metadata needed by analysis passes.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandTokens {
    /// Per-word representative token spans.
    pub argv: Vec<Span>,
    /// Per-word text values.
    pub argv_texts: Vec<String>,
    /// Per-word representative token kind. Preserves the [`tcl_lexer::TokenType`]
    /// of each argv entry so analysis passes can distinguish a
    /// brace-string literal (`Str`) from a bareword (`Esc`), a
    /// variable reference (`Var`), or a command substitution
    /// (`Cmd`). Mirrors Python's `Token.type` exposed via
    /// `cmd.argv[i].type`.
    pub argv_kinds: Vec<tcl_lexer::TokenType>,
    /// Whether each word consists of a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Span>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
}

// IR node types
//
// The Python codebase uses frozen dataclasses for each IR node kind,
// with `IRStatement` as a union type alias. In Rust we model this as
// a flat enum with struct variants. Each variant carries a `Span` for
// position tracking (replacing the Python `Range`), plus variant-specific
// fields.

/// A script: a sequence of IR statements.
///
/// Corresponds to Python's `IRScript`. Using `Vec` rather than a tuple
/// because scripts are built incrementally during lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Script {
    /// Statements in execution order.
    pub statements: Vec<Statement>,
}

impl Script {
    /// Create an empty script.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Create a script from a list of statements.
    #[must_use]
    pub fn from_statements(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

/// An `if` clause: condition + body.
#[derive(Debug, Clone, PartialEq)]
pub struct IfClause {
    /// The parsed condition expression.
    pub condition: ExprNode,
    /// Source span of the condition text.
    pub condition_span: Span,
    /// Body script to execute when the condition is true.
    pub body: Script,
    /// Source span of the body text.
    pub body_span: Span,
}

/// A `try` handler clause (`on`/`trap`).
#[derive(Debug, Clone, PartialEq)]
pub struct TryHandler {
    /// Handler kind: `"on"` or `"trap"`.
    pub kind: String,
    /// Return code or error class pattern to match.
    pub match_arg: String,
    /// Variable bound to the result, if any.
    pub var_name: Option<String>,
    /// Variable bound to the options dict, if any.
    pub options_var: Option<String>,
    /// Handler body.
    pub body: Script,
    /// Source span of the handler body.
    pub body_span: Span,
}

/// A `switch` arm: pattern + body.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchArm {
    /// Pattern text.
    pub pattern: String,
    /// Source span of the pattern.
    pub pattern_span: Span,
    /// Body script (`None` for fall-through arms).
    pub body: Option<Script>,
    /// Source span of the body (`None` for fall-through arms).
    pub body_span: Option<Span>,
    /// Whether this arm falls through to the next.
    pub fallthrough: bool,
}

/// An iterator group in a `foreach`/`lmap`/`dict for` loop.
///
/// Each group pairs a list of loop variable names with the list
/// argument text they iterate over.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeachIterator {
    /// Variable names assigned each iteration.
    pub vars: Vec<String>,
    /// Source text of the list argument.
    pub list_arg: String,
}

/// An IR statement — the building block of the compiler pipeline.
///
/// Corresponds to Python's `IRStatement` union type. Each variant
/// carries a [`Span`] for position tracking (replacing the Python
/// `Range` with inline `SourcePosition` pairs). The span is resolved
/// to text or `(line, character)` via a `SourceMap` on demand.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Constant assignment: `set name value` where value is a literal.
    AssignConst {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Constant value text.
        value: String,
    },

    /// Expression assignment: `set name [expr ...]`.
    AssignExpr {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Parsed expression AST.
        expr: ExprNode,
    },

    /// General value assignment: `set name value` where value may
    /// contain substitutions.
    AssignValue {
        /// Source span of the full command.
        span: Span,
        /// Variable name.
        name: String,
        /// Value text (may contain backslash substitutions).
        value: String,
        /// Whether the value contains backslash substitutions.
        value_needs_backsubst: bool,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Increment: `incr name ?amount?`.
    Incr {
        /// Source span.
        span: Span,
        /// Variable name.
        name: String,
        /// Increment amount (None = 1).
        amount: Option<String>,
        /// Whether it is safe if the variable is uninitialised.
        safe_on_uninit: bool,
    },

    /// Standalone expression evaluation: `expr ...`.
    ExprEval {
        /// Source span.
        span: Span,
        /// Parsed expression AST.
        expr: ExprNode,
    },

    /// A generic command invocation.
    ///
    /// Optionally annotated with variables it defines/reads.
    Call {
        /// Source span.
        span: Span,
        /// Command name.
        command: String,
        /// Argument texts.
        args: Vec<String>,
        /// Variables defined by this command.
        defs: Vec<String>,
        /// Variables read by this command (beyond `$`-references in args).
        reads: Vec<String>,
        /// Whether defined variables are also read (read-before-write).
        reads_own_defs: bool,
        /// Whether it is safe if defined variables are uninitialised.
        safe_on_uninit: bool,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
        /// Per-iterator-group sizes for synthetic `foreach` /
        /// `lmap` / `dict for` / `dict map` header calls produced
        /// by the CFG builder.  ``Some([n0, n1, …])`` encodes that
        /// `defs` is a flattened concatenation of `n0` + `n1` + …
        /// vars per iterator group, so the codegen can reconstruct
        /// the original `var-list` ↔ `list-arg` pairing.  ``None``
        /// for every other call shape.  Mirrors Python's
        /// `IRCall.foreach_groups` field added by upstream commit
        /// ``342d4c7a`` (PR #331).
        foreach_groups: Option<Vec<usize>>,
    },

    /// Return statement: `return ?value?`.
    Return {
        /// Source span.
        span: Span,
        /// Return value text, if any.
        value: Option<String>,
        /// Return expression, if any (for `return [expr ...]`).
        expr: Option<ExprNode>,
        /// Whether the value was braced.
        braced: bool,
    },

    /// A command whose side effects defeat static analysis.
    ///
    /// Commands like `eval`, `uplevel`, and `upvar` can modify
    /// arbitrary variables at runtime, so no constant propagation or
    /// dead-store reasoning can cross a barrier.
    Barrier {
        /// Source span.
        span: Span,
        /// Human-readable reason for the barrier.
        reason: String,
        /// Original command name.
        command: String,
        /// Original argument texts.
        args: Vec<String>,
        /// Command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Inline group of statements to splice into the enclosing
    /// script *without* introducing a separate scope. Produced by
    /// [`crate::inline_uplevel`] when a passthrough callsite is
    /// rewritten — the callee's body runs in the caller's frame, so
    /// it can be flattened directly into the parent block stream.
    /// Also produced by C35's const-propagation when an `eval` /
    /// `uplevel` body resolves to a brace-literal.
    ///
    /// Mirrors Python's `IRBlock` (`core/compiler/ir.py`).
    Block {
        /// Source span of the original call that produced this block.
        span: Span,
        /// The pre-lowered body, evaluated in the enclosing scope.
        body: Script,
        /// Fully-qualified namespace the body was lowered in.
        namespace: String,
        /// Original command-tokens snapshot for downstream analysis
        /// (lets diagnostics still report the source surface form).
        tokens: Option<CommandTokens>,
    },

    /// Static-body uplevel — `uplevel ?level? {body}` where the body
    /// is a brace-string literal so the body's IR can be lowered
    /// inline. Models the "shift the active frame, evaluate body,
    /// restore frame" semantics that `uplevel` provides without the
    /// runtime [`Self::Barrier`] dispatch.
    ///
    /// Introduced in main commit `2992e6cc` ("introduce `IRUpFrame`
    /// for static-body uplevel"). Codegen emits `frame_depth_stash`
    /// / `frame_depth_restore` around the body (matching `698f2f79`).
    UpFrame {
        /// Source span of the original `uplevel` call.
        span: Span,
        /// Frame shift in caller-relative levels — `1` for `uplevel
        /// 1 {body}` (the canonical form), `0` for the rare `uplevel
        /// #0 {body}` global form. Sign matches the C Tcl level
        /// argument: positive = move up the stack, `0` = absolute.
        frame_shift: i32,
        /// The pre-lowered body, evaluated at the shifted frame.
        body: Script,
        /// Original command tokens for downstream analysis.
        tokens: Option<CommandTokens>,
    },

    /// Conditional: `if cond body ?elseif cond body ...? ?else body?`.
    If {
        /// Source span.
        span: Span,
        /// `if`/`elseif` clauses in order.
        clauses: Vec<IfClause>,
        /// Optional `else` body.
        else_body: Option<Script>,
        /// Source span of the `else` body.
        else_span: Option<Span>,
    },

    /// `for` loop: `for init cond next body`.
    For {
        /// Source span.
        span: Span,
        /// Initialisation script.
        init: Script,
        /// Source span of the init script.
        init_span: Span,
        /// Loop condition expression.
        condition: ExprNode,
        /// Source span of the condition.
        condition_span: Span,
        /// Next-iteration script.
        next: Script,
        /// Source span of the next script.
        next_span: Span,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
    },

    /// `while` loop: `while cond body`.
    While {
        /// Source span.
        span: Span,
        /// Loop condition expression.
        condition: ExprNode,
        /// Source span of the condition.
        condition_span: Span,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
    },

    /// `foreach`/`lmap`/`dict for`/`dict map` loop.
    Foreach {
        /// Source span.
        span: Span,
        /// Iterator groups: `(var_list, list_arg)` pairs.
        iterators: Vec<ForeachIterator>,
        /// Loop body.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Whether this is an `lmap` (returns a list).
        is_lmap: bool,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Whether this is `dict for`/`dict map`.
        is_dict_iteration: bool,
    },

    /// `catch script ?resultVar? ?optionsVar?`.
    Catch {
        /// Source span.
        span: Span,
        /// Body script to evaluate.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Variable for the result, if any.
        result_var: Option<String>,
        /// Variable for the options dict, if any.
        options_var: Option<String>,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
        /// Original parsed tokens, including braced/quoted flags.
        /// Threaded through so the CFG's `Catch → Call` lowering
        /// (`emit_opaque_catch`) can preserve the `{…}` braces
        /// around the body when the codegen falls back to
        /// `tcl_eval`.  Without this, ``catch {$undef} msg`` would
        /// be reconstructed as ``catch $undef msg`` and the
        /// unset-var read would fire before catch could intercept
        /// it.  Mirrors Python's `IRCatch.tokens` field added by
        /// upstream commit ``31f5357f`` (PR #341).
        tokens: Option<CommandTokens>,
    },

    /// `try body ?on/trap ...? ?finally body?`.
    Try {
        /// Source span.
        span: Span,
        /// Body script.
        body: Script,
        /// Source span of the body.
        body_span: Span,
        /// Handler clauses.
        handlers: Vec<TryHandler>,
        /// Optional `finally` body.
        finally_body: Option<Script>,
        /// Source span of the `finally` body.
        finally_span: Option<Span>,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
    },

    /// `switch` statement.
    Switch {
        /// Source span.
        span: Span,
        /// Subject text being matched.
        subject: String,
        /// Source span of the subject.
        subject_span: Span,
        /// Pattern/body arms.
        arms: Vec<SwitchArm>,
        /// Optional default body.
        default_body: Option<Script>,
        /// Source span of the default body.
        default_span: Option<Span>,
        /// Match mode: `"exact"`, `"glob"`, or `"regexp"`.
        mode: SwitchMode,
        /// Whether matching is case-insensitive.
        nocase: bool,
        /// Raw argument texts for generic fallback.
        raw_args: Vec<String>,
    },
}

impl Statement {
    /// Return the source span of this statement.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::AssignConst { span, .. }
            | Self::AssignExpr { span, .. }
            | Self::AssignValue { span, .. }
            | Self::Incr { span, .. }
            | Self::ExprEval { span, .. }
            | Self::Call { span, .. }
            | Self::Return { span, .. }
            | Self::Barrier { span, .. }
            | Self::Block { span, .. }
            | Self::UpFrame { span, .. }
            | Self::If { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::Foreach { span, .. }
            | Self::Catch { span, .. }
            | Self::Try { span, .. }
            | Self::Switch { span, .. } => *span,
        }
    }
}

/// Switch matching mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SwitchMode {
    /// Exact string match (default).
    #[default]
    Exact,
    /// Glob pattern match.
    Glob,
    /// Regular expression match.
    Regexp,
}

impl SwitchMode {
    /// Parse a switch mode from its Tcl string form.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "glob" => Self::Glob,
            "regexp" => Self::Regexp,
            _ => Self::Exact,
        }
    }

    /// Return the Tcl string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Glob => "glob",
            Self::Regexp => "regexp",
        }
    }
}

// Procedure and module types

/// A procedure definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Procedure {
    /// Short procedure name.
    pub name: String,
    /// Fully qualified name (e.g. `::ns::proc`).
    pub qualified_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Source span of the definition.
    pub span: Span,
    /// Procedure body.
    pub body: Script,
    /// Raw parameter list text.
    pub params_raw: String,
    /// Source text of the body (`None` for synthetic procs like `when`).
    pub body_source: Option<String>,
    /// Whether defined inside `namespace eval`.
    pub namespace_scoped: bool,
    /// BIG-IP handler priority (0..2^32-1, default 500).
    pub base_priority: u32,
}

/// A method definition within a class body.
///
/// Compiles like [`Procedure`] but carries class context for
/// interprocedural analysis and devirtualisation.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    /// Enclosing class name.
    pub class_name: String,
    /// Method name.
    pub method_name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Method body.
    pub body: Script,
    /// Method kind.
    pub kind: MethodKind,
    /// Source span (may be absent for synthetic methods).
    pub span: Option<Span>,
}

/// The kind of a class method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MethodKind {
    /// Regular instance method.
    #[default]
    Method,
    /// Class-level method.
    ClassMethod,
    /// Constructor.
    Constructor,
    /// Destructor.
    Destructor,
}

impl MethodKind {
    /// Parse from the Python string representation.
    #[must_use]
    pub fn from_str_lossy(s: &str) -> Self {
        match s {
            "classmethod" => Self::ClassMethod,
            "constructor" => Self::Constructor,
            "destructor" => Self::Destructor,
            _ => Self::Method,
        }
    }

    /// Return the string form.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Method => "method",
            Self::ClassMethod => "classmethod",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
        }
    }
}

/// A top-level module: procedures + top-level script.
///
/// This is the only mutable IR type — it accumulates procedures and
/// methods during lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// Top-level script (code outside any procedure).
    pub top_level: Script,
    /// Named procedures.
    pub procedures: std::collections::HashMap<String, Procedure>,
    /// Named methods (keyed by `class::method`).
    pub methods: std::collections::HashMap<String, MethodDef>,
    /// Procedure names that were defined more than once.
    pub redefined_procedures: std::collections::HashSet<String>,
    /// `namespace import` directives captured at lowering time —
    /// `(context_namespace, absolute_pattern)` pairs. Future codegen
    /// passes pattern-match each against the final
    /// `Self::procedures` table to resolve unqualified calls
    /// directly instead of falling back to the runtime
    /// interpreter. Only absolute patterns (`::foo::*` /
    /// `::foo::bar`) are recorded; relative patterns require
    /// runtime namespace-path walking which compile-time
    /// resolution does not model. Mirrors Python's
    /// `IRModule.namespace_imports` (main commit `ea155a5c`).
    pub namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives captured at lowering time —
    /// `(context_namespace, pattern)` pairs. Codegen consults this
    /// list to gate the import shortcut on actual exportedness so
    /// `namespace import ::foo::*` only resolves names that
    /// `::foo` actually exports. Mirrors Python's
    /// `IRModule.namespace_exports` (main commit `2f5cb008`).
    pub namespace_exports: Vec<(String, String)>,
    /// SYNC9: literal command names that have an execution trace
    /// registered (`trace add execution NAME enter|leave HANDLER`).
    /// GVN consults this set to gate purity (a traced call is
    /// never pure because the trace handler composes side effects
    /// in).  Mirrors Python's `IRModule.traced_commands` field
    /// added by `8a6f4d58` (closes `#251`).
    pub traced_commands: std::collections::BTreeSet<String>,
    /// SYNC9: `true` when a `trace add execution` was seen with a
    /// non-literal command target (`trace add execution $cmd ...`).
    /// Forces GVN / partial-redundancy / loop-invariant passes to
    /// treat *every* call as potentially traced.  Mirrors Python's
    /// `IRModule.has_dynamic_trace`.
    pub has_dynamic_trace: bool,
}

/// Extract the event name from a `::when::` qualified name.
///
/// Handles both `::when::HTTP_REQUEST` and indexed forms like
/// `::when::HTTP_REQUEST#1`.
#[must_use]
pub fn when_event_name(qualified_name: &str) -> &str {
    let bare = qualified_name
        .strip_prefix("::when::")
        .unwrap_or(qualified_name);
    match bare.find('#') {
        Some(idx) => &bare[..idx],
        None => bare,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_script() {
        let script = Script::new();
        assert!(script.statements.is_empty());
    }

    #[test]
    fn script_from_statements() {
        let stmts = vec![Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            value: "1".into(),
        }];
        let script = Script::from_statements(stmts);
        assert_eq!(script.statements.len(), 1);
    }

    #[test]
    fn statement_span_accessor() {
        let stmt = Statement::Call {
            span: Span::new(5, 20),
            command: "puts".into(),
            args: vec!["hello".into()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        assert_eq!(stmt.span(), Span::new(5, 20));
    }

    #[test]
    fn assign_const_roundtrip() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            value: "42".into(),
        };
        if let Statement::AssignConst { name, value, .. } = &stmt {
            assert_eq!(name, "x");
            assert_eq!(value, "42");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn barrier_with_tokens() {
        let tokens = CommandTokens {
            argv: vec![Span::new(0, 4), Span::new(5, 10)],
            argv_texts: vec!["eval".into(), "body".into()],
            argv_kinds: vec![tcl_lexer::TokenType::Esc, tcl_lexer::TokenType::Str],
            single_token_word: vec![true, true],
            all_tokens: vec![Span::new(0, 4), Span::new(4, 5), Span::new(5, 10)],
            expand_word: None,
        };
        let stmt = Statement::Barrier {
            span: Span::new(0, 10),
            reason: "eval".into(),
            command: "eval".into(),
            args: vec!["body".into()],
            tokens: Some(tokens.clone()),
        };
        if let Statement::Barrier {
            tokens: Some(t), ..
        } = &stmt
        {
            assert_eq!(t.argv_texts, vec!["eval", "body"]);
        } else {
            panic!("expected tokens");
        }
    }

    #[test]
    fn if_statement_structure() {
        let stmt = Statement::If {
            span: Span::new(0, 50),
            clauses: vec![IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 3,
                    end: 4,
                },
                condition_span: Span::new(3, 4),
                body: Script::from_statements(vec![Statement::Call {
                    span: Span::new(6, 15),
                    command: "puts".into(),
                    args: vec!["yes".into()],
                    defs: Vec::new(),
                    reads: Vec::new(),
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                }]),
                body_span: Span::new(5, 16),
            }],
            else_body: None,
            else_span: None,
        };
        assert_eq!(stmt.span(), Span::new(0, 50));
        if let Statement::If { clauses, .. } = &stmt {
            assert_eq!(clauses.len(), 1);
        }
    }

    #[test]
    fn for_loop_structure() {
        let stmt = Statement::For {
            span: Span::new(0, 40),
            init: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(5, 12),
                name: "i".into(),
                value: "0".into(),
            }]),
            init_span: Span::new(4, 13),
            condition: ExprNode::Binary {
                op: crate::expr_ast::BinOp::Lt,
                left: Box::new(ExprNode::Var {
                    text: "$i".into(),
                    name: "i".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "10".into(),
                    start: 5,
                    end: 7,
                }),
            },
            condition_span: Span::new(14, 22),
            next: Script::from_statements(vec![Statement::Incr {
                span: Span::new(24, 30),
                name: "i".into(),
                amount: None,
                safe_on_uninit: false,
            }]),
            next_span: Span::new(23, 31),
            body: Script::new(),
            body_span: Span::new(32, 34),
            raw_args: Vec::new(),
        };
        assert_eq!(stmt.span(), Span::new(0, 40));
    }

    #[test]
    fn switch_mode_parse() {
        assert_eq!(SwitchMode::from_str_lossy("exact"), SwitchMode::Exact);
        assert_eq!(SwitchMode::from_str_lossy("glob"), SwitchMode::Glob);
        assert_eq!(SwitchMode::from_str_lossy("regexp"), SwitchMode::Regexp);
        assert_eq!(SwitchMode::from_str_lossy("unknown"), SwitchMode::Exact);
    }

    #[test]
    fn switch_mode_str() {
        assert_eq!(SwitchMode::Exact.as_str(), "exact");
        assert_eq!(SwitchMode::Glob.as_str(), "glob");
        assert_eq!(SwitchMode::Regexp.as_str(), "regexp");
    }

    #[test]
    fn method_kind_roundtrip() {
        for kind in [
            MethodKind::Method,
            MethodKind::ClassMethod,
            MethodKind::Constructor,
            MethodKind::Destructor,
        ] {
            assert_eq!(MethodKind::from_str_lossy(kind.as_str()), kind);
        }
    }

    #[test]
    fn when_event_name_simple() {
        assert_eq!(when_event_name("::when::HTTP_REQUEST"), "HTTP_REQUEST");
    }

    #[test]
    fn when_event_name_indexed() {
        assert_eq!(when_event_name("::when::HTTP_REQUEST#1"), "HTTP_REQUEST");
    }

    #[test]
    fn when_event_name_no_prefix() {
        assert_eq!(when_event_name("HTTP_REQUEST"), "HTTP_REQUEST");
    }

    #[test]
    fn procedure_construction() {
        let proc = Procedure {
            name: "greet".into(),
            qualified_name: "::greet".into(),
            params: vec!["name".into()],
            span: Span::new(0, 50),
            body: Script::from_statements(vec![Statement::Call {
                span: Span::new(20, 40),
                command: "puts".into(),
                args: vec!["Hello $name".into()],
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }]),
            params_raw: "name".into(),
            body_source: Some("puts \"Hello $name\"".into()),
            namespace_scoped: false,
            base_priority: 500,
        };
        assert_eq!(proc.name, "greet");
        assert_eq!(proc.params, vec!["name"]);
    }

    #[test]
    fn module_default() {
        let module = Module::default();
        assert!(module.top_level.statements.is_empty());
        assert!(module.procedures.is_empty());
        assert!(module.methods.is_empty());
        assert!(module.redefined_procedures.is_empty());
    }

    #[test]
    fn foreach_with_iterators() {
        let stmt = Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["k".into(), "v".into()],
                list_arg: "$dict".into(),
            }],
            body: Script::new(),
            body_span: Span::new(25, 28),
            is_lmap: false,
            raw_args: Vec::new(),
            is_dict_iteration: true,
        };
        if let Statement::Foreach {
            iterators,
            is_dict_iteration,
            ..
        } = &stmt
        {
            assert_eq!(iterators.len(), 1);
            assert_eq!(iterators[0].vars, vec!["k", "v"]);
            assert!(is_dict_iteration);
        }
    }

    #[test]
    fn catch_statement() {
        let stmt = Statement::Catch {
            span: Span::new(0, 30),
            body: Script::new(),
            body_span: Span::new(6, 12),
            result_var: Some("result".into()),
            options_var: Some("opts".into()),
            raw_args: Vec::new(),
            tokens: None,
        };
        if let Statement::Catch {
            result_var,
            options_var,
            ..
        } = &stmt
        {
            assert_eq!(result_var.as_deref(), Some("result"));
            assert_eq!(options_var.as_deref(), Some("opts"));
        }
    }

    #[test]
    fn try_with_handlers() {
        let stmt = Statement::Try {
            span: Span::new(0, 60),
            body: Script::new(),
            body_span: Span::new(4, 10),
            handlers: vec![TryHandler {
                kind: "on".into(),
                match_arg: "error".into(),
                var_name: Some("e".into()),
                options_var: None,
                body: Script::new(),
                body_span: Span::new(30, 40),
            }],
            finally_body: Some(Script::new()),
            finally_span: Some(Span::new(50, 58)),
            raw_args: Vec::new(),
        };
        if let Statement::Try {
            handlers,
            finally_body,
            ..
        } = &stmt
        {
            assert_eq!(handlers.len(), 1);
            assert_eq!(handlers[0].kind, "on");
            assert!(finally_body.is_some());
        }
    }

    #[test]
    fn switch_statement() {
        let stmt = Statement::Switch {
            span: Span::new(0, 80),
            subject: "$cmd".into(),
            subject_span: Span::new(7, 11),
            arms: vec![
                SwitchArm {
                    pattern: "start".into(),
                    pattern_span: Span::new(13, 18),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(19, 22)),
                    fallthrough: false,
                },
                SwitchArm {
                    pattern: "stop".into(),
                    pattern_span: Span::new(23, 27),
                    body: None,
                    body_span: None,
                    fallthrough: true,
                },
            ],
            default_body: Some(Script::new()),
            default_span: Some(Span::new(50, 60)),
            mode: SwitchMode::Exact,
            nocase: false,
            raw_args: Vec::new(),
        };
        if let Statement::Switch { arms, mode, .. } = &stmt {
            assert_eq!(arms.len(), 2);
            assert!(arms[1].fallthrough);
            assert_eq!(*mode, SwitchMode::Exact);
        }
    }

    #[test]
    fn clone_preserves_equality() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            value: "1".into(),
        };
        let cloned = stmt.clone();
        assert_eq!(stmt, cloned);
    }

    #[test]
    fn return_with_expr() {
        let stmt = Statement::Return {
            span: Span::new(0, 20),
            value: None,
            expr: Some(ExprNode::Binary {
                op: crate::expr_ast::BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "1".into(),
                    start: 5,
                    end: 6,
                }),
            }),
            braced: false,
        };
        if let Statement::Return { expr: Some(e), .. } = &stmt {
            let vars = e.vars();
            assert!(vars.contains("a"));
        }
    }
}
