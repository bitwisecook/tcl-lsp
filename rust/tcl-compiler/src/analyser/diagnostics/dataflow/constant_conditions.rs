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

//! I230 and I231 constant branches and switch arms, and the loops whose termination the solver decides.

use crate::analyser::bounds_checks::HeaderFact;
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use std::collections::HashSet;
use tcl_core_types::DiagCode;

/// The solver's decision at the loop header whose condition word spans
/// `span` in `fu`, from its `Applied` branch fact: `None` where the unit did
/// not decide it (the header is not reached, or its condition varies).
fn header_fact(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
) -> Option<HeaderFact> {
    let (_, node) = held_loop(fu, span)?;
    let crate::cfg::Terminator::Goto { target, .. } =
        fu.cfg.blocks.get(&node.start)?.terminator.as_ref()?
    else {
        return None;
    };
    // A rotated for retains its source condition on the latch. Its first
    // test is the synthetic header reached after the initialisation script.
    let header = fu.cfg.block_name(*target);
    let branch = fu
        .diagnostic_value_facts()
        .constant_branches()
        .iter()
        .find(|branch| {
            branch.kind == crate::sccp::BranchFactKind::Applied && branch.block == header
        })?;
    if !branch.value {
        return Some(HeaderFact::Never);
    }
    Some(HeaderFact::Always {
        exits: loop_exit_is_executable(fu, &branch.taken_target, &branch.not_taken_target),
    })
}

/// The loop `fu` holds whose condition word spans `span`, with the block it
/// leaves to.
fn held_loop(
    fu: &crate::compilation_unit::FunctionUnit,
    span: tcl_lexer::Span,
) -> Option<(crate::cfg::BlockId, &crate::cfg::LoopNode)> {
    use crate::ir::Statement;
    fu.cfg
        .loop_nodes
        .iter()
        .find(|(_, node)| match &node.statement {
            Statement::For { condition_span, .. } | Statement::While { condition_span, .. } => {
                fu.abs_span(*condition_span) == span
            }
            _ => false,
        })
        .map(|(&end, node)| (end, node))
}

/// The blocks a loop's passes may run: every block reachable from the block
/// its passes start from ([`crate::cfg::LoopNode::start`]) without passing
/// the block it leaves to, over the exception edges too — so a handler a pass
/// may reach, and what follows it, is counted, which only adds to what may
/// write.
fn loop_blocks(
    fu: &crate::compilation_unit::FunctionUnit,
    start: crate::cfg::BlockId,
    end: crate::cfg::BlockId,
) -> HashSet<crate::cfg::BlockId> {
    let mut seen = HashSet::new();
    let mut pending = fu.cfg.block_successors(start);
    while let Some(block) = pending.pop() {
        if block != end && seen.insert(block) {
            pending.extend(fu.cfg.block_successors(block));
        }
    }
    seen
}

/// The SSA symbol of a loop counter. A frame cell whose canonical key is not
/// its bare spelling is found through the reads the loop itself proves, its
/// header condition first.
fn loop_symbol(
    fu: &crate::compilation_unit::FunctionUnit,
    blocks: &HashSet<crate::cfg::BlockId>,
    var: &str,
) -> Option<crate::ssa::Symbol> {
    fu.ssa.var_symbol(var).or_else(|| {
        let mut ordered: Vec<_> = blocks.iter().copied().collect();
        ordered.sort();
        ordered.into_iter().find_map(|block| {
            fu.ssa
                .source_symbols_at(block, usize::MAX)?
                .get(var)
                .copied()
        })
    })
}

