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

//! W210 on a `return $v` read whose reaching version may be undefined.

use super::super::helpers::{
    PhiUndefMemo, block_dominated_by, build_phi_undef_index, phi_can_undef,
};
use super::{ReturnUndefCtx, reportable, undefined_var_suggestion};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::compilation_unit::ExistencePoint;
use rustc_hash::FxHashSet;
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_registry::value_transfer::FactView;

/// The read-only name/guard/suppression context for the `return`-value
/// phi-from-undef W210 pass ([`Analyser::emit_return_phi_undef_w210`]):
/// the proc parameters, dominating existence guards, scope aliases, the
/// caller-supplied known-defined / defined-var sets, the executable block
/// set, and the undef-suppression model.  Bundled to keep the emitter under
/// the argument limit.
/// The phi-from-undef indices ([`build_phi_undef_index`] output) borrowed for
/// one W210 pass, so the per-read check can be split into its own helper.
struct PhiUndefIndex<'a> {
    phi_def: &'a super::super::helpers::PhiDefMap,
    phi_block: &'a super::super::helpers::PhiBlockMap,
    pub(super) killed: &'a FxHashSet<crate::def_use::StorageValueKey>,
    may_defs: &'a super::super::helpers::MayDefMap,
}

fn return_undefined_variable_message(name: &str, defined_vars: &HashSet<String>) -> String {
    use std::fmt::Write as _;
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
        eprintln!("RBS_READ_OWNER stage=return_phi_message name={name}");
    }
    let mut message = format!("Variable '{name}' is read before it is set");
    if let Some(similar) = undefined_var_suggestion(name, defined_vars) {
        let _ = write!(message, "; did you mean '{similar}'?");
    }
    message
}

