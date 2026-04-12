//! Pre-codegen execution-intent facts derived from IR/CFG statements.
//!
//! Lightweight intent layer shared by analysis passes as an optional
//! fast-path. The facts describe *how* a value is evaluated —
//! whether it is a command substitution, what each argument looks
//! like (literal, scalar var, array element, nested command,
//! mixed), whether the substitution may have side effects or may
//! escape the current frame, and how much type-conversion pressure
//! the arguments impose.
//!
//! Ported from `core/compiler/execution_intent.py` in two strips:
//! - **C23e** (this file) — enums and dataclasses.
//! - **C23f** — the arg-categorisation + command-substitution parser
//!   + `build_function_execution_intent` that walks the CFG.

use std::collections::HashMap;

use tcl_lexer::{Lexer, TokenType};
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::Function as CfgFunction;
use crate::ir::Statement;
use crate::side_effects::classify_side_effects;

// ---------------------------------------------------------------------------
// Enums (C23e)
// ---------------------------------------------------------------------------

/// How a value is executed/evaluated by Tcl at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvocationShape {
    /// `[cmd args…]` — a bracketed command substitution.
    CommandSubstitution,
}

/// High-level argument-substitution category for command invocations.
///
/// Each argument in a command substitution is classified into one of
/// these categories; downstream passes use the resulting vector to
/// reason about type-conversion pressure, escape behaviour, and
/// purity approximations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubstitutionCategory {
    /// Plain literal text with no substitutions.
    Literal,
    /// `$var` reference.
    ScalarVar,
    /// `$arr(key)` array-element reference.
    ArrayVar,
    /// Nested `[cmd …]` substitution.
    NestedCommand,
    /// Mixed literal + substitution content.
    Mixed,
}

/// Conservative side-effect classification for command substitutions.
///
/// This is a two-valued lattice: `Pure` means the callee is known
/// pure; `MaySideEffect` is the conservative fallback consumers
/// should treat as "could do anything".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SideEffectClass {
    /// Callee is known pure (no observable side effects).
    Pure,
    /// Callee may have side effects — assume it does.
    MaySideEffect,
}

/// Conservative escape classification for command substitutions.
///
/// An "escaping" substitution may transfer control outside the
/// current frame (via `uplevel` / `eval` / `return`). Non-escaping
/// substitutions are safe to hoist or reorder within their block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeClass {
    /// Definitely does not escape the current frame.
    NoEscape,
    /// May escape — treat as a barrier.
    MayEscape,
}

// ---------------------------------------------------------------------------
// Intent structures (C23e)
// ---------------------------------------------------------------------------

/// Intent for a bracketed value like `[llength $x]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandSubstitutionIntent {
    /// Command word (first token of the substitution).
    pub command: String,
    /// Arguments after the command word (preserved verbatim).
    pub args: Vec<String>,
    /// Per-argument category, same length as [`args`](Self::args).
    pub arg_categories: Vec<SubstitutionCategory>,
    /// Conservative side-effect classification of `command`.
    pub side_effect: SideEffectClass,
    /// Conservative escape classification of `command`.
    pub escape: EscapeClass,
    /// Integer pressure metric estimating how much type conversion
    /// (shimmering) the callee is likely to force on its arguments.
    pub shimmer_pressure: u32,
    /// What shape of invocation this value represents.
    pub invocation_shape: InvocationShape,
}

impl CommandSubstitutionIntent {
    /// Convenience constructor used by the builder and tests.
    #[must_use]
    pub fn new(
        command: impl Into<String>,
        args: Vec<String>,
        arg_categories: Vec<SubstitutionCategory>,
        side_effect: SideEffectClass,
        escape: EscapeClass,
        shimmer_pressure: u32,
    ) -> Self {
        Self {
            command: command.into(),
            args,
            arg_categories,
            side_effect,
            escape,
            shimmer_pressure,
            invocation_shape: InvocationShape::CommandSubstitution,
        }
    }
}