/// What may write `var` over a loop's passes, its `blocks`
/// ([`crate::analyser::bounds_checks::LoopWrites`]): each statement of the
/// blocks whose SSA definitions name it — a cell update, a write a
/// substitution makes, a binder, a destructuring target, an opaque
/// statement's or an arm marker's may-definition, a procedure's write the
/// call states — and, as unseen, a call to code the module cannot see in the
/// blocks ([`crate::ssa::is_unseen_call_marker`]), a write to a computed
/// name in the function ([`crate::dynamic_names::DynamicNameBarrier`]), and
/// a callback script or variable trace of the module that names it.
fn loop_writes(
    fu: &crate::compilation_unit::FunctionUnit,
    blocks: &HashSet<crate::cfg::BlockId>,
    var: &str,
    module: &crate::analyser::bounds_checks::ModuleUnseenWrites,
) -> crate::analyser::bounds_checks::LoopWrites {
    let symbol = loop_symbol(fu, blocks, var);
    let statements = blocks
        .iter()
        .filter_map(|block| fu.ssa.blocks.get(block))
        .flat_map(|block| &block.statements);
    let mut definitions = 0;
    let mut unseen = fu.dynamic_names.writes || module.may_write(var);
    for statement in statements {
        if symbol.is_some_and(|symbol| statement.defs.contains_key(&symbol)) {
            definitions += 1;
        }
        unseen |= crate::ssa::is_unseen_call_marker(&statement.statement);
    }
    crate::analyser::bounds_checks::LoopWrites {
        definitions,
        unseen,
    }
}

/// The integer `var` holds where `node`'s passes start — the end of a
/// `for`'s start script, the block before any other loop
/// ([`crate::cfg::LoopNode::start`]) — when the unit's solver proves it
/// there, on a path that reaches the loop.
fn loop_start_integer(
    fu: &crate::compilation_unit::FunctionUnit,
    node: &crate::cfg::LoopNode,
    blocks: &HashSet<crate::cfg::BlockId>,
    var: &str,
) -> Option<i64> {
    use crate::analyses::{ConstValue, LatticeValue};
    if !fu.sccp.executable_blocks.contains(&node.start) {
        return None;
    }
    let symbol = loop_symbol(fu, blocks, var)?;
    let version = fu
        .ssa
        .blocks
        .get(&node.start)?
        .exit_versions
        .get(&symbol)
        .copied()
        .unwrap_or(0);
    match fu.sccp.value_at(node.start, (symbol, version))? {
        LatticeValue::Const(ConstValue::Int(start)) => Some(*start),
        _ => None,
    }
}

/// Whether an executable path leaves the loop whose body starts at block
/// `body` and whose exit is block `end`: a `break` reaches `end` (the
/// header's own false edge is never taken, the branch being decided), or a
/// block of the loop ends the procedure — a `return`, `error`, `exit` or
/// `throw`. A block the graph cannot name is taken to exit.
fn loop_exit_is_executable(
    fu: &crate::compilation_unit::FunctionUnit,
    body: &str,
    end: &str,
) -> bool {
    let (Some(start), Some(end)) = (fu.cfg.block_id(body), fu.cfg.block_id(end)) else {
        return true;
    };
    if fu.sccp.executable_blocks.contains(&end) {
        return true;
    }
    let mut successors: std::collections::HashMap<crate::cfg::BlockId, Vec<crate::cfg::BlockId>> =
        std::collections::HashMap::new();
    for &(from, to) in &fu.sccp.executable_edges {
        successors.entry(from).or_default().push(to);
    }
    let mut seen = HashSet::from([start]);
    let mut pending = vec![start];
    while let Some(block) = pending.pop() {
        let ends_the_procedure = fu.cfg.blocks.get(&block).is_some_and(|block| {
            matches!(
                block.terminator,
                Some(crate::cfg::Terminator::Return { .. })
            )
        });
        if ends_the_procedure {
            return true;
        }
        for &next in successors.get(&block).into_iter().flatten() {
            if seen.insert(next) {
                pending.push(next);
            }
        }
    }
    false
}