impl Analyser {
    /// W210 on `return $v` reads where `v`'s reaching version can be
    /// undefined on some executable path (phi-from-undef / `unset`-killed).
    /// Companion to [`Self::emit_read_before_set_diagnostics`]; see its
    /// trailing call site for why the def-use-chain pass cannot catch
    /// these (return values are terminator reads, not recorded uses).
    pub(in super::super) fn emit_return_phi_undef_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReturnUndefCtx<'_>,
    ) {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};

        // `defined_vars` / `considered` are used directly here; the remaining
        // sets are consulted by `return_read_fires_w210` through `ctx`.
        let defined_vars = ctx.defined_vars;
        let considered = ctx.considered;

        let Some(registry) = self.registry.as_deref() else {
            return;
        };

        let super::super::helpers::UndefIndexMaps {
            phi_def,
            phi_block,
            killed,
            may_defs,
        } = build_phi_undef_index(fu, considered, registry, &ctx.supp.call_steps);
        let phi_idx = PhiUndefIndex {
            phi_def: &phi_def,
            phi_block: &phi_block,
            killed: &killed,
            may_defs: &may_defs,
        };
        // Every `return_read_fires_w210` call below traces the same phi graph
        // with the same context, so they share one memo (issue #2021).
        let mut memo = PhiUndefMemo::default();

        let mut scanner = VarReferenceScanner::with_config(
            VarScanOptions {
                include_var_read_roles: false,
                recurse_cmd_substitutions: true,
                include_reads_before_write: false,
                element_qualified: false,
            },
            self.lexer_config(),
        );

        let mut reported: FxHashSet<String> = ctx.already_reported.iter().cloned().collect();
        // Deterministic block order for stable diagnostics (by BlockId =
        // creation order; the analyser re-sorts diagnostics by span/code).
        let mut block_ids: Vec<crate::cfg::BlockId> = considered.iter().copied().collect();
        block_ids.sort_unstable();

        for bn in block_ids {
            let Some(cfg_block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Return {
                value,
                expr,
                braced,
                ..
            }) = &cfg_block.terminator
            else {
                continue;
            };
            let Some(span) = cfg_block
                .terminator
                .as_ref()
                .and_then(crate::cfg::Terminator::span)
                .map(|s| fu.abs_span(s))
            else {
                continue;
            };
            if span.is_empty() {
                continue;
            }
            let Some(ssa_block) = fu.ssa.blocks.get(&bn) else {
                continue;
            };

            // Collect the variable names read by the return value (word
            // substitutions + nested `[...]`) and any parsed expr. The
            // return value is a single already-extracted word, not a
            // script, so it scans in value-body mode.
            // A **braced** value is literal — `return {$y}` returns the two
            // characters `$y` and reads nothing — so it contributes no reads
            // at all (`UseClass::Quoted`).
            let mut reads: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            if let Some(v) = value.as_ref().filter(|_| !*braced) {
                reads.extend(scanner.scan_word(v, registry));
            }
            if let Some(e) = expr {
                reads.extend(crate::var_refs::vars_in_expr(e, self.grammar()));
            }

            for name in reads {
                if reported.contains(&name) {
                    continue;
                }
                let ver = fu
                    .ssa
                    .var_symbol_at_terminator(bn, &name)
                    .and_then(|s| ssa_block.exit_versions.get(&s))
                    .copied()
                    .unwrap_or(0);
                if !Self::return_read_fires_w210(fu, &name, ver, bn, &phi_idx, ctx, &mut memo) {
                    continue;
                }
                // The existence rung has the last word here too.
                let fact = fu
                    .ssa
                    .var_symbol(&name)
                    .map_or(FactView::Pending, |symbol| {
                        fu.existence(symbol, ExistencePoint::Exit(bn))
                    });
                if reportable(&fact).is_none() {
                    continue;
                }
                reported.insert(name.clone());
                let message = return_undefined_variable_message(&name, defined_vars);
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W210,
                        span,
                        message,
                        Severity::Warning,
                    ));
            }
        }
    }

    /// Decide whether a single `return`-value read of `(name, ver)` in block
    /// `bn` is a W210 phi-from-undef read: its reaching version must be able to
    /// reach an undef origin, and it must not be a parameter / scope alias /
    /// known-defined / qualified / suppressed name or lie in the region an
    /// existence guard proves it bound ([`crate::sccp::SccpResult::guarded`]).
    /// Version-0 reads are handled by the def-use `DefKind::Parameter`
    /// emitter, so they never fire here.
    fn return_read_fires_w210(
        fu: &crate::compilation_unit::FunctionUnit,
        name: &str,
        ver: crate::ssa::Version,
        bn: crate::cfg::BlockId,
        phi_idx: &PhiUndefIndex<'_>,
        ctx: &ReturnUndefCtx<'_>,
        memo: &mut PhiUndefMemo,
    ) -> bool {
        // Version-0 return reads are recorded in def_use, so the version-0
        // (`DefKind::Parameter`) emitter handles them with the full suppression
        // set — this pass only covers the phi-from-undef / `unset`-killed
        // (version > 0) cases, which def-use can't express.  Skipping ver 0
        // avoids double-firing.
        let Some(symbol) = fu.ssa.var_symbol_at_terminator(bn, name) else {
            return false;
        };
        let binding_version = fu.ssa.binding_version(symbol, ver);
        if binding_version == 0 {
            return false;
        }
        let undef_ctx = super::super::helpers::PhiUndefCtx {
            registry: &ctx.registry,
            phi_def: phi_idx.phi_def,
            phi_block: phi_idx.phi_block,
            killed: phi_idx.killed,
            may_defs: phi_idx.may_defs,
            considered: ctx.considered,
            executable_edges: fu.diagnostic_value_facts().executable_edges(),
            exists_guards: ctx.exists_guards,
            initial_global: ctx.initial_global,
            global_aliases: ctx.global_aliases,
            dialect: ctx.dialect,
            ssa: &fu.ssa,
            preserved: &fu.sccp.preserved,
        };
        let Some(cell_name) = fu.ssa.cell_key(symbol) else {
            return false;
        };
        if !phi_can_undef(cell_name, ver, &undef_ctx, memo) {
            return false;
        }
        // A killed SSA version is concrete same-unit evidence that overrides
        // the conservative external-scope assumptions for `global`/`upvar`
        // aliases and qualified names.
        let known_killed = phi_idx.killed.contains(&(cell_name.to_owned(), ver));
        if ctx.params.contains(name)
            || (ctx.scope_aliases.contains(name) && !known_killed)
            || super::super::helpers::read_has_cell_fact(
                fu,
                (bn, usize::MAX),
                cell_name,
                &ctx.cell_facts.known_defined,
                &ctx.registry,
            )
            || ctx.extra_known_defined.contains(name)
            || (cell_name.namespace_membership_for_advice().is_some() && !known_killed)
            || ctx.supp.suppresses_read(name, &(cell_name.clone(), ver))
            || ctx
                .supp
                .written_by_substitution_before(name, &fu.ssa, bn, -1)
        {
            return false;
        }
        // An after-loop `return` of a variable the loop body defines on every
        // iteration is not read-before-set (we assume a may-run loop runs,
        // matching C Tcl); the return block sits outside the loop body.
        if ctx
            .supp
            .after_loop_defined(&(cell_name.to_owned(), ver), fu.cfg.block_name(bn))
        {
            return false;
        }
        // A dominating existence guard proves the var exists here.
        let original_cells = super::super::helpers::original_read_cells_at(
            &fu.ssa,
            (bn, usize::MAX),
            cell_name,
            &ctx.registry,
        );
        known_killed
            || !ctx.exists_guards.iter().any(|(guard, block)| {
                (original_cells.contains(guard)
                    || fu.ssa.point_contexts.is_none() && guard == cell_name)
                    && block_dominated_by(&fu.ssa, bn, *block)
            })
    }
}