/// Coordinate of a statement inside a CFG function: `(block_name,
/// statement_index)`.
pub type StatementKey = (String, usize);

/// Per-function intent facts keyed by CFG statement coordinates.
///
/// The builder in C23f populates this map by walking every
/// `Statement::AssignValue` in every block and attempting to parse
/// its value as a command substitution.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionExecutionIntent {
    /// `(block_name, statement_index)` → parsed intent.
    pub command_substitutions: HashMap<StatementKey, CommandSubstitutionIntent>,
}

impl FunctionExecutionIntent {
    /// Construct an empty intent record.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the intent for a statement, or `None`.
    #[must_use]
    pub fn intent_for(&self, block: &str, stmt_idx: usize) -> Option<&CommandSubstitutionIntent> {
        self.command_substitutions
            .get(&(block.to_owned(), stmt_idx))
    }

    /// Number of classified command substitutions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.command_substitutions.len()
    }

    /// Whether no command substitutions were classified.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.command_substitutions.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Builders (C23f)
// ---------------------------------------------------------------------------

/// Classify an argument's substitution category from its textual
/// form. Ported from `execution_intent.py::_categorise_arg`.
#[must_use]
pub fn categorise_arg(text: &str) -> SubstitutionCategory {
    let stripped = text.trim();
    if stripped.contains('[') && stripped.contains(']') {
        return SubstitutionCategory::NestedCommand;
    }
    if stripped.starts_with('$') {
        if stripped.contains('(') && stripped.contains(')') {
            return SubstitutionCategory::ArrayVar;
        }
        return SubstitutionCategory::ScalarVar;
    }
    if stripped.contains('$') || stripped.contains('[') || stripped.contains('\\') {
        return SubstitutionCategory::Mixed;
    }
    SubstitutionCategory::Literal
}

/// Estimate shimmer (type-conversion) pressure from a vector of
/// argument categories. Matches the weights in
/// `execution_intent.py::_shimmer_pressure`: scalar/array refs
/// contribute 1 each, nested commands and mixed contribute 2.
#[must_use]
pub fn shimmer_pressure(categories: &[SubstitutionCategory]) -> u32 {
    let mut p = 0u32;
    for c in categories {
        match c {
            SubstitutionCategory::ScalarVar | SubstitutionCategory::ArrayVar => p += 1,
            SubstitutionCategory::NestedCommand | SubstitutionCategory::Mixed => p += 2,
            SubstitutionCategory::Literal => {}
        }
    }
    p
}

/// Conservative side-effect classification by bridging to
/// [`crate::side_effects::classify_side_effects`] (C23d). Maps the
/// two-valued intent lattice onto the richer compiler classifier's
/// `pure` flag.
#[must_use]
pub fn classify_side_effect(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&str>,
) -> SideEffectClass {
    let cse = classify_side_effects(registry, command, args, dialect, None);
    if cse.pure {
        SideEffectClass::Pure
    } else {
        SideEffectClass::MaySideEffect
    }
}

/// Conservative escape classification. A substitution may escape
/// the current frame when the callee is a dynamic barrier
/// (`eval`/`uplevel`/…) or when any argument contains a nested
/// command or mixed text that the analysis cannot reason about
/// precisely.
#[must_use]
pub fn classify_escape(
    registry: &CommandRegistry,
    command: &str,
    categories: &[SubstitutionCategory],
) -> EscapeClass {
    if let Some(spec) = registry.get(command) {
        if spec.traits.contains(Traits::CREATES_BARRIER) {
            return EscapeClass::MayEscape;
        }
    }
    if categories.iter().any(|c| {
        matches!(
            c,
            SubstitutionCategory::NestedCommand | SubstitutionCategory::Mixed
        )
    }) {
        EscapeClass::MayEscape
    } else {
        EscapeClass::NoEscape
    }
}

