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

//! SSA-version-aware escape accumulator.
//!
//! The intra-procedural state ([`super::super::state::EscapeState`])
//! tracks per-name tags. The CFG variant tags every escape at the
//! SSA version that is live at the statement so codegen can decide
//! per SSA value whether the underlying name needs frame storage.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ssa::{SsaFunction, SsaStatement, Version};
use crate::var_escape::helpers::is_dynamic_name;
use crate::var_escape::types::{EscapeFlags, EscapeTag};

/// Tracked literal binding for the alias-inference path.
#[derive(Debug, Clone)]
enum LiteralBinding {
    Set(String),
    Invalidated,
}

/// Result of the CFG+SSA flow-sensitive analysis.
///
/// `ssa_tags` is the authoritative per-version result;
/// `name_tags` collapses to per-name (a name is `Frame` if any of
/// its versions was tagged `Frame`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CfgEscapeResult {
    /// Per-(name, version) escape tags.
    pub ssa_tags: HashMap<(String, Version), EscapeTag>,
    /// Per-name collapse: `Frame` iff any version was tagged.
    pub name_tags: HashMap<String, EscapeTag>,
    /// Pessimism / fallback flag set.
    pub flags: EscapeFlags,
    /// Caller-frame names this proc reaches via `upvar`.
    pub upvar_source_names: BTreeSet<String>,
    /// Statically-resolvable callees.
    pub direct_callees: BTreeSet<String>,
    /// Every name the analysis saw (params + assigns + reads).
    pub known_names: HashSet<String>,
    /// Structured barrier triggers recorded by the CFG walk.
    pub barriers: Vec<crate::var_escape::types::Barrier>,
    /// Per-name escape reasons recorded by the CFG walk.
    pub tag_reasons: HashMap<String, Vec<crate::var_escape::types::EscapeReason>>,
}

impl CfgEscapeResult {
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
}

/// Mutable accumulator for the CFG walk.
///
/// Tracks per-SSA-version tags plus per-name auxiliary data.
/// Auxiliary data is monotonic — no per-block join state.
#[derive(Debug, Default)]
pub struct CfgState {
    /// Per-(name, version) escape tags.
    pub ssa_tags: HashMap<(String, Version), EscapeTag>,
    /// Pessimism / fallback flag set.
    pub flags: EscapeFlags,
    /// Names the analysis has seen.
    pub known_names: HashSet<String>,
    /// Caller-frame names this proc reaches via `upvar`.
    pub upvar_source_names: BTreeSet<String>,
    /// Statically-resolvable callees.
    pub direct_callees: BTreeSet<String>,
    /// Single-writer alias tracking (`set $n` resolution).
    literal_assigns: HashMap<String, LiteralBinding>,
    /// Most recent SSA version observed per name.
    ssa_for_name: HashMap<String, Version>,
    /// Structured barrier triggers.
    pub barriers: Vec<crate::var_escape::types::Barrier>,
    /// Per-name escape reasons.
    pub tag_reasons: HashMap<String, Vec<crate::var_escape::types::EscapeReason>>,
}

impl CfgState {
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

    /// Return the SSA version to tag for *name* at this statement.
    /// Prefers a definition on this line; falls back to the most
    /// recently observed version (default 0 when never seen).
    fn version_for(&self, name: &str, defs: &HashMap<String, Version>) -> Version {
        if let Some(v) = defs.get(name) {
            return *v;
        }
        self.ssa_for_name.get(name).copied().unwrap_or(0)
    }

    /// Update the name → latest-version map from a [`SsaStatement`].
    pub fn remember_versions(&mut self, stmt: &SsaStatement, ssa: &SsaFunction) {
        for (&sym, version) in &stmt.defs {
            let name = ssa.var_name(sym);
            self.ssa_for_name.insert(name.to_owned(), *version);
            self.known_names.insert(name.to_owned());
        }
        for (&sym, version) in &stmt.uses {
            let name = ssa.var_name(sym);
            // Defensive: keep the latest if an SSA quirk hands us a
            // higher version on a use than on the def map.
            let cur = self.ssa_for_name.get(name).copied().unwrap_or(0);
            if *version > cur {
                self.ssa_for_name.insert(name.to_owned(), *version);
            }
        }
    }

