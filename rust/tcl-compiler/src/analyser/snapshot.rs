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

//! Analyser snapshot / restore.
//!
//! Captures cumulative analyser state at a chunk boundary so the
//! analyser can resume from a checkpoint when only later chunks
//! have changed. Used by the LSP for incremental document
//! re-analysis — the analyser snapshots after every top-level
//! command, then restores the matching prefix when the user
//! types into the document.
//!
//! ``const_strings`` / ``regex_vars`` are keyed on a ``Vec<usize>``
//! path through ``result.global_scope.children``, so the path
//! remains valid across a deep copy without any pointer remapping.

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use super::state::{Analyser, CmdCommandSite, VarCommandSite};
use super::types::AnalysisResult;

/// Captured analyser state for chunked re-analysis.
///
/// Carries an independent deep copy of the [`AnalysisResult`] tree
/// plus the per-walk fields the analyser would otherwise lose on
/// restoration:
///
/// - ``current_scope_path`` — index path through the result tree
///   pointing at the active scope when the snapshot was taken
///   (the scope tree has no back-pointers).
/// - ``last_comment`` — most recent doc-comment for the next
///   ``proc`` / ``oo::class create`` definition.
/// - ``current_event`` — enclosing ``when EVENT`` for iRules.
/// - ``conditional_depth`` — depth of the ``if`` / ``catch`` /
///   ``try`` nesting; used to mark ``package require`` records as
///   ``conditional=true``.
/// - ``control_flow_body_depth`` — depth of nesting inside any
///   ``Traits::CONTROL_FLOW`` command's body; used to tell a
///   straight-line `rename` from one that may never run.
/// - ``command_aliases`` — `interp alias` table.
/// - ``renamed_commands`` — static `rename` table.
/// - ``const_strings`` / ``regex_vars`` — per-scope const-string
///   tracker and regex-var set.
/// - ``var_command_sites`` / ``cmd_command_sites`` — deferred
///   variable-as-command and command-sub-as-command call sites
///   (resolved post-walk by W307).
#[derive(Debug, Clone, Default)]
pub struct AnalyserSnapshot {
    /// Captured result tree (deep-copied).
    pub result: AnalysisResult,
    /// Path through ``result.global_scope`` to the active scope.
    pub current_scope_path: Vec<usize>,
    /// Last comment seen by the analyser.
    pub last_comment: String,
    /// Enclosing ``when EVENT`` name for iRules.
    pub current_event: Option<String>,
    /// Conditional-nesting depth.
    pub conditional_depth: u32,
    /// Nesting depth inside a `Traits::CONTROL_FLOW` command's body.
    pub control_flow_body_depth: u32,
    /// Command aliases: ``name -> (target, prepended_args)``.
    pub command_aliases: HashMap<String, (String, Vec<String>)>,
    /// Static renames: ``new_qname -> old_qname``.
    pub renamed_commands: HashMap<String, String>,
    /// Per-scope const-string tracker.
    pub const_strings: HashMap<Vec<usize>, HashMap<String, (String, Span)>>,
    /// Per-scope set of const-string names whose current value was written
    /// conditionally (see [`Analyser::nondominating_consts`]).
    pub nondominating_consts: HashMap<Vec<usize>, HashSet<String>>,
    /// Variables known to contain regex patterns.
    pub regex_vars: HashSet<(Vec<usize>, String)>,
    /// Variable-as-command call sites collected during the walk.
    pub var_command_sites: Vec<VarCommandSite>,
    /// Command-substitution-as-command call sites.
    pub cmd_command_sites: Vec<CmdCommandSite>,
    /// Pending E002 / E003 arity candidates (flushed post-walk).
    /// Snapshotted with `result` so a speculative rollback discards
    /// the candidates of any rolled-back commands.
    pub pending_arity: Vec<(String, String, bool, super::types::Diagnostic)>,
    /// Pending same-file user-call arity candidates (flushed post-walk
    /// alongside `pending_arity`). Snapshotted for the same rollback
    /// reason.
    pub pending_user_call_arity: Vec<super::types::PendingUserCallArity>,
    /// Pending `TclOO` constructor-call arity candidates (flushed post-walk
    /// alongside `pending_arity`). Snapshotted for the same rollback reason.
    pub pending_ctor_arity: Vec<super::types::PendingCtorArity>,
    /// Pending `TclOO` `next` / `nextto` call-site arity candidates
    /// (flushed post-walk alongside `pending_arity`). Snapshotted for the
    /// same rollback reason.
    pub pending_next_arity: Vec<super::types::PendingNextArity>,
}

