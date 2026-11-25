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

//! Evaluating the statements of one block: definitions, barriers, calls to unseen code and statements that certainly raise.

use super::super::evaluation::{DefValues, RaisedDefs, evaluate_defs_under};
pub(super) use super::super::statement_transfer::held_value;
use super::super::statement_transfer::{
    BarrierHistory, FreshClobber, transfer_fresh_clobbers, widen_barrier_values, widen_held_values,
};
use super::super::{join, set_value};
use super::loop_enumeration::catch_body_answer;
use super::{BlockExit, ExistenceAt, StatementInputs, StatementsRun, is_externally_mutable};
use crate::analyses::LatticeValue;
use crate::cfg::BlockId;
use crate::ir::Statement;
use crate::ssa::{SsaFunction, SsaStatement, Symbol, ValueKey};
use crate::value_transfer::LatticeDriver;
use std::collections::{HashMap, HashSet};
use tcl_registry::value_transfer::Existence;

fn process_early_statement(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (ssa_block, index): (&crate::ssa::SsaBlock, usize),
    stmt_ssa: &crate::ssa::SsaStatement,
    inputs: StatementInputs<'_>,
    completes: bool,
    (driver, existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> Option<bool> {
    let clobbers = inputs.clobbers;
    if crate::ssa::is_unseen_call_marker(&stmt_ssa.statement) {
        return Some(clobber_at_unseen_call(
            values,
            (index, clobbers, completes),
            (driver, existence),
        ));
    }
    // A statement after one that certainly raises never runs: its
    // definitions keep what their places held.
    if !completes {
        return Some(keep_prior(
            values,
            (ssa_block, index),
            stmt_ssa,
            (driver, existence),
        ));
    }
    None
}

fn widen_source_may_statement(
    (values, stmt_ssa): (
        &mut HashMap<ValueKey, LatticeValue>,
        &crate::ssa::SsaStatement,
    ),
    (block, index): (BlockId, usize),
    (ssa, driver, mut existence): (
        &SsaFunction,
        &LatticeDriver<'_>,
        Option<&mut ExistenceAt<'_>>,
    ),
) -> bool {
    let mut changed = false;
    driver.record_raised((block, index), false);
    for (&symbol, &version) in &stmt_ssa.defs {
        changed |= set_value(values, (symbol, version), &LatticeValue::Overdefined);
        driver.record_folded((symbol, version), None);
        driver.record_preserved((symbol, version), None);
        if let Some(at) = existence.as_deref_mut() {
            changed |= at.advance(
                (symbol, version),
                crate::value_transfer::ExistenceStep::UNKNOWN,
                driver,
            );
        }
    }
    if !ssa.source_may_clobber_is_current_at(block, index) {
        // A changed CFG cannot retain a partial source footprint as
        // permission to preserve the rest of the frame.
        changed |= widen_held_values(values, |_| true);
        driver.forget_folded();
        driver.existence_clobber(|_| Existence::MayBound);
    }
    if let Some(at) = existence {
        at.finish_statement(index, driver);
    }
    changed
}

/// Evaluate each statement's defs for one block, widening across barriers.
/// Returns `true` if any lattice value changed. Extracted from [`sccp`].
///
/// With the existence rung, each statement also records the facts it
/// reads, advances each place it defines by its storage outcome — the
/// externally mutable ones stay may-bound — and applies its clobber.
pub(in super::super) fn sccp_process_statements(
    values: &mut HashMap<ValueKey, LatticeValue>,
    ssa_block: &crate::ssa::SsaBlock,
    ssa: &SsaFunction,
    inputs: StatementInputs<'_>,
    driver: &LatticeDriver<'_>,
    mut existence: Option<&mut ExistenceAt<'_>>,
) -> StatementsRun {
    let StatementInputs {
        block,
        escaping,
        has_dynamic_variable_trace,
        catches,
        ..
    } = inputs;
    let (mut changed, mut prepared) = (false, None);
    let mut exit = BlockExit::NORMAL;
    // The existence a marker for what a statement's scripts may write finds,
    // which is where those scripts start: the statement after the marker is
    // evaluated over it.
    let mut before_marker = None;
    for (index, stmt_ssa) in ssa_block.statements.iter().enumerate() {
        // The fresh versions a call to code the module cannot see leaves take
        // effect after the inputs and the barrier of the call they stand
        // beside.
        if let Some(progress) = process_early_statement(
            values,
            (ssa_block, index),
            stmt_ssa,
            inputs,
            exit.completes,
            (driver, existence.as_deref_mut()),
        ) {
            changed |= progress;
            continue;
        }
        if let Some(at) = existence.as_deref_mut() {
            at.record_reads(index, stmt_ssa, driver);
        }
        let scripts_start = before_marker.take();
        if crate::ssa::is_arm_writes_marker(&stmt_ssa.statement) {
            before_marker = driver.existence_state();
        }
        if matches!(
            stmt_ssa.statement,
            Statement::Barrier { .. } | Statement::UpFrame { .. }
        ) {
            changed |= widen_at_barrier(
                values,
                (index, stmt_ssa),
                inputs.clobbers,
                (driver, existence.as_deref_mut()),
            );
            continue;
        }
        if ssa.source_may_requires_unknown_at(block, index) {
            prepared = None;
            changed |= widen_source_may_statement(
                (values, stmt_ssa),
                (block, index),
                (ssa, driver, existence.as_deref_mut()),
            );
            continue;
        }
        // An opaque `catch` body the enumeration runs gives the names it
        // writes what it leaves, before the marker that states them widens.
        let body_run = catches.and_then(|catches| {
            catch_body_answer(
                (ssa_block, index),
                (values, ssa),
                (driver, catches),
                before_marker.as_deref(),
            )
        });
        let _entry = driver.body_entry_scope(scripts_start);
        // The statement is evaluated once, when a definition first needs
        // it: a call's ordered stores give each definition its own value.
        let mut evaluated = body_run
            .or_else(|| {
                driver.evaluate_catch_end((ssa_block, index), values, ssa, |entry| {
                    existence
                        .as_deref()
                        .and_then(|at| at.run.exits.get(&entry).cloned())
                })
            })
            .or_else(|| pair_answer(&mut prepared, (ssa_block, index), values, (ssa, driver)))
            .or_else(|| driver.procedure_answer((ssa_block, index), values, ssa));
        // Where a raise ends the path, whether the statement raises is part
        // of what it does, so it is evaluated whatever its definitions need.
        if driver.raises_end_paths() && evaluated.is_none() {
            evaluated = if stmt_ssa.defs.is_empty() {
                driver.probe_completion(stmt_ssa, values, ssa)
            } else {
                Some(evaluate_defs_under(stmt_ssa, values, ssa, driver))
            };
        }
        let raised = raised_writes(evaluated.as_ref());
        driver.record_raised((block, index), raised.is_some());
        let site = StatementSite {
            block: ssa_block,
            index,
            ssa,
            escaping,
            has_dynamic_variable_trace,
            driver,
        };
        changed |= define_values(values, &site, evaluated, existence.as_deref_mut());
        if let Some(written) = raised {
            exit = BlockExit {
                completes: false,
                entry_throws: index > 0 || written == 0,
            };
        }
        if let Some(at) = existence.as_deref_mut() {
            at.finish_statement(index, driver);
        }
    }
    StatementsRun { changed, exit }
}

/// The fresh versions the marker for a call to code the module cannot see at
/// `index` gives the names live after it
/// ([`crate::ssa::SsaFunction::value_clobbers`]): that code may have rewritten
/// each, so each states no value and, with the rest of the frame, may be bound
/// or not — save a parameter of a procedure a caller proved pure, whose seed
/// no such code can rebind. After a statement that certainly raised the
/// marker was never reached, so each keeps what its place held. Whether
/// anything moved.
pub(super) fn clobber_at_unseen_call(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (index, clobbers, completes): (usize, Option<&crate::ssa::BlockValueClobbers>, bool),
    (driver, existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> bool {
    let fresh_versions = clobbers.and_then(|markers| markers.get(&index));
    let policy = if completes {
        FreshClobber::Reached {
            protected_parameters: driver.proven_pure_parameters(),
        }
    } else {
        FreshClobber::Unreached
    };
    let mut changed = transfer_fresh_clobbers(values, fresh_versions, policy, |key, kept| {
        driver.record_folded(key, kept.and_then(|prior| driver.folded_of(prior)));
    });
    if let Some(at) = existence {
        if completes {
            at.finish_statement(index, driver);
        }
        let step = if completes {
            crate::value_transfer::ExistenceStep::Set(Existence::MayBound)
        } else {
            crate::value_transfer::ExistenceStep::PRESERVE
        };
        for (&var, &(_, fresh)) in fresh_versions.into_iter().flatten() {
            changed |= at.advance((var, fresh), step, driver);
        }
    }
    changed
}

/// What a barrier or an up-frame statement does to the values: every value
/// is widened and the statement's own definitions are opaque. Whether a value
/// moved.
fn widen_at_barrier(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (index, stmt_ssa): (usize, &SsaStatement),
    clobbers: Option<&crate::ssa::BlockValueClobbers>,
    (driver, mut existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> bool {
    let mut changed = stmt_ssa.statement.is_opaque_script_body()
        && clobber_at_unseen_call(
            values,
            (index, clobbers, true),
            (driver, existence.as_deref_mut()),
        );
    // A return leaves the frame before any later effects could run.
    if stmt_ssa.statement.is_return_barrier() {
        return changed;
    }
    // A widened value carries no folded type. `UpFrame` can reassign names
    // in another frame visible to this procedure and widens in the same way.
    changed |= widen_barrier_values(values, &stmt_ssa.defs, BarrierHistory::WidenNonEntry);
    driver.forget_folded();
    // Every place is may-bound after a barrier, as every value is
    // widened.
    if let Some(at) = existence {
        changed |= at.barrier(index, stmt_ssa, driver);
    }
    changed
}

/// The statement a block's sweep is defining the values of, with what it is
/// evaluated under.
struct StatementSite<'a> {
    pub(in super::super) block: &'a crate::ssa::SsaBlock,
    pub(in super::super) index: usize,
    pub(in super::super) ssa: &'a SsaFunction,
    pub(super) escaping: &'a HashSet<String>,
    pub(in super::super) has_dynamic_variable_trace: bool,
    pub(in super::super) driver: &'a LatticeDriver<'a>,
}

/// How many stores ran before the statement certainly raised, when it did.
fn raised_writes(evaluated: Option<&DefValues>) -> Option<usize> {
    match evaluated {
        Some(DefValues::Raised(raised)) => Some(raised.written),
        _ => None,
    }
}

/// Each definition of the statement at `site`: its value, folded type and
/// the version it preserved, and with the existence rung its step. `evaluated`
/// is the statement's evaluation when one was made ahead; a definition that
/// needs one makes it. Whether a value moved.
fn define_values(
    values: &mut HashMap<ValueKey, LatticeValue>,
    site: &StatementSite<'_>,
    mut evaluated: Option<DefValues>,
    mut existence: Option<&mut ExistenceAt<'_>>,
) -> bool {
    let StatementSite {
        block: ssa_block,
        index,
        ssa,
        escaping,
        has_dynamic_variable_trace,
        driver,
    } = *site;
    let stmt_ssa = &ssa_block.statements[index];
    let mut changed = false;
    // An element write's base def carries no scalar value of its own —
    // `set arr(k) 5` / `set arr($i) 5` refresh `arr` for whole-array
    // readers but must never let `$arr` fold to the element's value.
    let element_write_base = typed_element_write_base(&stmt_ssa.statement, ssa);
    let raised = raised_writes(evaluated.as_ref());
    let keeps_all =
        matches!(&evaluated, Some(DefValues::Raised(raised)) if raised.prefix.is_none());
    for (&var, &ver) in &stmt_ssa.defs {
        let mut value_of = |values: &HashMap<ValueKey, LatticeValue>| {
            let evaluated =
                evaluated.get_or_insert_with(|| evaluate_defs_under(stmt_ssa, values, ssa, driver));
            (
                evaluated.of((var, ver)),
                evaluated.folded_of((var, ver)),
                evaluated.stated((var, ver)),
                evaluated.preserved((var, ver)),
                evaluated.existence((var, ver)),
            )
        };

        // A definition's folded type is its own evaluation's: a widened
        // or a joined definition states none. One whose outcome left its
        // place untouched names the version the place held.
        let mut preserved = None;
        let (val, folded) =
            if n17_source_only_definition(ssa.source_variable_symbol(var).is_some(), (var, ver))
                || is_externally_mutable(ssa.var_name(var), escaping, has_dynamic_variable_trace)
                || ssa
                    .cell_key(var)
                    .is_some_and(|key| driver.is_native_error_storage(key))
                || element_write_base == Some(var)
            {
                (LatticeValue::Overdefined, None)
            } else if keeps_all {
                // The statement raised before it stored anything.
                let prior = prior_version(ssa_block, index, var);
                preserved = Some(prior);
                (held_value(values, (var, prior)), None)
            } else if stmt_ssa.may_defs.contains(&var) {
                // A synthetic array-element may-def: the write may or may
                // not have hit this element, so its value is the JOIN of
                // the prior version (recorded as a use) and the written
                // value — unless the statement's evaluated outcome names
                // the element's place (`array set arr {k v}` writes
                // `arr(k)`), which makes the write definite. The base
                // refresh of an element write carries no prior use — the
                // base holds no value of its own.
                match stmt_ssa.uses.get(&var) {
                    Some(prev_ver) => {
                        let (written, folded, stated, _, _) = value_of(values);
                        if stated {
                            (written, folded)
                        } else {
                            let prev = values
                                .get(&(var, *prev_ver))
                                .cloned()
                                .unwrap_or(LatticeValue::Overdefined);
                            (join(&prev, &written), None)
                        }
                    }
                    None => (LatticeValue::Overdefined, None),
                }
            } else {
                let (value, folded, _, kept, _) = value_of(values);
                if kept {
                    let prior = prior_version(ssa_block, index, var);
                    preserved = Some(prior);
                    // A statement that raised before it reached a place
                    // left what the place held, whether or not it read it.
                    if raised.is_some() {
                        (held_value(values, (var, prior)), folded)
                    } else {
                        (value, folded)
                    }
                } else {
                    (value, folded)
                }
            };
        driver.record_folded((var, ver), folded);
        driver.record_preserved((var, ver), preserved);
        if set_value(values, (var, ver), &val) {
            changed = true;
        }
        if let Some(at) = existence.as_deref_mut() {
            let step = at.step_for(
                stmt_ssa,
                (var, ssa.var_name(var), element_write_base),
                driver,
                (keeps_all, || value_of(values).4),
            );
            changed |= at.advance((var, ver), step, driver);
        }
    }
    changed
}

fn n17_source_only_definition(value: bool, key: ValueKey) -> bool {
    #[cfg(debug_assertions)]
    if value
        && tcl_dialect::trace_env::var("TCL_LSP_TRACE_N17_FOLD").as_deref() == Ok("first-gate-v1")
    {
        eprintln!("N17_NATIVE decline=sccp.source-only-definition key={key:?}");
    }
    #[cfg(not(debug_assertions))]
    let _ = key;
    value
}

/// The definitions of the statement at `index` of a block that never runs
/// it, because an earlier statement certainly raised: each keeps what its
/// place held, and its existence is left as it was. Whether a value moved.
fn keep_prior(
    values: &mut HashMap<ValueKey, LatticeValue>,
    (ssa_block, index): (&crate::ssa::SsaBlock, usize),
    stmt_ssa: &SsaStatement,
    (driver, mut existence): (&LatticeDriver<'_>, Option<&mut ExistenceAt<'_>>),
) -> bool {
    let mut changed = false;
    for (&var, &ver) in &stmt_ssa.defs {
        let prior = prior_version(ssa_block, index, var);
        let held = held_value(values, (var, prior));
        driver.record_folded((var, ver), None);
        driver.record_preserved((var, ver), Some(prior));
        changed |= set_value(values, (var, ver), &held);
        if let Some(at) = existence.as_deref_mut() {
            changed |= at.advance(
                (var, ver),
                crate::value_transfer::ExistenceStep::PRESERVE,
                driver,
            );
        }
    }
    changed
}

pub(in super::super) fn typed_element_write_base(
    statement: &Statement,
    ssa: &SsaFunction,
) -> Option<Symbol> {
    if !matches!(
        statement,
        Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. }
    ) {
        return None;
    }
    let names = crate::ir_helpers::structural_variable_write_projection(statement);
    let [name] = names.literal_names.as_slice() else {
        return None;
    };
    let (root, _) = crate::naming::split_element_ref(name)?;
    ssa.var_symbol(root)
}

/// The evaluation the statement at `index` of `block` starts with when it is
/// one of a synthetic embedded call and its host, which are one evaluation: the
/// call's definitions are what the host's words write, and the host's are its
/// result. At the call this evaluates the pair and holds the host's answer in
/// `prepared`; at the host it hands that answer over. `None` for any other
/// statement and wherever the pair declines.
fn pair_answer(
    prepared: &mut Option<(usize, DefValues)>,
    (block, index): (&crate::ssa::SsaBlock, usize),
    values: &HashMap<ValueKey, LatticeValue>,
    (ssa, driver): (&SsaFunction, &LatticeDriver<'_>),
) -> Option<DefValues> {
    let host = prepared.take().filter(|(at, _)| *at == index);
    if let Some((_, answer)) = host {
        return Some(answer);
    }
    let (host, pair) = driver.evaluate_embedded(block, index, values, ssa)?;
    *prepared = Some((host, pair.host));
    Some(match pair.raised {
        Some(written) => DefValues::Raised(Box::new(RaisedDefs {
            prefix: Some(pair.call),
            written,
        })),
        None => DefValues::PerDef(pair.call),
    })
}

/// The version of `var` the statement at `index` of `block` finds in its
/// place: the block's latest earlier definition, else the version the block
/// enters with, else the undefined root.
pub(crate) fn prior_version(
    block: &crate::ssa::SsaBlock,
    index: usize,
    var: Symbol,
) -> crate::ssa::Version {
    block.statements[..index]
        .iter()
        .rev()
        .find_map(|statement| statement.defs.get(&var).copied())
        .or_else(|| block.entry_versions.get(&var).copied())
        .unwrap_or(0)
}