    /// Mark *name*'s current SSA version as `Frame`.
    pub fn escape(&mut self, name: &str, defs: &HashMap<String, Version>) {
        if name.is_empty() || is_dynamic_name(name) {
            return;
        }
        let version = self.version_for(name, defs);
        self.ssa_tags
            .insert((name.to_string(), version), EscapeTag::Frame);
        self.known_names.insert(name.to_string());
        self.ssa_for_name.entry(name.to_string()).or_insert(version);
    }

    /// Mark every known proc-local name as `Frame` at its current
    /// version.
    pub fn escape_all_known(&mut self, defs: &HashMap<String, Version>) {
        let names: Vec<String> = self.known_names.iter().cloned().collect();
        for name in names {
            if is_dynamic_name(&name) {
                continue;
            }
            let version = self.version_for(&name, defs);
            self.ssa_tags.insert((name, version), EscapeTag::Frame);
        }
    }

    /// Mark the whole proc as needing a dynamic-barrier fallback.
    pub fn mark_pessimistic(&mut self) {
        self.flags.insert(EscapeFlags::DYNAMIC_BARRIER);
    }

    /// Record a structured barrier
    /// (sets [`EscapeFlags::DYNAMIC_BARRIER`] AND appends the
    /// barrier to [`Self::barriers`] for downstream rendering).
    pub fn record_barrier(&mut self, barrier: crate::var_escape::types::Barrier) {
        self.flags.insert(EscapeFlags::DYNAMIC_BARRIER);
        self.barriers.push(barrier);
    }

    /// Escape *name* at its current
    /// SSA version and record the triggering reason.
    pub fn escape_with_reason(
        &mut self,
        name: &str,
        defs: &HashMap<String, Version>,
        reason: crate::var_escape::types::EscapeReason,
    ) {
        if name.is_empty() || is_dynamic_name(name) {
            return;
        }
        let version = self.version_for(name, defs);
        self.ssa_tags
            .insert((name.to_string(), version), EscapeTag::Frame);
        self.known_names.insert(name.to_string());
        self.ssa_for_name.entry(name.to_string()).or_insert(version);
        self.tag_reasons
            .entry(name.to_string())
            .or_default()
            .push(reason);
    }

    /// Record a literal caller-frame name reached via `upvar`.
    pub fn record_upvar_source(&mut self, src: &str) {
        if !src.is_empty() {
            self.upvar_source_names.insert(src.to_string());
        }
    }

    /// Record that the upvar source set cannot be enumerated.
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
    pub fn record_call_fallback(&mut self) {
        self.flags.insert(EscapeFlags::HAS_CALL_FALLBACK);
    }

    /// Track *name* = literal *value* under the single-writer rule.
    pub fn note_literal_assign(&mut self, name: &str, value: &str) {
        if name.is_empty() || is_dynamic_name(name) {
            return;
        }
        match self.literal_assigns.get(name) {
            Some(_) => {
                self.literal_assigns
                    .insert(name.to_string(), LiteralBinding::Invalidated);
            }
            None => {
                self.literal_assigns
                    .insert(name.to_string(), LiteralBinding::Set(value.to_string()));
            }
        }
    }

    /// Mark *name*'s tracked literal invalid.
    pub fn invalidate_literal(&mut self, name: &str) {
        if !name.is_empty() {
            self.literal_assigns
                .insert(name.to_string(), LiteralBinding::Invalidated);
        }
    }

