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

//! Liveness-based dead-store detection — the `analysis.dead_stores` the
//! explorer's `cfgPostSsa` view reports, distinct from the optimiser's O109
//! dead-store finder ([`crate::optimiser::find_dead_stores`]).
//!
//! A *dead store* is an assignment to a plain proc-local scalar/array whose
//! SSA value is never read (no use in any statement, branch condition, return
//! value, or executable phi edge — exactly the def-use chain's "no uses"
//! verdict). The following suppressions apply:
//!
//! * **reportable-local place** — only a proc-local scalar/array is reportable;
//!   globals/namespace vars, `upvar` aliases, instance vars, traced, and
//!   dynamic (runtime-named) places are observable outside the frame and never
//!   flagged (subsumes the `::`-prefix +
//!   scope-alias guards the W220 diagnostic uses).
//! * **substitution-hidden reads** — a name read inside a `[…]` command
//!   substitution / `expr` / branch condition the version-precise SSA misses.
//! * **element-observed** — an array-element write a read may alias
//!   (via the place-overlap relation).
//! * **SCCP-unreachable blocks** — skipped (reported as O107 instead).
//! * **IR-statement type** — only `AssignConst`, `AssignValue` without a `[…]`
//!   substitution, and `AssignExpr` without a command call are reportable;
//!   side-effecting writes are not (dropping them would drop the side effect).
//!
//! The result is gated by the `tcl diff --show cfg` golden tests.

use tcl_registry::CommandRegistry;

use crate::analyser::state::Analyser;
use crate::analyses::DeadStore;
use crate::compilation_unit::FunctionUnit;
use crate::def_use::DefKind;
use crate::ir::Statement;
use crate::ir_helpers::expr_has_command;
use crate::place::{LOCAL_NS, Place, PlaceKind};
use crate::place_bridge::{build_resolve_context, def_places, element_writes_observed_by_reads};

/// A def *place* a dead-store check may report: a plain proc-local scalar or
/// array — not a global/namespace var (`ns != LOCAL_NS`), `upvar` alias or
/// instance var (wrong `kind`/`owner`), traced (`observed`), or dynamic
/// (runtime-named) target.
fn is_reportable_local(place: &Place) -> bool {
    matches!(
        place.kind,
        PlaceKind::Scalar | PlaceKind::ArrayElem | PlaceKind::ArrayWhole
    ) && place.ns == LOCAL_NS
        && !place.observed
        && !place.dynamic
        && place.owner.is_empty()
}

/// Liveness-based dead stores for one function. See the module docs.
#[must_use]
pub fn liveness_dead_stores(fu: &FunctionUnit, registry: &CommandRegistry) -> Vec<DeadStore> {
    let ctx = build_resolve_context(&fu.cfg, &fu.name);
    let hidden_reads = Analyser::substitution_hidden_reads_of(fu, registry);
    // Array-element / dict writes the name-level SSA mis-folds but that a read
    // observes — keyed by (block, statement_index).
    let element_observed = element_writes_observed_by_reads(&fu.cfg, &fu.name, registry);

    let mut dead: Vec<DeadStore> = Vec::new();
    for chain in fu.def_use.chains.values() {
        if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
            continue;
        }
        let (var, version) = &chain.key;
        // A name read in a position the version-precise `used` set can't see
        // keeps every write of it alive.
        if hidden_reads.contains(var) {
            continue;
        }
        // A synthetic may-def (base refresh / element fan) is not a write
        // the user made — the element's own chain reports.
        if fu.ssa.is_synthetic_def(
            &chain.definition.block,
            chain.definition.statement_index,
            var,
        ) {
            continue;
        }
        // Definitions in SCCP-unreachable blocks are reported as O107.
        if !fu
            .cfg
            .block_id(&chain.definition.block)
            .is_some_and(|id| fu.sccp.executable_blocks.contains(&id))
        {
            continue;
        }
        let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
            continue;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            continue;
        };
        let Some(stmt) = block.statements.get(idx) else {
            continue;
        };
        // IR-statement type filter: only pure
        // assignments are reportable.
        match stmt {
            Statement::AssignConst { .. } => {}
            Statement::AssignValue { value, .. } if !value.contains('[') => {}
            Statement::AssignExpr { expr, .. } if !expr_has_command(expr) => {}
            _ => continue,
        }
        // Reportable-local place (subsumes the `::`-prefix / scope-alias /
        // instance / upvar / dynamic / traced guards).
        let places = def_places(stmt, &ctx, registry);
        if places.len() != 1 || !is_reportable_local(&places[0]) {
            continue;
        }
        // An array-element write a read may alias is not dead.
        if element_observed.contains(&(
            chain.definition.block.clone(),
            chain.definition.statement_index,
        )) {
            continue;
        }
        dead.push(DeadStore {
            block: chain.definition.block.clone(),
            statement_index: idx,
            variable: var.clone(),
            version: *version,
        });
    }
    // Order by block *creation* order (the `prefix_N` counter suffix) then
    // statement index. (`fu.cfg.blocks` is a
    // `HashMap`, but the name suffix recovers the order.)
    dead.sort_by(|a, b| {
        (
            block_creation_order(&a.block),
            a.statement_index,
            &a.variable,
            a.version,
        )
            .cmp(&(
                block_creation_order(&b.block),
                b.statement_index,
                &b.variable,
                b.version,
            ))
    });
    dead
}

/// The block's creation-order counter — the integer suffix of a `prefix_N`
/// block name (`if_then_3` → 3). Falls back to `u32::MAX` for an unexpected
/// shape so such blocks sort last deterministically.
fn block_creation_order(block: &str) -> u32 {
    block
        .rsplit_once('_')
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(u32::MAX)
}
