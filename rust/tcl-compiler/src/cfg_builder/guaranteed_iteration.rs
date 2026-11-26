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

//! Loops whose body provably runs at least once, and `foreach` loops that never run.

use super::CfgBuilder;
use super::flow_facts::structural_write_roots;
use crate::ir::Statement;

// Guaranteed-iteration loops
//
// A loop whose body provably runs at least once does not skip its body, so a
// variable the body assigns is defined when code after the loop reads it.  The
// usual CFG models every loop as possibly running zero times (the header's exit
// edge), false-firing W210 on such a read.  In analysis builds we *rotate* a
// provably-non-empty loop so the 0-iteration skip becomes a *separate*
// entry-guard edge whose condition is statically true; SCCP marks that edge dead
// and the FP-RBS-16 dead-edge phi filter then ignores the version-0 operand it
// carried — with no synthetic def, so SCCP values are untouched.  `break` /
// `continue` stay real CFG edges, so partial-def exits remain sound.

/// True when `text` is a static, non-empty Tcl list literal.
///
/// Conservative: any `$` / `[` (a possible substitution) disqualifies it, so a
/// runtime-computed list is never claimed non-empty.  `foreach` stores a braced
/// list with its outer braces stripped (`{1 2 3}` → `1 2 3`), so a literal
/// splits directly.
fn list_literal_nonempty(text: &str, rules: tcl_syntax::word_rules::WordValueRules) -> bool {
    if text.is_empty() || text.contains('$') || text.contains('[') {
        return false;
    }
    !crate::tcl_expr_eval::split_tcl_list(text, rules).is_empty()
}

/// True when a `foreach`/`lmap` provably iterates ≥1 time.
///
/// `foreach` runs `max` over its iterator groups, so *any* non-empty iterator
/// list guarantees at least one iteration (shorter lists just pad their loop
/// vars with `""`).
pub(crate) fn foreach_runs_at_least_once(
    stmt: &Statement,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> bool {
    let Statement::Foreach { iterators, .. } = stmt else {
        return false;
    };
    iterators
        .iter()
        .any(|it| list_literal_nonempty(&it.list_arg, rules))
}

/// True when a `foreach`/`lmap` provably never iterates: every iterator list
/// is a literal with no elements.
pub(crate) fn foreach_never_runs(
    stmt: &Statement,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> bool {
    let Statement::Foreach { iterators, .. } = stmt else {
        return false;
    };
    !iterators.is_empty()
        && iterators.iter().all(|it| {
            !it.list_arg.contains('$')
                && !it.list_arg.contains('[')
                && crate::tcl_expr_eval::split_tcl_list(&it.list_arg, rules).is_empty()
        })
}

impl CfgBuilder<'_> {
    /// Names a non-`AssignConst` init statement may write.  `None` means "can't
    /// tell" — the caller then drops *all* constant bindings (a write it can't
    /// characterise might clobber any of them).
    ///
    /// A `Call`'s write set includes the caller-side variables an `upvar`-using
    /// callee modifies (direct call or `[upvar_proc …]` embedded substitution),
    /// recovered via [`Self::apply_upvar_invalidation`] — the module upvar pass
    /// adds these to a call's `defs`, but they are absent from the raw IR the
    /// init clause carries.  Without this, `for {set i 0; setter} {$i < 3} …`
    /// where `setter` runs `upvar 1 i i; set i 5` would keep the stale
    /// `i = 0` binding and be wrongly judged guaranteed.
    pub(super) fn init_written_names(&self, stmt: &Statement) -> Option<Vec<String>> {
        let mut names: Vec<String> = match stmt {
            Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. } => structural_write_roots(stmt)?,
            // A plain call contributes only the (augmented) defs collected
            // below.
            Statement::Call { .. } => Vec::new(),
            // Any other statement shape in the init (a nested if/loop/…) is not
            // characterised here — clear every constant binding to stay sound.
            _ => return None,
        };
        for s in self.upvar_invalidated(stmt.clone()) {
            // A widening barrier means the callee can write ANY caller
            // variable (an `upvar` with a dynamic caller-side name) — no
            // per-name set exists, so report "can't tell" and let the
            // caller drop every constant binding.
            if matches!(s, Statement::Barrier { .. }) {
                return None;
            }
            if let Statement::Call { defs, .. } = &s {
                for d in defs {
                    if !names.contains(d) {
                        names.push(d.clone());
                    }
                }
            }
        }
        Some(names)
    }

    /// True when a `for` loop's condition is statically true on entry.
    ///
    /// Evaluates the condition against the constant bindings the init clause
    /// establishes (`for {set i 0} {$i < 3} …` → `i = 0` → `0 < 3` is true),
    /// which proves the first iteration always runs.  Init statements are
    /// processed in order: an `AssignConst` (re)binds a constant, but any
    /// *other* write *invalidates* that variable's binding — so
    /// `for {set i 0; set i $n} …` and `for {set i 0; incr i 5} …` leave `i`
    /// unknown rather than stale-constant `0`, and an `upvar`-writing call in
    /// the init invalidates the var it writes through the caller frame.
    /// Conservative: a condition referencing an unbound variable (or
    /// a command-substitution condition) evaluates to `None` → not guaranteed.
    pub(crate) fn for_runs_at_least_once(&self, stmt: &Statement) -> bool {
        use crate::tcl_expr_eval::TclValue;
        let Statement::For {
            init, condition, ..
        } = stmt
        else {
            return false;
        };
        let mut env: crate::tcl_expr_eval::Env = std::collections::HashMap::new();
        for s in &init.statements {
            if let Statement::AssignConst { value, .. } = s {
                let projection = crate::ir_helpers::structural_variable_write_projection(s);
                if projection.opaque_variable_frame {
                    env.clear();
                } else {
                    for name in projection.literal_names {
                        if let Some((root, _)) = tcl_syntax::naming::split_element_ref(&name) {
                            env.remove(root);
                        } else {
                            env.insert(name, coerce_scalar(value));
                        }
                    }
                }
                continue;
            }
            match self.init_written_names(s) {
                // Unknown write — every prior constant binding is now suspect.
                None => env.clear(),
                Some(names) => {
                    for n in names {
                        env.remove(&n);
                    }
                }
            }
        }
        match crate::tcl_expr_eval::eval_tcl_expr_with_policy(
            condition,
            &env,
            crate::tcl_expr_eval::FoldPolicy::from_registry(self.registry),
        ) {
            Some(TclValue::Int(i)) => i != 0,
            Some(TclValue::Float(f)) => f != 0.0,
            // A bignum is canonical (beyond i64), hence never zero.
            Some(TclValue::Big(_)) => true,
            None => false,
        }
    }
}

/// Coerce a literal assignment value to int/float when possible (else str),
/// for the `for`-init constant environment.
fn coerce_scalar(text: &str) -> crate::tcl_expr_eval::EnvValue {
    use crate::tcl_expr_eval::{EnvValue, TclValue};
    if let Ok(i) = text.parse::<i64>() {
        return EnvValue::Number(TclValue::Int(i));
    }
    if let Ok(f) = text.parse::<f64>() {
        return EnvValue::Number(TclValue::Float(f));
    }
    EnvValue::text(text)
}
