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