/// Render a single token into its intent-form text: VAR becomes
/// `$name`, CMD becomes `[text]`, other kinds keep their body
/// unchanged. Mirrors Python `_token_text_for_intent`.
fn token_intent_text<'src>(source: &'src str, tok: &tcl_lexer::Token) -> &'src str {
    let start = tok.span.start() as usize;
    let end = tok.span.end() as usize;
    // Token spans already cover the full token text (including any
    // leading `$` / `[` and trailing `]` for wrapper tokens).
    // Returning the raw slice preserves the Python-side behaviour.
    let _ = tok.kind;
    &source[start..end]
}

/// Parse a bracketed command substitution `[cmd args...]` into a
/// [`CommandSubstitutionIntent`].
///
/// Returns `None` when:
/// - `value` is not `[…]`-wrapped.
/// - The bracket body is empty.
/// - The body contains multiple top-level commands (EOL followed
///   by another word).
///
/// Uses the Rust lexer to tokenise the inner body; COMMENT and SEP
/// tokens are skipped; EOL terminates the first command and
/// rejects any further words. Adjacent tokens without a separator
/// are concatenated into one argument (matches the Python behaviour
/// for `$a$b` or `foo$x`).
#[must_use]
pub fn parse_command_substitution(
    registry: &CommandRegistry,
    value: &str,
    dialect: Option<&str>,
) -> Option<CommandSubstitutionIntent> {
    let stripped = value.trim();
    let inner = stripped.strip_prefix('[')?.strip_suffix(']')?;

    let lexer = Lexer::new(inner);
    let tokens = lexer.tokenise_all().ok()?;

    let mut argv: Vec<String> = Vec::new();
    let mut prev_type = TokenType::Eol;
    let mut saw_terminator = false;

    for tok in &tokens {
        match tok.kind {
            TokenType::Comment | TokenType::Sep => {
                prev_type = tok.kind;
                continue;
            }
            TokenType::Eol => {
                // Single-command only: remember the terminator and
                // reject further tokens.
                if !argv.is_empty() {
                    saw_terminator = true;
                }
                prev_type = tok.kind;
                continue;
            }
            TokenType::Eof => break,
            _ => {}
        }
        if saw_terminator {
            return None;
        }
        let text = token_intent_text(inner, tok).to_owned();
        if matches!(prev_type, TokenType::Sep | TokenType::Eol) || argv.is_empty() {
            argv.push(text);
        } else {
            // Concatenate onto the previous word (adjacent tokens
            // without a separator — e.g. `$a$b` or `foo$x`).
            argv.last_mut().unwrap().push_str(&text);
        }
        prev_type = tok.kind;
    }

    if argv.is_empty() {
        return None;
    }
    let command = argv.remove(0);
    let remaining_args = argv;
    let categories: Vec<SubstitutionCategory> =
        remaining_args.iter().map(|a| categorise_arg(a)).collect();
    let side_effect = classify_side_effect(registry, &command, &remaining_args, dialect);
    let escape = classify_escape(registry, &command, &categories);
    let shimmer_p = shimmer_pressure(&categories);
    Some(CommandSubstitutionIntent::new(
        command,
        remaining_args,
        categories,
        side_effect,
        escape,
        shimmer_p,
    ))
}