/// The command of the case-list statement that holds the `Selected` fact
/// `arm`, as the source spells it, less a leading `::`, and capitalised to
/// open a message: `Switch`, or `Case` for the statement a `case` spells.
fn case_list_command(
    fu: &crate::compilation_unit::FunctionUnit,
    arm: &crate::sccp::ConstantBranch,
) -> String {
    let command = arm
        .span
        .zip(fu.cfg.block_by_name(&arm.block))
        .and_then(|(pattern, block)| {
            block
                .statements
                .iter()
                .find_map(|statement| match statement {
                    crate::ir::Statement::Switch { span, command, .. }
                        if span.start() <= pattern.start() && pattern.end() <= span.end() =>
                    {
                        Some(command.as_str())
                    }
                    _ => None,
                })
        })
        .unwrap_or("switch");
    let mut letters = tcl_syntax::naming::without_leading_colons(command).chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

impl Analyser {
    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection, by the branch's own target blocks:
    /// - a target named ``switch_…`` → I231 (switch-arm).
    /// - a target named ``if_…`` → I230 (constant if).
    /// - Otherwise → I230 with the generic
    ///   ``"Branch condition '...' is constant"`` message.
    ///
    /// A loop's own test decided true — the idiomatic `while 1` — is not
    /// reported; a loop's test is the branch whose false edge enters the
    /// block the loop leaves to.
    ///
    /// Severity is mapped to ``Hint`` because the
    /// [`Severity`] enum has no ``Info`` variant — ``Hint`` is
    /// the closest non-actionable level.
    pub(in super::super) fn emit_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        // The advisory facts, then the execution solver's: what it decides
        // for erasure (an existence query on the rung) is decided for advice
        // too. Each branch is judged by its own solve's reach.
        let advisory = fu.diagnostic_value_facts();
        let mut reported = HashSet::new();
        for (branch, executable) in advisory
            .constant_branches()
            .iter()
            .map(|branch| (branch, advisory.executable_blocks()))
            .chain(
                fu.sccp
                    .constant_branches
                    .iter()
                    .map(|branch| (branch, &fu.sccp.executable_blocks)),
            )
            .filter(|(branch, _)| branch.kind == crate::sccp::BranchFactKind::Applied)
        {
            // A branch is dead when the not-taken target is
            // unreachable.  SCCP exposes
            // ``executable_blocks`` (the complement); a block
            // is unreachable iff it's in ``cfg.blocks`` but
            // NOT in ``executable_blocks``.
            if fu
                .cfg
                .block_id(&branch.not_taken_target)
                .is_some_and(|id| executable.contains(&id))
                || !reported.insert(branch.block.as_str())
            {
                continue;
            }
            // Locate the branch's terminator span.
            let Some(block) = fu.cfg.block_by_name(&branch.block) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Branch {
                span: Some(span),
                false_target,
                ..
            }) = &block.terminator
            else {
                continue;
            };
            let span = fu.abs_span(*span);
            // A loop's own test is the branch that leaves the loop when false:
            // its false edge enters the block the loop leaves to
            // (`cfg.loop_nodes`), wherever the test sits. A decided `if` in the
            // loop's body, or right after the loop in the block it leaves to,
            // is no loop test, whatever its block's name.
            let is_loop = fu.cfg.loop_nodes.contains_key(false_target);

            // The lowering names a branch's own targets after the command
            // that built it (`if_then` / `if_next`, `switch_arm_body` /
            // `switch_next` / `switch_default`); the block the branch sits
            // in names whatever encloses it, so an `if` inside a switch arm
            // or at the switch's join is still an `if`.
            let targets = [
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = targets.iter().any(|n| n.starts_with("switch_"));
            let is_if = targets.iter().any(|n| n.starts_with("if_"));
            // Suppress the idiomatic infinite loop `while 1 { … }`:
            // a constant-TRUE loop condition is intentional, not a bug (a
            // constant-FALSE loop still flags its unreachable body).
            if is_loop && branch.value {
                continue;
            }

            let (code, message) = if is_switch {
                let code = DiagCode::I231;
                let msg = if branch.value {
                    format!(
                        "Switch condition '{}' is always true here; \
                         subsequent switch arms are unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Switch arm condition '{}' is always false; \
                         this arm is unreachable",
                        branch.condition,
                    )
                };
                (code, msg)
            } else if is_if {
                let msg = if branch.value {
                    format!(
                        "Condition '{}' is always true; \
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                } else {
                    format!(
                        "Condition '{}' is always false; \
                         the consequent branch is unreachable",
                        branch.condition,
                    )
                };
                (DiagCode::I230, msg)
            } else if is_loop {
                // A loop's own test decided true was skipped above.
                let msg = format!(
                    "Loop condition '{}' is never true; the loop leaves at this test",
                    branch.condition,
                );
                (DiagCode::I230, msg)
            } else {
                let msg = format!(
                    "Branch condition '{}' is constant; one branch is unreachable",
                    branch.condition,
                );
                (DiagCode::I230, msg)
            };

            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    code,
                    span,
                    message,
                    // I230/I231 are observational (LSP `Information`).
                    Severity::Info,
                ));
        }
    }

    /// **W230 / W231 / W232 (dynamic).** Interval-driven out-of-range index
    /// detection for a `$var` index whose [`crate::intervals`] range — guard-
    /// narrowed at the use site — proves the access is wholly out of range
    /// against a statically-established container length.  Complements the
    /// syntactic bounds checks (literal index + literal container only); the
    /// two never double-fire because the syntactic checks back off on any
    /// `$var` index, and the proven-word re-run leaves a site reported here
    /// to this check ([`Self::interval_index_sites`]).  Restricted to
    /// SCCP-reachable blocks so a dynamic index in dead code does not warn.
    /// Resolve each loop the walk examined that `fu` decides against the
    /// unit's branch fact at its condition span
    /// ([`crate::analyser::bounds_checks::LoopTerminationCandidate::resolve`]).
    /// A header false at entry is W240, one true at every test with no
    /// executable exit is W241, and either suppresses W242. A loop the unit
    /// holds and does not decide is read against what the unit knows of it
    /// ([`crate::analyser::bounds_checks::LoopTerminationCandidate::settle`]):
    /// what may write a variable over its passes ([`loop_writes`]) and the
    /// integer its counter starts at ([`loop_start_integer`]). It stays
    /// queued, as one the unit does not hold does, for another unit or for
    /// [`Self::flush_loop_terminations`].
    pub(in super::super) fn resolve_loop_terminations(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        for mut candidate in std::mem::take(&mut self.loop_candidates) {
            if let Some(header) = header_fact(fu, candidate.condition_span) {
                self.result
                    .diagnostics
                    .extend(candidate.resolve(Some(header)));
                continue;
            }
            if let Some((end, node)) = held_loop(fu, candidate.condition_span) {
                let blocks = loop_blocks(fu, node.start, end);
                let module = &self.loop_unseen_writes;
                candidate.settle(
                    |var| loop_writes(fu, &blocks, var, module),
                    |var| loop_start_integer(fu, node, &blocks, var),
                );
            }
            self.loop_candidates.push(candidate);
        }
    }

    /// Report every loop no unit decided as its text says: a constant-false
    /// literal, a constant-true one whose body never leaves the loop, a
    /// `for` counter that never terminates, or a counter nothing modifies.
    pub(in crate::analyser) fn flush_loop_terminations(&mut self) {
        for candidate in std::mem::take(&mut self.loop_candidates) {
            self.result.diagnostics.extend(candidate.resolve(None));
        }
    }

    /// I231 for each arm of an opaque `switch` the solver's selection never
    /// runs the body of — the `Selected` branch facts. The arms have no
    /// blocks of their own, so nothing is dropped and no reachability is
    /// applied: the statement stays one call, and only the pattern that can
    /// never be the one selected is reported, at its span.
    pub(in super::super) fn emit_selected_arm_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        for arm in fu
            .diagnostic_value_facts()
            .constant_branches()
            .iter()
            .filter(|branch| branch.kind == crate::sccp::BranchFactKind::Selected)
        {
            let Some(span) = arm.span else {
                continue;
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::I231,
                    fu.abs_span(span),
                    format!(
                        "{} arm '{}' is never selected; this arm is unreachable",
                        case_list_command(fu, arm),
                        arm.condition,
                    ),
                    // I230/I231 are observational (LSP `Information`).
                    Severity::Info,
                ));
        }
    }
}
