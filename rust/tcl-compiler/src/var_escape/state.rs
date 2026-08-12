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

//! Mutable per-proc escape accumulator.
//!
//! The walker holds a `&mut EscapeState` while visiting an IR
//! script body, calling the methods below as it sees relevant
//! shapes.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::var_escape::helpers::is_dynamic_name;
use crate::var_escape::types::{Barrier, EscapeFlags, EscapeReason, EscapeTag};

/// Tracked literal binding for the alias-inference path.
///
/// `Set(name, value)` records the most recent literal a variable
/// was assigned. `Invalidated` marks the binding as untrustworthy
/// once a second writer or a non-literal value appears — only the
/// single-writer case is trusted.
#[derive(Debug, Clone)]
enum LiteralBinding {
    Set(String),
    Invalidated,
}

/// Mutable per-proc escape accumulator used during the IR walk.
#[derive(Debug, Default)]
pub struct EscapeState {
    /// Per-name escape tags collected so far.
    pub tags: HashMap<String, EscapeTag>,
    /// Pessimism / fallback flag set.
    pub flags: EscapeFlags,
    /// Names the compiler already knows about (params, assigns,
    /// upvars). Used by the "spill all" branch of alias inference.
    pub known_names: HashSet<String>,
    /// Names in the caller's frame that this proc reaches by name
    /// via `upvar <positive-level>`. Consumed by the
    /// interprocedural pass to escape matching locals in callers.
    pub upvar_source_names: BTreeSet<String>,
    /// Statically-resolvable callees — qualified proc names this
    /// proc invokes with a known command word.
    pub direct_callees: BTreeSet<String>,
    /// Sequential single-literal alias tracking. Used by the
    /// `set $n` alias inference to resolve `$n` back to the
    /// literal when one writer was observed.
    literal_assigns: HashMap<String, LiteralBinding>,
    /// Recorded barrier triggers.  Each
    /// [`record_barrier`](Self::record_barrier) call appends here;
    /// the summary builder copies this into
    /// [`super::types::ProcEscapeSummary::barriers`] verbatim.
    pub barriers: Vec<Barrier>,
    /// Recorded per-name escape reasons; the summary builder copies
    /// this into [`super::types::ProcEscapeSummary::tag_reasons`].
    pub tag_reasons: HashMap<String, Vec<EscapeReason>>,
}

impl EscapeState {
    /// Create a state seeded with the given known-name set.
    #[must_use]
    pub fn new<I: IntoIterator<Item = String>>(known_names: I) -> Self {
        Self {
            known_names: known_names.into_iter().collect(),
            ..Default::default()
        }
    }

    /// True if the whole-proc dynamic-barrier marker is set.
    #[must_use]
    pub fn dynamic_barrier(&self) -> bool {
        self.flags.dynamic_barrier()
    }

    /// True if the upvar-source set can't be statically enumerated.
    #[must_use]
    pub fn unbounded_upvar_source(&self) -> bool {
        self.flags.unbounded_upvar_source()
    }

    /// True if codegen needs the eval fallback for this proc.
    #[must_use]
    pub fn has_fallback(&self) -> bool {
        self.flags.has_fallback()
    }

    /// True if a non-frameless `Statement::Call` with a statically
    /// resolvable command word was seen.
    #[must_use]
    pub fn has_call_fallback(&self) -> bool {
        self.flags.has_call_fallback()
    }

    /// Mark *name* as needing frame storage.
    pub fn escape(&mut self, name: &str) {
        if name.is_empty() {
            return;
        }
        self.tags.insert(name.to_string(), EscapeTag::Frame);
        self.known_names.insert(name.to_string());
    }

    /// Spill every known proc-local name (not proc-pessimistic).
    pub fn escape_all_known(&mut self) {
        let names: Vec<String> = self.known_names.iter().cloned().collect();
        for n in names {
            self.tags.insert(n, EscapeTag::Frame);
        }
    }

    /// Mark the whole proc as needing a dynamic-barrier fallback.
    pub fn mark_pessimistic(&mut self) {
        self.flags.insert(EscapeFlags::DYNAMIC_BARRIER);
    }