impl Analyser {
    /// Capture the current analyser state for later restoration.
    ///
    /// Deep-copies the result tree (via [`AnalysisResult::clone`],
    /// which recursively clones all owned fields) and shallow-copies
    /// the per-walk tracking state (every field is owned, so
    /// shallow clone is independent).
    ///
    /// Per-call cost is dominated by the result-tree clone — the
    /// scope tree, proc / class / variable maps, and diagnostic
    /// vector all get walked. For chunked re-analysis the
    /// analyser snapshots after every top-level command, so the
    /// allocator pressure is meaningful but bounded.
    #[must_use]
    pub fn snapshot(&self) -> AnalyserSnapshot {
        AnalyserSnapshot {
            result: self.result.clone(),
            current_scope_path: self.current_scope_path.clone(),
            last_comment: self.last_comment.clone(),
            current_event: self.current_event.clone(),
            conditional_depth: self.conditional_depth,
            control_flow_body_depth: self.control_flow_body_depth,
            command_aliases: self.command_aliases.clone(),
            renamed_commands: self.renamed_commands.clone(),
            const_strings: self.const_strings.clone(),
            nondominating_consts: self.nondominating_consts.clone(),
            regex_vars: self.regex_vars.clone(),
            var_command_sites: self.var_command_sites.clone(),
            cmd_command_sites: self.cmd_command_sites.clone(),
            pending_arity: self.pending_arity.clone(),
            pending_user_call_arity: self.pending_user_call_arity.clone(),
            pending_ctor_arity: self.pending_ctor_arity.clone(),
            pending_next_arity: self.pending_next_arity.clone(),
        }
    }

