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

//! Cross-event variable scope analysis for iRules connection
//! lifecycles.
//!
//! iRules ``when`` event handlers share a connection-scoped Tcl
//! stack — variables set in one event persist until the
//! connection closes or the variable is explicitly ``unset``.
//! This module computes which local variables flow across
//! ``when`` event boundaries so that per-procedure diagnostics
//! (dead stores, read-before-set, unused variables) can be
//! suppressed when the variable is actually live across events.
//!
//! Used by:
//!
//! - `analyser/diagnostics.rs::emit_racy_static_diagnostics`
//!   (IRULE4005 — ``static::`` cross-event flow from a
//!   non-RULE_INIT event).
//! - The CFG/SSA RBS / unused-var emitters (cross-event
//!   suppression).  The analyser threads
//!   `connection_scope.cross_event_defs` /
//!   `cross_event_imports` through
//!   `emit_cfg_ssa_diagnostics_for_function` so a
//!   ``set::ip [IP::client_addr]`` in `CLIENT_ACCEPTED` that's
//!   read in `HTTP_REQUEST` is not falsely flagged as unused.

use std::collections::{HashMap, HashSet};

use tcl_registry::events::EventRegistry;

use crate::compilation_unit::FunctionUnit;
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::ir::{Statement, when_event_name};

/// Variable summary for a single ``when`` event handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventVarSummary {
    /// Event name (e.g. ``"CLIENT_ACCEPTED"``).
    pub event: String,
    /// Variable names defined (set) on at least one path
    /// through the event.
    pub defs: HashSet<String>,
    /// Variable names used at SSA version 0 (read before any
    /// local def).  These are candidate cross-event imports.
    pub uses_before_def: HashSet<String>,
    /// Variable names explicitly ``unset`` in this event.
    pub unsets: HashSet<String>,
}

/// Cross-event variable scope analysis result.
///
/// Built once from
/// the ``::when::*`` subset of `CompilationUnit::procedures`
/// and cached on `CompilationUnit::connection_scope`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConnectionScope {
    /// Per-event summaries keyed by event name.
    pub summaries: HashMap<String, EventVarSummary>,
    /// Variables defined in one event AND used-before-def in a
    /// different event.  Suppresses dead-store / unused-var
    /// diagnostics on the producer side.
    pub cross_event_defs: HashSet<String>,
    /// Variables used-before-def in one event AND defined in a
    /// different event.  Suppresses W210 on the consumer side.
    pub cross_event_imports: HashSet<String>,
    /// ``static::`` vars defined in a non-RULE_INIT event and
    /// used cross-event — feeds the **IRULE4005** racy-static
    /// emitter.
    pub racy_static_defs: HashSet<String>,
}

/// Build a [`ConnectionScope`] from compiled ``when``
/// procedures.
///
/// `when_procedures`
/// should be the subset of `CompilationUnit::procedures` whose
/// qualified names start with ``::when::``.
///
/// **Branch-condition vars.**  A dedicated branch-condition scan
/// (walking `Terminator::Branch.condition`) is omitted — branch-condition
/// vars at version 0 are already part of the SSA statement uses,
/// so the sweep is rarely load-bearing.
#[must_use]
pub fn build_connection_scope<S: std::hash::BuildHasher>(
    when_procedures: &HashMap<String, FunctionUnit, S>,
) -> ConnectionScope {
    let registry = EventRegistry::build();
    let mut summaries: HashMap<String, EventVarSummary> = HashMap::new();
    for (qname, fu) in when_procedures {
        let event = when_event_name(qname).to_string();
        let summary = extract_event_summary(&event, fu);
        // Multiple ``when EVENT`` handlers for the same event
        // get merged — union the def / use / unset sets.
        if let Some(prev) = summaries.get(&event) {
            let merged = EventVarSummary {
                event: event.clone(),
                defs: prev.defs.union(&summary.defs).cloned().collect(),
                uses_before_def: prev
                    .uses_before_def
                    .union(&summary.uses_before_def)
                    .cloned()
                    .collect(),
                unsets: prev.unsets.union(&summary.unsets).cloned().collect(),
            };
            summaries.insert(event, merged);
        } else {
            summaries.insert(event, summary);
        }
    }

    let mut cross_defs: HashSet<String> = HashSet::new();
    let mut cross_imports: HashSet<String> = HashSet::new();
    let mut racy_statics: HashSet<String> = HashSet::new();

    let events: Vec<&String> = summaries.keys().collect();
    for (i, ev_a) in events.iter().enumerate() {
        let sum_a = &summaries[ev_a.as_str()];
        for (j, ev_b) in events.iter().enumerate() {
            if i == j {
                continue;
            }
            let sum_b = &summaries[ev_b.as_str()];
            // Variables defined in A and used-before-def in B.
            for var in sum_a.defs.intersection(&sum_b.uses_before_def) {
                if registry.variable_scope_note(ev_a, ev_b).is_none() {
                    // No scoping concern → valid cross-event
                    // flow.
                    cross_defs.insert(var.clone());
                    cross_imports.insert(var.clone());
                    if var.starts_with("static::") && ev_a.as_str() != "RULE_INIT" {
                        racy_statics.insert(var.clone());
                    }
                }
            }
        }
    }

    ConnectionScope {
        summaries,
        cross_event_defs: cross_defs,
        cross_event_imports: cross_imports,
        racy_static_defs: racy_statics,
    }
}