    /// Record a structured [`Barrier`]
    /// describing why the proc went pessimistic.
    ///
    /// Sets the [`EscapeFlags::DYNAMIC_BARRIER`] flag (so callers that
    /// only check the flag still see the pessimistic state) AND
    /// appends the barrier to [`Self::barriers`] so consumers (LSP
    /// hover, compiler-explorer surface) can render the specific
    /// kind / detail / range instead of opaque "frame needed".
    pub fn record_barrier(&mut self, barrier: Barrier) {
        self.flags.insert(EscapeFlags::DYNAMIC_BARRIER);
        self.barriers.push(barrier);
    }

    /// Escape *name* and record the
    /// triggering [`EscapeReason`].
    ///
    /// Sister of [`Self::escape`]; same `Frame` tag plus a structured
    /// per-name reason in [`Self::tag_reasons`].  Callers that don't
    /// have a specific reason handy keep using the bare
    /// [`Self::escape`] — those paths fall through to the
    /// barrier-synthesised reason in
    /// [`crate::var_escape::types::ProcEscapeSummary::reasons_for`].
    pub fn escape_with_reason(&mut self, name: &str, reason: EscapeReason) {
        if name.is_empty() {
            return;
        }
        self.tags.insert(name.to_string(), EscapeTag::Frame);
        self.known_names.insert(name.to_string());
        self.tag_reasons
            .entry(name.to_string())
            .or_default()
            .push(reason);
    }

    /// Record a per-name [`EscapeReason`] without changing the tag.
    /// Useful when the caller already escaped the var via a separate
    /// path and now wants to attach a reason.
    pub fn record_reason(&mut self, name: &str, reason: EscapeReason) {
        if name.is_empty() {
            return;
        }
        self.tag_reasons
            .entry(name.to_string())
            .or_default()
            .push(reason);
    }

    /// Record a literal caller-frame name this proc accesses via
    /// `upvar`.
    pub fn record_upvar_source(&mut self, src: &str) {
        if !src.is_empty() {
            self.upvar_source_names.insert(src.to_string());
        }
    }

    /// Record that the upvar source set cannot be statically
    /// enumerated.
    pub fn record_unbounded_upvar(&mut self) {
        self.flags.insert(EscapeFlags::UNBOUNDED_UPVAR_SOURCE);
    }

    /// Record a statically-resolvable callee.
    pub fn record_callee(&mut self, qname: &str) {
        if !qname.is_empty() {
            self.direct_callees.insert(qname.to_string());
        }
    }

    /// Record a definite interpreter dispatch (barrier-shaped).
    pub fn record_fallback(&mut self) {
        self.flags.insert(EscapeFlags::HAS_FALLBACK);
    }

    /// Record a non-frameless `Statement::Call` with a static command word.
    /// Whether this really reaches the eval fallback is decided by
    /// the interprocedural pass; if the callee resolves to a
    /// compiled proc the downgrade there drops the flag.
    pub fn record_call_fallback(&mut self) {
        self.flags.insert(EscapeFlags::HAS_CALL_FALLBACK);
    }

    /// Record that *name* was assigned *value* via
    /// `Statement::AssignConst`. Invalidates the tracked literal
    /// once a second writer appears: only the single-writer case
    /// is trusted for alias inference.
    pub fn note_literal_assign(&mut self, name: &str, value: &str) {
        if name.is_empty() || is_dynamic_name(name) {
            return;
        }
        match self.literal_assigns.get(name) {
            Some(_) => {
                // Second write — invalidate.
                self.literal_assigns
                    .insert(name.to_string(), LiteralBinding::Invalidated);
            }
            None => {
                self.literal_assigns
                    .insert(name.to_string(), LiteralBinding::Set(value.to_string()));
            }
        }
    }

    /// Mark *name*'s tracked literal invalid (subsequent writer).
    pub fn invalidate_literal(&mut self, name: &str) {
        if !name.is_empty() {
            self.literal_assigns
                .insert(name.to_string(), LiteralBinding::Invalidated);
        }
    }

