//! Global Value Numbering (GVN).
//!
//! Detects redundant computations by canonicalising pure expression
//! invocations to SSA-qualified [`ExprKey`] tuples and looking them
//! up in a dominator-tree-scoped hash table. A match means the same
//! expression was computed at a dominating definition; the new
//! occurrence can be replaced with the earlier result.
//!
//! Ported from `core/compiler/gvn.py` in four strips:
//! - **C26a** (this file) — value-table types and the scoped
//!   lookup table.
//! - **C26b** — canonicalisation helpers and diagnostic message
//!   builders.
//! - **C26c** — statement-level helpers: purity classifier, cmd-
//!   tokens extractor, per-statement pure-expression occurrence
//!   collector.
//! - **C26d** — `find_redundancies` driver that walks the
//!   dominator tree and reports full/partial redundancies.

use std::collections::HashMap;

use tcl_lexer::Span;

// ---------------------------------------------------------------------------
// Expression-key alias (C26a)
// ---------------------------------------------------------------------------

/// Canonical identity for a computed expression.
///
/// A call to `cmd arg1 arg2 …` becomes `["call", "cmd", arg1, arg2, …]`
/// after variable references have been rewritten to their SSA-
/// versioned form (see `canonicalise_word` in C26b). Two occurrences
/// that produce the same `ExprKey` are known to compute the same
/// value under the current SSA.
pub type ExprKey = Vec<String>;

// ---------------------------------------------------------------------------
// Redundant-computation diagnostic (C26a)
// ---------------------------------------------------------------------------

/// A computation that re-evaluates an already-available expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RedundantComputation {
    /// Span of the duplicate computation.
    pub span: Span,
    /// Span of the first computation that produced the same value.
    pub first_span: Span,
    /// Human-readable expression text for diagnostic messages.
    pub expression_text: String,
    /// Diagnostic code (e.g. `"O105"` for full redundancy,
    /// `"O106"` for partial, `"O107"` for loop-invariant).
    pub code: String,
    /// Formatted diagnostic message.
    pub message: String,
}

impl RedundantComputation {
    /// Minimal constructor used by the driver and tests.
    #[must_use]
    pub fn new(
        span: Span,
        first_span: Span,
        expression_text: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            span,
            first_span,
            expression_text: expression_text.into(),
            code: code.into(),
            message: message.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Value-table entries (C26a)
// ---------------------------------------------------------------------------

/// A single entry in a [`ScopedValueTable`] scope. Carries the
/// block / statement coordinates and the rendered expression text
/// so later occurrences can point back at where the value was
/// first computed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueEntry {
    /// Canonical key (same as the map key; stored here for ease of
    /// programmatic inspection).
    pub key: ExprKey,
    /// CFG block containing the first computation.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Source span of the first computation.
    pub span: Span,
    /// Rendered expression text (`cmd arg1 arg2 …`).
    pub expression_text: String,
}

/// One pure-expression occurrence observed in a statement stream.
///
/// Produced by the per-statement collector in C26c and consumed by
/// the fixed-point walk in C26d.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprOccurrence {
    /// Canonical key.
    pub key: ExprKey,
    /// Source span.
    pub span: Span,
    /// Rendered expression text.
    pub expression_text: String,
    /// CFG block containing the occurrence.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Variable names referenced by the expression (for loop-
    /// invariance detection).
    pub variable_uses: Vec<String>,
}

// ---------------------------------------------------------------------------
// Dominator-tree-scoped value table (C26a)
// ---------------------------------------------------------------------------

/// Stack of `ExprKey → ValueEntry` maps, one per scope.
///
/// The outermost scope always exists (index 0). Each
/// `push_scope` / `pop_scope` pair brackets the processing of a
/// dominator-tree subtree so that entries introduced on one path
/// don't leak into its siblings. Lookups search from the innermost
/// scope outward; `kill_all` discards everything (used on barrier
/// or impure-call statements).
#[derive(Debug, Default)]
pub struct ScopedValueTable {
    scopes: Vec<HashMap<ExprKey, ValueEntry>>,
}