    /// Restore analyser state from a snapshot.
    ///
    /// Replaces the result tree with an independent deep copy of the
    /// snapshot and resets the per-walk tracking state.
    ///
    /// Fields not in the snapshot (``disabled_diagnostics``,
    /// ``file_path``, ``builtin_names`` cache, ``body_depth``,
    /// ``ensemble_namespaces``, ``objdefined_vars``,
    /// ``unresolved_commands_emitted``) are *not* restored —
    /// they're either stable across the analyser's lifetime
    /// (``disabled_diagnostics`` / ``file_path``) or will be
    /// rebuilt as the walk continues from the restored point.
    pub fn restore(&mut self, snap: AnalyserSnapshot) {
        self.result = snap.result;
        self.current_scope_path = snap.current_scope_path;
        self.last_comment = snap.last_comment;
        self.current_event = snap.current_event;
        self.conditional_depth = snap.conditional_depth;
        self.control_flow_body_depth = snap.control_flow_body_depth;
        self.command_aliases = snap.command_aliases;
        self.renamed_commands = snap.renamed_commands;
        self.const_strings = snap.const_strings;
        self.nondominating_consts = snap.nondominating_consts;
        self.regex_vars = snap.regex_vars;
        self.var_command_sites = snap.var_command_sites;
        self.cmd_command_sites = snap.cmd_command_sites;
        self.pending_arity = snap.pending_arity;
        self.pending_user_call_arity = snap.pending_user_call_arity;
        self.pending_ctor_arity = snap.pending_ctor_arity;
        self.pending_next_arity = snap.pending_next_arity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::{ProcDef, Scope, ScopeKind, VarDef};

    fn proc(name: &str) -> ProcDef {
        ProcDef {
            name: name.to_string(),
            qualified_name: format!("::{name}"),
            params: Vec::new(),
            name_span: Span::new(0, 0),
            body_span: Span::new(0, 0),
            doc: String::new(),
            param_traits: std::collections::HashMap::new(),
            caller_frame_params: std::collections::HashSet::new(),
        }
    }

    fn var(name: &str) -> VarDef {
        VarDef {
            name: name.to_string(),
            definition_span: Span::new(0, 0),
            references: Vec::new(),
            warn_if_unused: true,
            array_indices: std::collections::BTreeSet::new(),
            link_target: None,
            link_target_span: None,
        }
    }

    #[test]
    fn snapshot_of_empty_analyser_is_default() {
        let a = Analyser::new();
        let snap = a.snapshot();
        assert!(snap.result.all_procs.is_empty());
        assert!(snap.current_scope_path.is_empty());
        assert!(snap.last_comment.is_empty());
        assert!(snap.current_event.is_none());
        assert_eq!(snap.conditional_depth, 0);
        assert!(snap.command_aliases.is_empty());
        assert!(snap.const_strings.is_empty());
        assert!(snap.regex_vars.is_empty());
        assert!(snap.var_command_sites.is_empty());
        assert!(snap.cmd_command_sites.is_empty());
    }

    #[test]
    fn restore_returns_analyser_to_snapshot_state() {
        let mut a = Analyser::new();
        // Take an initial snapshot.
        let snap = a.snapshot();
        // Mutate state.
        a.result.all_procs.insert("::foo".to_string(), proc("foo"));
        a.last_comment = "doc".to_string();
        a.conditional_depth = 3;
        a.command_aliases
            .insert("alias".to_string(), ("target".to_string(), vec![]));
        // Restore.
        a.restore(snap);
        // State is back to empty.
        assert!(a.result.all_procs.is_empty());
        assert!(a.last_comment.is_empty());
        assert_eq!(a.conditional_depth, 0);
        assert!(a.command_aliases.is_empty());
    }

    #[test]
    fn snapshot_restore_preserves_populated_state() {
        let mut a = Analyser::new();
        a.result.all_procs.insert("::foo".to_string(), proc("foo"));
        a.result
            .global_scope
            .variables
            .insert("x".to_string(), var("x"));
        a.last_comment = "first".to_string();
        a.conditional_depth = 2;
        a.const_strings
            .entry(vec![])
            .or_default()
            .insert("k".to_string(), ("v".to_string(), Span::new(1, 2)));

        let snap = a.snapshot();

        // Mutate after snapshot.
        a.result.all_procs.insert("::bar".to_string(), proc("bar"));
        a.last_comment = "second".to_string();
        a.conditional_depth = 9;
        a.const_strings.clear();

        // Restore — state matches the snapshot, not the mutated value.
        a.restore(snap);
        assert!(a.result.all_procs.contains_key("::foo"));
        assert!(!a.result.all_procs.contains_key("::bar"));
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert_eq!(a.last_comment, "first");
        assert_eq!(a.conditional_depth, 2);
        assert!(a.const_strings.contains_key(&vec![]));
    }

    #[test]
    fn snapshot_is_independent_of_live_state() {
        // Mutating the live analyser after snapshotting must not
        // affect the snapshot's data — it's a deep copy.
        let mut a = Analyser::new();
        a.result.all_procs.insert("::foo".to_string(), proc("foo"));
        let snap = a.snapshot();
        a.result.all_procs.insert("::bar".to_string(), proc("bar"));
        a.result.all_procs.remove("::foo");
        // Snapshot still has ::foo and not ::bar.
        assert!(snap.result.all_procs.contains_key("::foo"));
        assert!(!snap.result.all_procs.contains_key("::bar"));
    }

    #[test]
    fn restore_does_not_disturb_disabled_diagnostics() {
        // Sticky fields not captured by the snapshot survive restore.
        let disabled: HashSet<String> = ["W210"].iter().map(|s| (*s).to_string()).collect();
        let mut a = Analyser::with_disabled_diagnostics(disabled.clone());
        a.file_path = Some("/etc/tcl.conf".to_string());
        let snap = a.snapshot();
        // These fields are sticky — not in the snapshot.
        a.restore(snap);
        assert_eq!(a.disabled_diagnostics, disabled);
        assert_eq!(a.file_path.as_deref(), Some("/etc/tcl.conf"));
    }

    #[test]
    fn snapshot_path_round_trips() {
        // Index-path scope addressing — the path stays valid
        // across deep copy because we don't use pointer ids.
        let mut a = Analyser::new();
        a.current_scope_path = vec![0, 1, 2];
        // Build out a deeper scope tree.
        let inner = Scope::new(ScopeKind::Namespace, "inner");
        a.result.global_scope.children.push(inner);
        let snap = a.snapshot();
        // Mutate path and tree.
        a.current_scope_path = vec![];
        a.result.global_scope.children.clear();
        a.restore(snap);
        assert_eq!(a.current_scope_path, vec![0, 1, 2]);
        assert_eq!(a.result.global_scope.children.len(), 1);
        assert_eq!(a.result.global_scope.children[0].name, "inner");
    }
}