    /// Resolve a `$n` / `${n}` token to the bound literal under
    /// the single-writer rule.
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
        if !literal
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
        {
            return None;
        }
        Some(literal.clone())
    }

    /// Convert the accumulated state into a [`CfgEscapeResult`].
    /// Collapses per-version tags to per-name tags.
    #[must_use]
    pub fn into_result(self) -> CfgEscapeResult {
        let mut name_tags: HashMap<String, EscapeTag> = HashMap::new();
        for ((name, _), tag) in &self.ssa_tags {
            if *tag == EscapeTag::Frame {
                name_tags.insert(name.clone(), EscapeTag::Frame);
            }
        }
        CfgEscapeResult {
            ssa_tags: self.ssa_tags,
            name_tags,
            flags: self.flags,
            upvar_source_names: self.upvar_source_names,
            direct_callees: self.direct_callees,
            known_names: self.known_names,
            barriers: self.barriers,
            tag_reasons: self.tag_reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs(pairs: &[(&str, Version)]) -> HashMap<String, Version> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn escape_tags_at_def_version() {
        let mut s = CfgState::default();
        s.escape("x", &defs(&[("x", 3)]));
        assert_eq!(s.ssa_tags.get(&("x".into(), 3)), Some(&EscapeTag::Frame));
    }

    #[test]
    fn escape_falls_back_to_latest_seen_version() {
        let mut s = CfgState::default();
        s.ssa_for_name.insert("x".into(), 5);
        s.escape("x", &HashMap::new());
        assert_eq!(s.ssa_tags.get(&("x".into(), 5)), Some(&EscapeTag::Frame));
    }

    #[test]
    fn escape_uses_zero_when_name_never_seen() {
        let mut s = CfgState::default();
        s.escape("brand_new", &HashMap::new());
        assert_eq!(
            s.ssa_tags.get(&("brand_new".into(), 0)),
            Some(&EscapeTag::Frame)
        );
    }

    #[test]
    fn escape_ignores_empty_and_dynamic_names() {
        let mut s = CfgState::default();
        s.escape("", &HashMap::new());
        s.escape("$dyn", &HashMap::new());
        assert!(s.ssa_tags.is_empty());
    }

    #[test]
    fn escape_all_known_tags_every_seeded_name() {
        let mut s = CfgState::new(["a".to_string(), "b".to_string()]);
        s.ssa_for_name.insert("a".into(), 2);
        s.ssa_for_name.insert("b".into(), 7);
        s.escape_all_known(&HashMap::new());
        assert_eq!(s.ssa_tags.get(&("a".into(), 2)), Some(&EscapeTag::Frame));
        assert_eq!(s.ssa_tags.get(&("b".into(), 7)), Some(&EscapeTag::Frame));
    }

    #[test]
    fn note_literal_invalidates_on_second_write() {
        let mut s = CfgState::default();
        s.note_literal_assign("n", "first");
        s.note_literal_assign("n", "second");
        assert_eq!(s.resolve_literal("$n"), None);
    }

    #[test]
    fn into_result_collapses_per_name_tags() {
        let mut s = CfgState::default();
        s.escape("x", &defs(&[("x", 1)]));
        s.escape("x", &defs(&[("x", 2)]));
        s.escape("y", &defs(&[("y", 0)]));
        let r = s.into_result();
        assert_eq!(r.name_tags.get("x"), Some(&EscapeTag::Frame));
        assert_eq!(r.name_tags.get("y"), Some(&EscapeTag::Frame));
        // Two SSA versions of x were tagged.
        assert!(r.ssa_tags.contains_key(&("x".into(), 1)));
        assert!(r.ssa_tags.contains_key(&("x".into(), 2)));
    }

    #[test]
    fn record_upvar_source_collects_caller_names() {
        let mut s = CfgState::default();
        s.record_upvar_source("caller_var");
        s.record_upvar_source("");
        assert!(s.upvar_source_names.contains("caller_var"));
        assert_eq!(s.upvar_source_names.len(), 1);
    }

    #[test]
    fn record_callee_collects_qualified_names() {
        let mut s = CfgState::default();
        s.record_callee("::ns::helper");
        assert!(s.direct_callees.contains("::ns::helper"));
    }

    #[test]
    fn fallback_flags_are_independent() {
        let mut s = CfgState::default();
        s.record_fallback();
        assert!(s.has_fallback());
        assert!(!s.has_call_fallback());
        s.record_call_fallback();
        assert!(s.has_call_fallback());
    }
}