/// Build a per-event variable summary from a compiled ``when``
/// procedure.
///
/// Walks the SSA
/// statements collecting:
///
/// - Names defined on at least one path (excluding global
///   ``::`` names and ``static::`` names destroyed by
///   ``unset``).
/// - Names used at SSA version 0 (i.e. read before any local
///   def — candidate cross-event imports).
/// - Names explicitly ``unset`` in this event.
///
/// Record every `info exists <name>` literal-variable read found in `text`
/// (the base name, namespace-global `::`-prefixed excluded — those aren't
/// connection-scoped). Catches the pattern wherever it appears: a bare
/// statement, a `[info exists …]` command substitution, or a branch condition.
pub(crate) fn scan_info_exists(text: &str, out: &mut HashSet<String>) {
    const NEEDLE: &str = "info exists";
    let mut search = text;
    while let Some(pos) = search.find(NEEDLE) {
        let after = search[pos + NEEDLE.len()..].trim_start();
        // The variable name runs until the first non-name byte (whitespace,
        // `(` array index, `]`/`}` closer, etc.). Keep the base (pre-`(`) name.
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .collect();
        let base = name.split('(').next().unwrap_or(&name);
        if !base.is_empty() && !base.starts_with("::") {
            out.insert(base.to_string());
        }
        search = &search[pos + NEEDLE.len()..];
    }
}