    /// Resolve a dynamic var-name token (`$n` / `${n}`) to the
    /// literal it was assigned, if the single-writer rule holds.
    ///
    /// Returns `None` when *token* isn't a simple `$name`
    /// reference, when the tracked literal has been invalidated,
    /// or when the tracked value isn't a valid Tcl identifier.
    #[must_use]
    pub fn resolve_literal(&self, token: &str) -> Option<String> {
        if token.is_empty() || !token.starts_with('$') {
            return None;
        }
        let inner = if let Some(rest) = token.strip_prefix("${") {
            rest.strip_suffix('}')?
        } else {
            &token[1..]
        };
        if inner.is_empty() || inner.contains('[') || inner.contains(']') || inner.contains('$') {
            return None;
        }
        let binding = self.literal_assigns.get(inner)?;
        let LiteralBinding::Set(literal) = binding else {
            return None;
        };
        if literal.is_empty() {
            return None;
        }
        // Guard: only plain identifiers (alphanumeric, underscore,
        // colons).
        if !literal
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
        {
            return None;
        }
        Some(literal.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_marks_name_as_frame() {
        let mut s = EscapeState::default();
        s.escape("x");
        assert_eq!(s.tags.get("x"), Some(&EscapeTag::Frame));
        assert!(s.known_names.contains("x"));
    }

    #[test]
    fn escape_ignores_empty_name() {
        let mut s = EscapeState::default();
        s.escape("");
        assert!(s.tags.is_empty());
    }

    #[test]
    fn escape_all_known_spills_seeded_names() {
        let mut s = EscapeState::new(["a".to_string(), "b".to_string()]);
        s.escape_all_known();
        assert_eq!(s.tags.get("a"), Some(&EscapeTag::Frame));
        assert_eq!(s.tags.get("b"), Some(&EscapeTag::Frame));
    }

    #[test]
    fn mark_pessimistic_sets_dynamic_barrier() {
        let mut s = EscapeState::default();
        s.mark_pessimistic();
        assert!(s.dynamic_barrier());
    }

    #[test]
    fn note_literal_assign_records_first_writer() {
        let mut s = EscapeState::default();
        s.note_literal_assign("n", "real_name");
        assert_eq!(s.resolve_literal("$n"), Some("real_name".into()));
        assert_eq!(s.resolve_literal("${n}"), Some("real_name".into()));
    }

    #[test]
    fn note_literal_assign_invalidates_on_second_write() {
        let mut s = EscapeState::default();
        s.note_literal_assign("n", "first");
        s.note_literal_assign("n", "second");
        assert_eq!(s.resolve_literal("$n"), None);
    }

    #[test]
    fn invalidate_literal_drops_tracking() {
        let mut s = EscapeState::default();
        s.note_literal_assign("n", "first");
        s.invalidate_literal("n");
        assert_eq!(s.resolve_literal("$n"), None);
    }

    #[test]
    fn resolve_literal_rejects_dynamic_inside_braces() {
        let mut s = EscapeState::default();
        s.note_literal_assign("n", "real");
        assert_eq!(s.resolve_literal("${$inner}"), None);
        assert_eq!(s.resolve_literal("${[cmd]}"), None);
    }

    #[test]
    fn resolve_literal_rejects_non_identifier_value() {
        let mut s = EscapeState::default();
        s.note_literal_assign("n", "has space");
        assert_eq!(s.resolve_literal("$n"), None);
    }

    #[test]
    fn record_upvar_source_collects_caller_names() {
        let mut s = EscapeState::default();
        s.record_upvar_source("caller_var");
        s.record_upvar_source("");
        assert!(s.upvar_source_names.contains("caller_var"));
        assert_eq!(s.upvar_source_names.len(), 1);
    }

    #[test]
    fn record_callee_collects_qualified_names() {
        let mut s = EscapeState::default();
        s.record_callee("::ns::helper");
        s.record_callee("");
        assert!(s.direct_callees.contains("::ns::helper"));
        assert_eq!(s.direct_callees.len(), 1);
    }

    #[test]
    fn fallback_flags_are_independent() {
        let mut s = EscapeState::default();
        s.record_fallback();
        assert!(s.has_fallback());
        assert!(!s.has_call_fallback());
        s.record_call_fallback();
        assert!(s.has_call_fallback());
    }
}