impl ScopedValueTable {
    /// Build a table with a single empty root scope.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new empty scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the innermost scope, keeping the root scope in place.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Look `key` up from the innermost scope outward.
    #[must_use]
    pub fn lookup(&self, key: &ExprKey) -> Option<&ValueEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.get(key) {
                return Some(entry);
            }
        }
        None
    }

    /// Insert an entry in the innermost scope. An existing entry
    /// at the same key is replaced (matching Python behaviour).
    pub fn insert(&mut self, entry: ValueEntry) {
        let key = entry.key.clone();
        self.scopes
            .last_mut()
            .expect("root scope present")
            .insert(key, entry);
    }

    /// Drop every tracked entry. Used on barrier / impure-call
    /// statements where no previously-tracked value can be trusted.
    pub fn kill_all(&mut self) {
        self.scopes = vec![HashMap::new()];
    }

    /// Number of scopes currently on the stack. Primarily exposed
    /// for tests.
    #[must_use]
    pub fn scope_depth(&self) -> usize {
        self.scopes.len()
    }

    /// Total entries across all scopes. Primarily exposed for
    /// tests — the driver does not use this.
    #[must_use]
    pub fn total_entries(&self) -> usize {
        self.scopes.iter().map(HashMap::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &[&str], block: &str, idx: usize) -> ValueEntry {
        let key_owned: ExprKey = key.iter().map(|s| (*s).into()).collect();
        ValueEntry {
            key: key_owned.clone(),
            block: block.into(),
            statement_index: idx,
            span: Span::new(0, 0),
            expression_text: key_owned.join(" "),
        }
    }

    #[test]
    fn scoped_table_root_scope_always_present() {
        let t = ScopedValueTable::new();
        assert_eq!(t.scope_depth(), 1);
        assert!(t.lookup(&vec!["call".into(), "foo".into()]).is_none());
    }

    #[test]
    fn insert_and_lookup_in_root() {
        let mut t = ScopedValueTable::new();
        let e = entry(&["call", "llength", "$x@1"], "entry", 2);
        t.insert(e.clone());
        let key: ExprKey = vec!["call".into(), "llength".into(), "$x@1".into()];
        assert_eq!(t.lookup(&key), Some(&e));
    }

    #[test]
    fn pop_scope_discards_inner_entries_only() {
        let mut t = ScopedValueTable::new();
        t.insert(entry(&["call", "root"], "b", 0));
        t.push_scope();
        t.insert(entry(&["call", "inner"], "b", 1));
        assert_eq!(t.total_entries(), 2);
        t.pop_scope();
        let root_key: ExprKey = vec!["call".into(), "root".into()];
        let inner_key: ExprKey = vec!["call".into(), "inner".into()];
        assert!(t.lookup(&root_key).is_some());
        assert!(t.lookup(&inner_key).is_none());
    }

    #[test]
    fn pop_scope_preserves_root_scope() {
        let mut t = ScopedValueTable::new();
        t.pop_scope(); // Should be a no-op; root always survives.
        t.pop_scope();
        assert_eq!(t.scope_depth(), 1);
    }

    #[test]
    fn lookup_shadows_outer_scope_first() {
        let mut t = ScopedValueTable::new();
        let key: ExprKey = vec!["call".into(), "f".into()];
        t.insert(entry(&["call", "f"], "outer", 0));
        t.push_scope();
        t.insert(entry(&["call", "f"], "inner", 5));
        assert_eq!(t.lookup(&key).unwrap().block, "inner");
        t.pop_scope();
        assert_eq!(t.lookup(&key).unwrap().block, "outer");
    }

    #[test]
    fn kill_all_drops_every_scope() {
        let mut t = ScopedValueTable::new();
        t.insert(entry(&["call", "a"], "b", 0));
        t.push_scope();
        t.insert(entry(&["call", "b"], "b", 1));
        t.kill_all();
        assert_eq!(t.scope_depth(), 1);
        assert_eq!(t.total_entries(), 0);
    }

    #[test]
    fn redundant_computation_constructor() {
        let r = RedundantComputation::new(
            Span::new(10, 20),
            Span::new(0, 5),
            "llength $x",
            "O105",
            "message",
        );
        assert_eq!(r.expression_text, "llength $x");
        assert_eq!(r.code, "O105");
        assert_eq!(r.span.start(), 10);
        assert_eq!(r.first_span.end(), 5);
    }

    #[test]
    fn expr_occurrence_carries_variable_uses() {
        let occ = ExprOccurrence {
            key: vec!["call".into(), "llength".into(), "$x@1".into()],
            span: Span::new(0, 5),
            expression_text: "llength $x".into(),
            block: "entry".into(),
            statement_index: 0,
            variable_uses: vec!["x".into()],
        };
        assert_eq!(occ.variable_uses, vec!["x".to_string()]);
    }
}
