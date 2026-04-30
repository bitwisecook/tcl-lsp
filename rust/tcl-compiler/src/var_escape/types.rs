//! Types for the var-escape analysis (C33a).
//!
//! Mirrors `core/compiler/var_escape/_types.py`.

use std::collections::{BTreeSet, HashMap};

/// Where a Tcl variable must live at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeTag {
    /// Only accessed through statically resolved positions; the
    /// WASM local slot is the single source of truth.
    Local,
    /// Must live in the runtime frame so the interpreter (or an
    /// `upvar` alias) can read and write it by name.
    Frame,
}

/// Join operator on the lattice: `Frame` dominates `Local`.
#[must_use]
pub fn join(a: EscapeTag, b: EscapeTag) -> EscapeTag {
    if a == EscapeTag::Frame || b == EscapeTag::Frame {
        EscapeTag::Frame
    } else {
        EscapeTag::Local
    }
}

/// Per-procedure escape classification.
///
/// `tags` maps variable name to its escape tag. Names not present
/// default to [`EscapeTag::Local`].
///
/// `dynamic_barrier` is set when the analysis encountered a
/// construct whose name-reference set cannot be bounded
/// (`eval $body`, `uplevel 1`, `info level`, etc.). In that case
/// every variable in the proc is effectively `Frame` regardless
/// of what `tags` contains.
///
/// `frame_needed` is a convenience flag for codegen: true if the
/// proc needs a runtime frame at all.
///
/// `upvar_source_names` is the set of literal variable names
/// this proc (or any of its transitive callees once the
/// interprocedural pass has run) names as the *source* of a
/// caller-frame `upvar`. A caller must treat any of its local
/// vars whose names appear here as `Frame`.
///
/// `unbounded_upvar_source` is true when the source set can't be
/// enumerated (dynamic source name, pessimistic callee). Callers
/// must spill every local in that case.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcEscapeSummary {
    /// Per-name escape tag.
    pub tags: HashMap<String, EscapeTag>,
    /// Whole-proc pessimism marker.
    pub dynamic_barrier: bool,
    /// Convenience flag for codegen.
    pub frame_needed: bool,
    /// Names this proc treats as upvar sources.
    pub upvar_source_names: BTreeSet<String>,
    /// True if the upvar-source set can't be enumerated.
    pub unbounded_upvar_source: bool,
    /// Statically resolvable callees.
    pub direct_callees: BTreeSet<String>,
    /// True if codegen needs the eval fallback for this proc.
    pub has_fallback: bool,
    /// True if the intraprocedural pass saw a non-frameless `IRCall`
    /// with a statically resolvable command word. Whether that
    /// reaches the eval fallback depends on whether the callee is
    /// a compiled proc — only the interprocedural pass can tell.
    pub has_call_fallback: bool,
    /// Per-SSA-version escape tags, populated by the flow-sensitive
    /// CFG+SSA propagation. Empty when the analysis was driven
    /// from an IR-only source. Keyed by `(name, ssa_version)`.
    pub ssa_tags: HashMap<(String, u32), EscapeTag>,
}

impl ProcEscapeSummary {
    /// Return the tag for *name* (defaults to `Local`).
    #[must_use]
    pub fn tag(&self, name: &str) -> EscapeTag {
        if self.dynamic_barrier {
            return EscapeTag::Frame;
        }
        self.tags.get(name).copied().unwrap_or(EscapeTag::Local)
    }

    /// Shorthand: does *name* need to live in the runtime frame?
    #[must_use]
    pub fn is_frame(&self, name: &str) -> bool {
        self.tag(name) == EscapeTag::Frame
    }

    /// Return a new summary with *`extra_escaped`* spilled to
    /// `Frame`. Used by the interprocedural pass to fold
    /// callee-induced escapes into a caller's summary without
    /// mutating the originally computed structure.
    #[must_use]
    pub fn with_escapes<I: IntoIterator<Item = String>>(
        &self,
        extra_escaped: I,
        pessimistic: bool,
    ) -> Self {
        let mut new_tags = self.tags.clone();
        for name in extra_escaped {
            new_tags.insert(name, EscapeTag::Frame);
        }
        let new_pessimistic = self.dynamic_barrier || pessimistic;
        let new_frame_needed = new_pessimistic || new_tags.values().any(|t| *t == EscapeTag::Frame);
        Self {
            tags: new_tags,
            dynamic_barrier: new_pessimistic,
            frame_needed: new_frame_needed,
            upvar_source_names: self.upvar_source_names.clone(),
            unbounded_upvar_source: self.unbounded_upvar_source,
            direct_callees: self.direct_callees.clone(),
            has_fallback: self.has_fallback,
            has_call_fallback: self.has_call_fallback,
            ssa_tags: self.ssa_tags.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_frame_dominates() {
        assert_eq!(join(EscapeTag::Local, EscapeTag::Local), EscapeTag::Local);
        assert_eq!(join(EscapeTag::Local, EscapeTag::Frame), EscapeTag::Frame);
        assert_eq!(join(EscapeTag::Frame, EscapeTag::Local), EscapeTag::Frame);
        assert_eq!(join(EscapeTag::Frame, EscapeTag::Frame), EscapeTag::Frame);
    }

    #[test]
    fn default_tag_is_local() {
        let s = ProcEscapeSummary::default();
        assert_eq!(s.tag("x"), EscapeTag::Local);
        assert!(!s.is_frame("anything"));
    }

    #[test]
    fn explicit_frame_tag_returns_frame() {
        let s = ProcEscapeSummary {
            tags: HashMap::from([("x".into(), EscapeTag::Frame)]),
            ..Default::default()
        };
        assert!(s.is_frame("x"));
        assert!(!s.is_frame("y"));
    }

    #[test]
    fn dynamic_barrier_forces_frame_for_all_names() {
        let s = ProcEscapeSummary {
            dynamic_barrier: true,
            tags: HashMap::from([("x".into(), EscapeTag::Local)]),
            ..Default::default()
        };
        // Even though ``x`` is recorded as Local, the barrier forces
        // every name to Frame.
        assert!(s.is_frame("x"));
        assert!(s.is_frame("not_in_tags"));
    }

    #[test]
    fn with_escapes_spills_named_vars() {
        let s = ProcEscapeSummary::default();
        let new = s.with_escapes(["a".into(), "b".into()], false);
        assert_eq!(new.tag("a"), EscapeTag::Frame);
        assert_eq!(new.tag("b"), EscapeTag::Frame);
        assert!(new.frame_needed);
    }

    #[test]
    fn with_escapes_pessimistic_sets_dynamic_barrier() {
        let s = ProcEscapeSummary::default();
        let new = s.with_escapes(std::iter::empty(), true);
        assert!(new.dynamic_barrier);
        assert!(new.frame_needed);
    }
}