/// Walk an expression for embedded command substitutions / raw text and scan
/// each for `info exists` reads (branch conditions hold the `[info exists …]`
/// as a `Command` node).
fn scan_expr_info_exists(node: &crate::expr_ast::ExprNode, out: &mut HashSet<String>, depth: u32) {
    use crate::expr_ast::ExprNode;
    // Native-stack safety net (issue #996): walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the `info exists` reads gathered so far is the safe
    // fallback (reads buried deeper than the cap go unrecorded; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, .. } | ExprNode::Raw { text } => scan_info_exists(text, out),
        ExprNode::Binary { left, right, .. } => {
            scan_expr_info_exists(left, out, depth + 1);
            scan_expr_info_exists(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => scan_expr_info_exists(operand, out, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            scan_expr_info_exists(condition, out, depth + 1);
            scan_expr_info_exists(true_branch, out, depth + 1);
            scan_expr_info_exists(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                scan_expr_info_exists(a, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn extract_event_summary(event: &str, fu: &FunctionUnit) -> EventVarSummary {
    let mut defs: HashSet<String> = HashSet::new();
    let mut uses_v0: HashSet<String> = HashSet::new();
    let mut unsets: HashSet<String> = HashSet::new();

    for block in fu.ssa.blocks.values() {
        for stmt in &block.statements {
            let is_unset = matches!(
                &stmt.statement,
                Statement::Call { command, .. } if command == "unset"
            );

            for &sym in stmt.defs.keys() {
                let name = fu.ssa.var_name(sym);
                if name.starts_with("::") {
                    continue;
                }
                if !(name.starts_with("static::") && is_unset) {
                    defs.insert(name.to_owned());
                }
            }

            for (&sym, ver) in &stmt.uses {
                let name = fu.ssa.var_name(sym);
                if *ver == 0 && !name.starts_with("::") {
                    uses_v0.insert(name.to_owned());
                }
            }

            // Track unsets explicitly so the racy-static check
            // can ignore them.  It's the same condition as
            // ``is_unset`` above, but walking ``stmt.defs`` for
            // the names is the canonical shape.
            if is_unset && let Statement::Call { args, .. } = &stmt.statement {
                for a in args {
                    if !a.starts_with("::") && !a.starts_with('-') {
                        unsets.insert(a.clone());
                    }
                }
            }
        }
    }

    // `info exists VAR` reads VAR by *literal name*, not a `$`-substitution, so
    // the SSA never records it as a use — yet it observes cross-event state
    // (e.g. `if {[info exists ans_cleared]}` in DNS_RESPONSE reads a flag set in
    // DNS_REQUEST). Scan statement words AND branch conditions for the pattern
    // so the cross-event sweep keeps the producing store alive (else O126
    // deletes `set ans_cleared 1` and SCCP folds `info exists` to 0 — a
    // miscompile). Conservative: any `info exists <name>` counts as a use.
    for block in fu.cfg.blocks.values() {
        for stmt in &block.statements {
            match stmt {
                Statement::Call { args, .. } | Statement::Barrier { args, .. } => {
                    for a in args {
                        scan_info_exists(a.as_str(), &mut uses_v0);
                    }
                }
                Statement::AssignValue { value, .. } => {
                    scan_info_exists(value.as_str(), &mut uses_v0);
                }
                _ => {}
            }
        }
        if let Some(crate::cfg::Terminator::Branch { condition, .. }) = &block.terminator {
            scan_expr_info_exists(condition, &mut uses_v0, 0);
        }
    }

    EventVarSummary {
        event: event.to_string(),
        defs,
        uses_before_def: uses_v0,
        unsets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    /// Regression coverage for issue #996: `scan_expr_info_exists` recurses
    /// once per `ExprNode` level with no depth cap before this fix. A tree
    /// built directly is unbounded (the Pratt parser caps its own output at
    /// 256) and empirically overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread. 3000 is past that crash range
    /// and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is that it returns
    /// at all.
    #[test]
    fn deeply_nested_scan_expr_info_exists_survives() {
        use crate::expr_ast::{ExprNode, UnaryOp};
        let mut node = ExprNode::Command {
            text: "[info exists x]".into(),
            start: 0,
            end: 15,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let mut out = HashSet::new();
        scan_expr_info_exists(&node, &mut out, 0);
    }

    fn cu(source: &str) -> CompilationUnit {
        // `when` is registry-resolved.  This
        // module's tests all lower iRule code (`when
        // CLIENT_ACCEPTED { ... }`), so the test registry must
        // load iRules to make `when` resolve to `LoweringHookId
        // ::When`.  Without the load, `when` would fall through
        // to `lower_default` and no `::when::*` procedures would
        // be registered for the connection-scope builder to walk.
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        CompilationUnit::build_for(source, &registry, false)
    }

    fn when_procs(cu: &CompilationUnit) -> HashMap<String, FunctionUnit> {
        cu.procedures
            .iter()
            .filter(|(qn, _)| qn.starts_with("::when::"))
            .map(|(qn, fu)| (qn.clone(), fu.clone()))
            .collect()
    }

    #[test]
    fn build_connection_scope_empty_when_no_when_procs() {
        let cu = cu("proc foo {} {}");
        let cs = build_connection_scope(&when_procs(&cu));
        assert!(cs.summaries.is_empty());
        assert!(cs.cross_event_defs.is_empty());
        assert!(cs.cross_event_imports.is_empty());
        assert!(cs.racy_static_defs.is_empty());
    }

    #[test]
    fn build_connection_scope_records_defs_and_uses() {
        // Two when blocks: CLIENT_ACCEPTED writes ``ip``,
        // HTTP_REQUEST reads ``ip``.  Cross-event flow is
        // valid (CLIENT_ACCEPTED fires before HTTP_REQUEST).
        let source = "
            when CLIENT_ACCEPTED { set ip [IP::client_addr] }
            when HTTP_REQUEST { log local0. \"$ip\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        // ``ip`` is in cross_event_imports / cross_event_defs.
        assert!(cs.cross_event_defs.contains("ip"));
        assert!(cs.cross_event_imports.contains("ip"));
        // No static:: var ⇒ no racy_static_defs.
        assert!(cs.racy_static_defs.is_empty());
    }

    #[test]
    fn build_connection_scope_flags_racy_static() {
        // ``static::counter`` written in HTTP_REQUEST and read
        // in HTTP_RESPONSE — both per-request events; the
        // cross-event flow is racy.
        let source = "
            when HTTP_REQUEST { incr static::counter }
            when HTTP_RESPONSE { log local0. \"$static::counter\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        assert!(cs.racy_static_defs.contains("static::counter"));
    }

    #[test]
    fn build_connection_scope_skips_rule_init_static() {
        // ``static::`` written in RULE_INIT is **not** racy —
        // RULE_INIT runs once at iRule load.
        let source = "
            when RULE_INIT { set static::config 1 }
            when HTTP_REQUEST { log local0. \"$static::config\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        assert!(!cs.racy_static_defs.contains("static::config"));
    }

    #[test]
    fn build_connection_scope_merges_duplicate_events() {
        // Two ``when CLIENT_ACCEPTED`` blocks — their summaries
        // are merged.
        let source = "
            when CLIENT_ACCEPTED { set a 1 }
            when CLIENT_ACCEPTED { set b 2 }
            when HTTP_REQUEST { log local0. \"$a $b\" }
        ";
        let cu = cu(source);
        let cs = build_connection_scope(&when_procs(&cu));
        let s = cs
            .summaries
            .get("CLIENT_ACCEPTED")
            .expect("CLIENT_ACCEPTED summary");
        assert!(s.defs.contains("a") && s.defs.contains("b"));
    }
}