/// Walk a CFG function, collecting intent records for every
/// `Statement::AssignValue` whose value parses as a command
/// substitution. Ported from
/// `execution_intent.py::build_function_execution_intent`.
#[must_use]
pub fn build_function_execution_intent(
    registry: &CommandRegistry,
    cfg: &CfgFunction,
    dialect: Option<&str>,
) -> FunctionExecutionIntent {
    let mut out = FunctionExecutionIntent::new();
    for (block_name, block) in &cfg.blocks {
        for (idx, stmt) in block.statements.iter().enumerate() {
            if let Statement::AssignValue { value, .. } = stmt {
                if let Some(parsed) = parse_command_substitution(registry, value, dialect) {
                    out.command_substitutions
                        .insert((block_name.clone(), idx), parsed);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_default_shape_is_command_substitution() {
        let intent = CommandSubstitutionIntent::new(
            "llength",
            vec!["$x".into()],
            vec![SubstitutionCategory::ScalarVar],
            SideEffectClass::Pure,
            EscapeClass::NoEscape,
            1,
        );
        assert_eq!(
            intent.invocation_shape,
            InvocationShape::CommandSubstitution
        );
        assert_eq!(intent.command, "llength");
        assert_eq!(intent.shimmer_pressure, 1);
    }

    #[test]
    fn function_intent_lookup_by_statement_key() {
        let mut fi = FunctionExecutionIntent::new();
        let intent = CommandSubstitutionIntent::new(
            "llength",
            vec!["$x".into()],
            vec![SubstitutionCategory::ScalarVar],
            SideEffectClass::Pure,
            EscapeClass::NoEscape,
            1,
        );
        fi.command_substitutions
            .insert(("entry_0".into(), 0), intent);
        assert_eq!(fi.len(), 1);
        assert!(!fi.is_empty());
        assert!(fi.intent_for("entry_0", 0).is_some());
        assert!(fi.intent_for("entry_0", 1).is_none());
    }

    #[test]
    fn empty_intent_is_empty() {
        let fi = FunctionExecutionIntent::new();
        assert!(fi.is_empty());
        assert_eq!(fi.len(), 0);
    }

    // -- C23f: builders --

    #[test]
    fn categorise_literal_text() {
        assert_eq!(categorise_arg("hello"), SubstitutionCategory::Literal);
        assert_eq!(categorise_arg("42"), SubstitutionCategory::Literal);
        assert_eq!(categorise_arg("  spaced  "), SubstitutionCategory::Literal);
    }

    #[test]
    fn categorise_scalar_and_array_vars() {
        assert_eq!(categorise_arg("$x"), SubstitutionCategory::ScalarVar);
        assert_eq!(
            categorise_arg("$arr(key)"),
            SubstitutionCategory::ArrayVar
        );
    }

    #[test]
    fn categorise_nested_command_and_mixed() {
        assert_eq!(
            categorise_arg("[llength $x]"),
            SubstitutionCategory::NestedCommand
        );
        assert_eq!(categorise_arg("hello$x"), SubstitutionCategory::Mixed);
        assert_eq!(categorise_arg("hello\\n"), SubstitutionCategory::Mixed);
    }

    #[test]
    fn shimmer_pressure_weights() {
        // Literal → 0, ScalarVar → 1, ArrayVar → 1,
        // NestedCommand → 2, Mixed → 2.
        assert_eq!(shimmer_pressure(&[]), 0);
        assert_eq!(shimmer_pressure(&[SubstitutionCategory::Literal]), 0);
        assert_eq!(shimmer_pressure(&[SubstitutionCategory::ScalarVar]), 1);
        assert_eq!(
            shimmer_pressure(&[SubstitutionCategory::ArrayVar, SubstitutionCategory::ScalarVar]),
            2
        );
        assert_eq!(
            shimmer_pressure(&[
                SubstitutionCategory::NestedCommand,
                SubstitutionCategory::Mixed
            ]),
            4
        );
    }

    #[test]
    fn classify_side_effect_pure_vs_impure() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            classify_side_effect(&registry, "expr", &["{1 + 2}".into()], None),
            SideEffectClass::Pure
        );
        assert_eq!(
            classify_side_effect(&registry, "set", &["x".into(), "1".into()], None),
            SideEffectClass::MaySideEffect
        );
    }

    #[test]
    fn classify_escape_eval_is_may_escape() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            classify_escape(&registry, "eval", &[]),
            EscapeClass::MayEscape
        );
    }

    #[test]
    fn classify_escape_nested_args_force_may_escape() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            classify_escape(
                &registry,
                "puts",
                &[SubstitutionCategory::NestedCommand]
            ),
            EscapeClass::MayEscape
        );
        assert_eq!(
            classify_escape(&registry, "puts", &[SubstitutionCategory::Mixed]),
            EscapeClass::MayEscape
        );
    }

    #[test]
    fn classify_escape_pure_literals_no_escape() {
        let registry = CommandRegistry::build_default();
        assert_eq!(
            classify_escape(&registry, "llength", &[SubstitutionCategory::ScalarVar]),
            EscapeClass::NoEscape
        );
        assert_eq!(
            classify_escape(&registry, "llength", &[SubstitutionCategory::Literal]),
            EscapeClass::NoEscape
        );
    }

    #[test]
    fn parse_rejects_non_bracketed() {
        let registry = CommandRegistry::build_default();
        assert!(parse_command_substitution(&registry, "hello", None).is_none());
        assert!(parse_command_substitution(&registry, "$x", None).is_none());
    }

    #[test]
    fn parse_empty_brackets_returns_none() {
        let registry = CommandRegistry::build_default();
        assert!(parse_command_substitution(&registry, "[]", None).is_none());
    }

    #[test]
    fn parse_multi_command_body_returns_none() {
        let registry = CommandRegistry::build_default();
        // Two commands separated by `;` — Python rejects.
        let out = parse_command_substitution(&registry, "[set x 1; set y 2]", None);
        assert!(out.is_none());
    }

    #[test]
    fn parse_llength_scalar_var() {
        let registry = CommandRegistry::build_default();
        let intent = parse_command_substitution(&registry, "[llength $x]", None).unwrap();
        assert_eq!(intent.command, "llength");
        assert_eq!(intent.args, vec!["$x".to_string()]);
        assert_eq!(intent.arg_categories, vec![SubstitutionCategory::ScalarVar]);
        assert_eq!(intent.shimmer_pressure, 1);
        assert_eq!(intent.escape, EscapeClass::NoEscape);
    }

    #[test]
    fn parse_set_classifies_as_may_side_effect() {
        let registry = CommandRegistry::build_default();
        let intent = parse_command_substitution(&registry, "[set y 42]", None).unwrap();
        assert_eq!(intent.command, "set");
        assert_eq!(intent.side_effect, SideEffectClass::MaySideEffect);
    }

    #[test]
    fn parse_eval_classifies_as_may_escape() {
        let registry = CommandRegistry::build_default();
        let intent = parse_command_substitution(&registry, "[eval {expr 1}]", None).unwrap();
        assert_eq!(intent.escape, EscapeClass::MayEscape);
    }

    #[test]
    fn build_intent_visits_assign_value_statements() {
        use crate::cfg::{Function, Terminator};
        use crate::ir::Statement;
        use tcl_lexer::Span;

        let registry = CommandRegistry::build_default();
        let mut f = Function::new("::top", "entry_0");
        {
            let entry = f.blocks.get_mut("entry_0").unwrap();
            entry.statements.push(Statement::AssignValue {
                span: Span::new(0, 0),
                name: "len".into(),
                value: "[llength $x]".into(),
                value_needs_backsubst: false,
                tokens: None,
            });
            entry.statements.push(Statement::AssignValue {
                span: Span::new(0, 0),
                name: "plain".into(),
                value: "hello".into(),
                value_needs_backsubst: false,
                tokens: None,
            });
            entry.terminator = Some(Terminator::Return {
                value: None,
                span: None,
                expr: None,
                braced: false,
            });
        }
        let intent = build_function_execution_intent(&registry, &f, None);
        // Only the [llength $x] assign is classified — the plain
        // literal does not look like a command substitution.
        assert_eq!(intent.len(), 1);
        let record = intent.intent_for("entry_0", 0).expect("first stmt");
        assert_eq!(record.command, "llength");
    }

    #[test]
    fn substitution_categories_are_distinct() {
        // Exhaustive distinctness check — guards the enum's Hash
        // derive against accidental duplicates after refactors.
        use std::collections::HashSet;
        let set: HashSet<SubstitutionCategory> = [
            SubstitutionCategory::Literal,
            SubstitutionCategory::ScalarVar,
            SubstitutionCategory::ArrayVar,
            SubstitutionCategory::NestedCommand,
            SubstitutionCategory::Mixed,
        ]
        .into_iter()
        .collect();
        assert_eq!(set.len(), 5);
    }
}
