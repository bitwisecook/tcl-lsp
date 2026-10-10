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

//! Control- and data-flow diagnostics derived from the per-function CFG and
//! SSA form.
//!
//! These checks run from the per-function dispatcher over the compilation
//! unit: dead stores (W220), unused variables (W211) and parameters (W214),
//! read-before-set (W210) including the phi-from-undef and provably-unset
//! variants, unset-on-possibly-undefined (W213), a paste-error fingerprint
//! (H300), constant branches and switch arms (I230/I231), channel-argument
//! validation (W126), divide-by-zero (W233), interval-bounds violations, an
//! invalid IP-address literal at its def site (W124), the racy `static::`
//! cross-event flow (IRULE4005), and the flow-sensitive renamed-command
//! check (W128).

/// Bounded normal stores with literal values are diagnostic candidates.
/// This does not establish that their possible compiler failure can be erased.
fn reportable_dead_assignment(
    statement: &crate::ir::Statement,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> bool {
    match statement {
        crate::ir::Statement::AssignConst { .. } => true,
        crate::ir::Statement::AssignValue { value, .. } => !value.contains('['),
        crate::ir::Statement::AssignExpr { expr, .. } => !crate::ir_helpers::expr_has_command(expr),
        crate::ir::Statement::Call {
            tokens: Some(tokens),
            ..
        } => {
            let Some(binding) = &tokens.source_binding else {
                return false;
            };
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                registry, context, tokens,
            )
            .and_then(|normal| {
                normal
                    .stored_value_word(&binding.variable_context, registry)
                    .map(|word| {
                        matches!(
                            word,
                            crate::ir::WordExpr::Literal { .. }
                                | crate::ir::WordExpr::BracedLiteral { .. }
                        )
                    })
            })
            .unwrap_or(false)
        }
        _ => false,
    }
}

use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;

use rustc_hash::{FxHashMap, FxHashSet};

use super::helpers::{
    PhiUndefMemo, UndefSuppression, block_dominated_by, build_phi_undef_index,
    collect_existence_guards, find_dotted_quads, is_ident_continue, is_word_byte, phi_can_undef,
    source_slice,
};
#[cfg(test)]
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;

use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::analyser::utils::param_name_spans;
use crate::expr_ast::{ExprNode, UnaryOp};

/// Scope facts that make a store observable outside its local SSA chain.
struct DeadStoreVisibility<'a> {
    scope_aliases: &'a HashSet<String>,
    cross_event_vars: &'a HashSet<String>,
    dialect: Option<SurfaceQuery<'a>>,
}

fn dead_store_has_visible_or_synthetic_effects(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    cell_name: &crate::var_resolve::VariableCellKey,
    var: &str,
    registry: &tcl_registry::CommandRegistry,
    visibility: &DeadStoreVisibility<'_>,
) -> bool {
    // Globals (``::``-prefixed) are externally consumed.
    if cell_name.namespace_membership_for_advice().is_some() {
        return true;
    }
    // Interpreter-provided special variables (``auto_path``, ``env``,
    // ``tcl_precision``, …) are read by the runtime / auto-loader even
    // when the script never reads them back, so ``set auto_path …`` is
    // not a dead store.  Dialect-aware: the iRules set differs.
    if super::helpers::special_variable_definition(fu, definition, cell_name, registry).is_some_and(
        |simple| tcl_registry::special_vars::is_externally_read(&simple, visibility.dialect),
    ) {
        return true;
    }
    // A synthetic may-def (base refresh / element fan) is not a
    // write the user made — never a reportable dead store.
    if fu
        .ssa
        .is_synthetic_def(&definition.block, definition.statement_index, cell_name)
    {
        return true;
    }
    // The *direct* base def of a dynamic-key element write
    // (`set a($k) 9` defs base `a` directly): its liveness is
    // carried by the fanned element chains, which exact-name
    // liveness can't see — never report the base.
    if !var.contains('(') && def_is_element_write(fu, definition) {
        return true;
    }
    // Scope-aliased vars (introduced via ``global`` or
    // ``upvar``) write through to a different scope — the
    // local "no use" verdict is unsafe. Policy sets hold *base*
    // names, so an element symbol (`a(k)`) checks its base too.
    let var_base = crate::naming::split_array_name_braced(var, true).0;
    if visibility.scope_aliases.contains(var) || visibility.scope_aliases.contains(var_base) {
        return true;
    }
    // Cross-event vars (iRules ``::when::*`` defs/imports
    // or ``pkgIndex.tcl`` ``$dir``) may be read in
    // another event/scope at runtime.
    if visibility.cross_event_vars.contains(var) || visibility.cross_event_vars.contains(var_base) {
        return true;
    }
    // Suppress dead stores in SCCP-unreachable blocks —
    // O107 already reports the whole block as dead, and
    // re-flagging individual stores inside it adds noise.
    if !fu.cfg.block_id(&definition.block).is_some_and(|id| {
        fu.diagnostic_value_facts()
            .executable_blocks()
            .contains(&id)
    }) {
        return true;
    }
    false
}

/// Find the original span of a conditional dead-assignment diagnostic.
/// Supplied availability metadata grants no executable removal.
fn reportable_dead_assignment_span(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    original_overwrite: bool,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    place_suppressed: &HashSet<(String, i32)>,
) -> Option<tcl_lexer::Span> {
    let block = fu.cfg.block_by_name(&definition.block)?;
    let idx = usize::try_from(definition.statement_index).ok()?;
    let stmt = block.statements.get(idx)?;
    // IR-statement type filter.
    // Only pure assignments are reportable; side-effecting
    // writes (``Call``, ``Incr``, command-substitution
    // values, expressions invoking commands) are skipped
    // because dropping them would also drop the side effect. An exact
    // original overwrite receipt identifies its literal native setter
    // independently of the legacy statement category; its conditional
    // diagnostic still grants no executable removal.
    if !original_overwrite && !reportable_dead_assignment(stmt, registry, context) {
        return None;
    }
    // Suppress when this element write is observed by a read the
    // name-level SSA can't see (place-model overlap).
    if place_suppressed.contains(&(definition.block.clone(), definition.statement_index)) {
        return None;
    }
    let cmd_span = fu.abs_span(stmt.span());
    if cmd_span.is_empty() {
        return None;
    }
    Some(cmd_span)
}

/// Named command outputs and synthetic spans do not identify unused assignments.
fn unused_variable_definition_span(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
) -> Option<tcl_lexer::Span> {
    let block = fu.cfg.block_by_name(&definition.block)?;
    let idx = usize::try_from(definition.statement_index).ok()?;
    let stmt = block.statements.get(idx)?;
    // Only pure assignments are reportable as "set but never used".
    // A variable written by a command (`scan` / `binary scan` /
    // `regexp -> capture`, etc.) or a barrier is a command output the
    // user may legitimately ignore; `Statement::Call` /
    // `Statement::Barrier` defs are skipped.  This is deliberate
    // policy, not a gap: a destructuring writer's surplus output
    // (`binary scan $d H2H* type rest` with
    // `rest` unread) is how Tcl spells "ignore the remainder" — there
    // is no `_` placeholder — so flagging it would punish the idiom.
    if matches!(
        stmt,
        crate::ir::Statement::Call { .. } | crate::ir::Statement::Barrier { .. }
    ) {
        return None;
    }
    // The CFG span is relative to the unit's `base_offset`.
    let cmd_span = fu.abs_span(stmt.span());
    if cmd_span.is_empty() {
        return None;
    }
    Some(cmd_span)
}

/// The read-only name/guard/suppression context for the `return`-value
/// phi-from-undef W210 pass ([`Analyser::emit_return_phi_undef_w210`]):
/// the proc parameters, dominating existence guards, scope aliases, the
/// caller-supplied known-defined / defined-var sets, the executable block
/// set, and the undef-suppression model.  Bundled to keep the emitter under
/// the argument limit.
/// The phi-from-undef indices ([`build_phi_undef_index`] output) borrowed for
/// one W210 pass, so the per-read check can be split into its own helper.
struct PhiUndefIndex<'a> {
    phi_def: &'a super::helpers::PhiDefMap,
    phi_block: &'a super::helpers::PhiBlockMap,
    killed: &'a FxHashSet<crate::def_use::SsaValueKey>,
}

/// The read-only scope and suppression facts for the version-0 / statement
/// W210 pass. Keeping this alongside [`ReturnUndefCtx`] prevents the two W210
/// emitters from acquiring separate, subtly divergent entry-scope contracts.
pub(super) struct ReadBeforeSetCtx<'a> {
    pub initial_global: bool,
    pub global_aliases: &'a HashSet<String>,
    pub defined_vars: &'a HashSet<String>,
    pub scope_aliases: &'a HashSet<String>,
    pub extra_known_defined: &'a HashSet<String>,
    pub cell_facts: &'a super::helpers::DiagnosticCellFacts,
    pub supp: &'a UndefSuppression,
}

/// Existing absence proofs and guards used to classify declared reads.
#[derive(Clone, Copy)]
struct DeclaredReadProofs<'a> {
    exists_guards: &'a [super::helpers::ExistenceGuard],
    proved: &'a std::collections::HashMap<String, tcl_lexer::Span>,
}

pub(super) struct ReturnUndefCtx<'a> {
    pub registry: std::sync::Arc<tcl_registry::CommandRegistry>,
    pub initial_global: bool,
    pub global_aliases: &'a HashSet<String>,
    pub dialect: Option<SurfaceQuery<'a>>,
    pub params: &'a HashSet<&'a str>,
    pub exists_guards: &'a [super::helpers::ExistenceGuard],
    pub scope_aliases: &'a HashSet<String>,
    pub extra_known_defined: &'a HashSet<String>,
    pub cell_facts: &'a super::helpers::DiagnosticCellFacts,
    pub defined_vars: &'a HashSet<String>,
    pub considered: &'a HashSet<crate::cfg::BlockId>,
    pub supp: &'a UndefSuppression,
}

/// Registry-owned startup lifecycle facts for one SSA variable version.
#[derive(Clone, Copy)]
struct StartupReadFacts {
    readable: bool,
    initially_bound: bool,
    lazy_read: bool,
}

fn startup_read_facts(
    cell: &crate::var_resolve::VariableCellKey,
    version: crate::ssa::Version,
    killed: bool,
    initial_global: bool,
    global_aliases: &HashSet<String>,
    dialect: Option<SurfaceQuery<'_>>,
) -> StartupReadFacts {
    let (startup_name, global_binding) =
        super::helpers::startup_cell_binding(cell, initial_global, global_aliases);
    StartupReadFacts {
        readable: global_binding
            && version == 0
            && !killed
            && tcl_registry::special_vars::is_readable_at_startup(startup_name, dialect),
        initially_bound: global_binding
            && version == 0
            && tcl_registry::special_vars::is_initially_bound(startup_name, dialect),
        lazy_read: global_binding
            && tcl_registry::special_vars::is_lazily_readable(startup_name, dialect),
    }
}

/// Facts used while recording the read sites of one undef def-use chain.
struct W210ChainCtx<'a> {
    cell_facts: &'a super::helpers::DiagnosticCellFacts,
    exists_guards: &'a [super::helpers::ExistenceGuard],
    supp: &'a UndefSuppression,
    startup: StartupReadFacts,
}

impl Analyser {
    /// **W128.** Flag a call to a command that was
    /// renamed or deleted earlier in the same file — it falls through to
    /// the `unknown` handler.
    ///
    /// Backed by the flow-sensitive command-binding lattice
    /// ([`crate::command_binding`]).  The lattice is seeded with every
    /// module procedure (canonically qualified) as `Proc` so a proc
    /// defined inside a `namespace eval` block — whose top-level CFG never
    /// sees the full qname — is still known, matching the optimiser's
    /// gating view.  A call fires W128 only when its resolved binding is
    /// `Opaque` *and* its name was actually perturbed somewhere in this
    /// file (`rebound_names`); a merely-undefined external command (always
    /// opaque, never rebound) does not.  A dynamic mutation collapses the
    /// lattice to the wildcard ⊤, under which every binding resolves to
    /// `Unknown` (not `Opaque`), so W128 conservatively goes quiet.
    pub(super) fn emit_w128_renamed_command(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
        registry: &tcl_registry::CommandRegistry,
    ) {
        use crate::command_binding::{Binding, BindingKind, analyse_command_binding};
        use crate::ir::Statement;
        use crate::naming::normalise_qualified_name as nqn;

        let cfg = &cu.top_level.cfg;
        let seed: Vec<(String, Binding)> = cu
            .ir_module
            .procedures
            .keys()
            .map(|q| {
                (
                    q.clone(),
                    Binding {
                        kind: BindingKind::Proc,
                        target: Some(q.clone()),
                    },
                )
            })
            .collect();
        let binding = analyse_command_binding(cfg, registry, &seed);
        let rebound = binding.rebound_names();
        if rebound.is_empty() {
            return;
        }
        // Reverse-postorder for deterministic diagnostic ordering.
        for block_id in cfg.reverse_postorder() {
            let Some(block) = cfg.blocks.get(&block_id) else {
                continue;
            };
            for (idx, stmt) in block.statements.iter().enumerate() {
                let Statement::Call { command, span, .. } = stmt else {
                    continue;
                };
                // The mutation commits after this invocation's successful
                // completion, so the point-wise binding query naturally sees
                // the command's incoming binding here.  No mutator-name
                // exclusion is required.
                if command.is_empty() {
                    continue;
                }
                if binding.binding_at(block_id, idx, command).kind != BindingKind::Opaque {
                    continue;
                }
                if !rebound.contains(&nqn(command)) {
                    continue; // never bound here → an ordinary external command
                }
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::W128,
                        *span,
                        format!(
                            "Command '{command}' was renamed or deleted earlier in this \
file; this call falls through to the 'unknown' handler."
                        ),
                        Severity::Warning,
                    ));
            }
        }
    }

    /// Statements whose dead-store **W220** hint should be **suppressed**
    /// because their array-element / dict-path def place is observed by some
    /// read in the function.
    ///
    /// Name-level SSA folds `a(k)` / `a(j)` / `$a` to the base name `a`, so a
    /// later `set a(j) 2` looks like it overwrites `set a(k) 1` before any read
    /// — a false dead store when `a(k)` is in fact read.  Delegates to the
    /// shared [`crate::place_bridge::element_writes_observed_by_reads`] (also
    /// used by the optimiser's O109), which resolves each element write to a
    /// [`Place`](crate::place::Place) and consults the over-approximating
    /// [`overlap`](crate::place::overlap).  Scalars keep the precise name-level
    /// verdict (they don't fold), so a genuine `set x 1; set x 2; puts $x` dead
    /// store still fires.  Empty when no registry is bound (e.g. the bare
    /// `emit_cfg_ssa_diagnostics` test path).
    fn place_suppressed_dead_stores(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> std::collections::HashSet<(String, i32)> {
        self.registry
            .as_deref()
            .map_or_else(Default::default, |reg| {
                crate::place_bridge::element_writes_observed_by_reads(&fu.cfg, &fu.name, reg)
            })
    }

    /// Variable names read inside positions the version-precise SSA `used`
    /// set can't see — `[…]` command substitutions in command arguments,
    /// `expr` values, and `if`/`while`/`for` branch conditions. A write to
    /// such a name is not a dead store even when its SSA version looks
    /// unused.
    fn substitution_hidden_reads(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) -> FxHashSet<String> {
        self.registry
            .as_deref()
            .as_ref()
            .map_or_else(FxHashSet::default, |reg| {
                Self::substitution_hidden_reads_of(fu, reg)
            })
    }

    /// `self`-free core of [`Self::substitution_hidden_reads`] so the explorer's
    /// liveness dead-store pass (which has no `Analyser`) can reuse it.
    pub(crate) fn substitution_hidden_reads_of(
        fu: &crate::compilation_unit::FunctionUnit,
        registry: &tcl_registry::CommandRegistry,
    ) -> FxHashSet<String> {
        crate::optimiser::elimination::collect_rmw_hidden_reads(fu, registry)
            .into_iter()
            .collect()
    }

    /// W220 — dead-store hint.
    ///
    /// A *dead store* is an
    /// assignment whose value is overwritten before being read —
    /// some other SSA version of the same variable is live, so
    /// this version's value never reaches a user.
    ///
    /// Walks every dead [`Statement`](crate::ir::Statement) chain
    /// in `fu.def_use`, checks that another version of the same
    /// variable has live uses, and emits a Hint at the dead
    /// statement's span.  When the variable's name has a
    /// case-insensitive twin among `defined_vars`, the message
    /// includes a "did you mean…?" suggestion.
    ///
    /// Filters applied:
    ///
    /// 1. **SCCP-unreachable blocks** — definitions in blocks
    ///    SCCP proved unreachable are reported as O107 by the
    ///    optimiser and intentionally suppressed here so we
    ///    don't double-up on dead-code calls.
    /// 2. **Scope aliases** (`global` / `upvar`) — writes are
    ///    visible in another scope; the local "no use" verdict
    ///    is unsafe.
    /// 3. **Cross-event vars** — for `pkgIndex.tcl` `$dir` and
    ///    iRules `::when::*` cross-event defs/imports, a write
    ///    in one event may be read in another at runtime.
    /// 4. **Globals (`::`-prefixed)** — externally consumed.
    /// 5. **Side-effecting stores** — only `AssignConst`,
    ///    `AssignValue` without `[`, and `AssignExpr` without a
    ///    command call are considered.  `Call.defs`, `Incr`, and
    ///    other side-effecting writes shouldn't be flagged
    ///    because removing the assignment would also drop the
    ///    side effect.
    pub(super) fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
        cell_facts: &super::helpers::DiagnosticCellFacts,
    ) {
        use crate::def_use::DefKind;
        use std::fmt::Write as _;

        // A dynamic read (`[set $name]`, `subst $tmpl`) can observe *any*
        // store, so "this assignment is never read" is unprovable anywhere in
        // the function.  Abstain toward silence.
        let hidden_reads = self.substitution_hidden_reads(fu);
        // Array-element / dict-path writes the
        // name-level SSA mis-folds but that a read actually observes.
        let place_suppressed = self.place_suppressed_dead_stores(fu);
        let generation = self.analysis_context();
        let registry = generation.commands();
        let unread_layout = std::cell::OnceCell::new();
        let visibility = DeadStoreVisibility {
            scope_aliases,
            cross_event_vars,
            dialect: Some(generation.context().authoring_query()),
        };
        for chain in fu.def_use.chains.values() {
            if chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (cell_name, _version) = &chain.key;
            let Some(symbol) = fu.ssa.cell_symbol(cell_name) else {
                continue;
            };
            if super::helpers::definition_has_cell_fact(
                fu,
                &chain.definition,
                cell_name,
                &cell_facts.externally_read,
                registry,
            ) {
                continue;
            }
            let var = fu.ssa.var_name(symbol);
            if original_unrepresented_use_advice(
                fu,
                &chain.definition,
                var,
                registry,
                &unread_layout,
            ) {
                continue;
            }
            let overwrite = original_overwrite_advice(fu, &chain.definition, var, registry);
            let conditional_unread = fu.dynamic_names.reads
                && chain.is_dead()
                && !hidden_reads.contains(var)
                && original_unread_store_advice(
                    fu,
                    &chain.definition,
                    var,
                    registry,
                    &unread_layout,
                );
            if !chain.is_dead() && overwrite.is_none() {
                continue;
            }
            // A name read inside a command substitution / expr / branch
            // condition the version-precise `used` set can't see keeps every
            // write of it alive (`set i 0` before `[incr i $j]`). Suppress at
            // name level.
            if (hidden_reads.contains(var) || (fu.dynamic_names.reads && !conditional_unread))
                && overwrite.is_none()
            {
                continue;
            }
            if dead_store_has_visible_or_synthetic_effects(
                fu,
                &chain.definition,
                cell_name,
                var,
                registry,
                &visibility,
            ) {
                continue;
            }
            // A dead assignment is W220 whether or not the variable is also
            // unused overall: the assignment-level dead store (W220) and,
            // when the variable is never read at all, the variable-level
            // unused hint (W211) are distinct diagnostics with distinct
            // fixes (drop this assignment vs. drop the variable).  Fires
            // on any dead store regardless of other live versions.
            let Some(cmd_span) = reportable_dead_assignment_span(
                fu,
                &chain.definition,
                overwrite.is_some(),
                registry,
                Some(generation.as_ref().into()),
                &place_suppressed,
            ) else {
                continue;
            };
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            let mut message = if conditional_unread {
                format!("Assignment to '{var}' may be unused in the declared local frame")
            } else if overwrite == Some(OverwriteDiagnostic::Conditional) {
                format!("Assignment to '{var}' may be overwritten before it is read")
            } else {
                format!("Assignment to '{var}' is never read")
            };
            if let Some(similar) = find_case_mismatch(var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W220,
                    span,
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// Authored setter ordering may supply a conditional warning even when an
    /// earlier native read fails. It supplies no physical definition or edit.
    pub(super) fn emit_conditional_declared_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        scope_aliases: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
    ) {
        let context = self.analysis_context();
        let registry = self.registry.as_deref().unwrap_or(context.commands());
        let image = tcl_lexer::SourceImage::document(&self.source);
        let Some(report) = self
            .head_identities
            .source_bindings()
            .original_procedure_declaration_flow(procedure, &image, registry)
            .or_else(|| function_declaration_flow(fu, procedure, &image, registry))
        else {
            return;
        };
        for advice in crate::registry_invocation::conditional_declared_overwrite_advice(&report) {
            let name = advice.name();
            let base = crate::naming::split_array_name_braced(name, true).0;
            let span = advice.target();
            if !advice.owns_source(&image)
                || scope_aliases.contains(name)
                || scope_aliases.contains(base)
                || cross_event_vars.contains(name)
                || cross_event_vars.contains(base)
                || super::helpers::special_variable_definition_at_span(fu, span, registry)
                    .is_some_and(|simple| {
                        tcl_registry::special_vars::is_externally_read(
                            &simple,
                            Some(context.context().authoring_query()),
                        )
                    })
                || self
                    .result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagCode::W220 && diagnostic.span == span)
            {
                continue;
            }
            self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                DiagCode::W220,
                span,
                format!("Assignment to '{name}' may be overwritten before a local read if execution reaches these declared native setters"),
                Severity::Hint,
            ));
        }
    }

    /// Narrow a whole-command span to its assignment-target token (the
    /// second word, `argv[1]`), returning that token's absolute span — or
    /// `None` when it can't be located, so callers fall back to the command
    /// span.  W211 / W220 anchor at the variable-name column, not the command
    /// start.  Re-lexes the command's own source slice (token-based) and
    /// takes the first non-separator word after the command name.
    fn narrow_to_assigned_name(&self, stmt_span: tcl_lexer::Span) -> Option<tcl_lexer::Span> {
        let base = stmt_span.start();
        let slice = source_slice(&self.source, stmt_span)?;
        let toks = tcl_lexer::Lexer::with_source_map(
            tcl_lexer::SourceMap::new(&slice),
            self.lexer_config(),
        )
        .tokenise_all()
        .ok()?;
        let name = toks
            .iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    tcl_lexer::TokenType::Sep
                        | tcl_lexer::TokenType::Eol
                        | tcl_lexer::TokenType::Comment
                )
            })
            .nth(1)?;
        Some(tcl_lexer::Span::new(
            name.span.start() + base,
            name.span.end() + base,
        ))
    }

    /// Narrow a whole-command span to the `$var` read token for *var*,
    /// returning that token's absolute span — or `None` when no matching
    /// `Var` token is found anywhere in the statement (the caller falls
    /// back to the command span).  W210 anchors at the variable read, not
    /// the command-start column.
    ///
    /// The read is frequently nested inside a command substitution
    /// (`set x [cmd $var]`, `if {[llength $var]} …`), so the scan descends
    /// into `Cmd` tokens' inner scripts rather than stopping at the
    /// top-level word walk.  Braced (`Str`) words are literal text — a
    /// `$var` inside one is not a read — so they are never descended.
    fn narrow_to_read_var(&self, stmt_span: tcl_lexer::Span, var: &str) -> Option<tcl_lexer::Span> {
        // The SSA name is already resolved: a leading dollar is literal.
        // Source sigils and the selected release's lexical extent belong to
        // the shared scanner, separately from the combined-name split owner.
        // naming.diagnostics.original-variable-name-anchor
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
        /// First `$target` `Var` token within `slice` (whose absolute start
        /// is `abs_base`), descending into command-substitution contents.
        /// `depth` bounds pathological nesting.
        fn find_var(
            slice: &str,
            abs_base: u32,
            target: &str,
            config: tcl_lexer::LexerConfig,
            depth: u8,
        ) -> Option<tcl_lexer::Span> {
            if depth > 4 {
                return None;
            }
            let toks = tcl_lexer::Lexer::with_source_map(tcl_lexer::SourceMap::new(slice), config)
                .tokenise_all()
                .ok()?;
            for t in &toks {
                match t.kind {
                    tcl_lexer::TokenType::Var => {
                        let Ok(Some(reference)) = tcl_lexer::scan_var_ref(
                            slice.as_bytes(),
                            t.span.start() as usize,
                            config,
                        ) else {
                            continue;
                        };
                        let name = std::str::from_utf8(reference.name).ok()?;
                        let root =
                            crate::naming::split_element_ref(name).map_or(name, |(root, _)| root);
                        if root == target {
                            return Some(tcl_lexer::Span::new(
                                t.span.start() + abs_base,
                                u32::try_from(reference.next).ok()? + abs_base,
                            ));
                        }
                    }
                    // A `[…]` word: recurse into its inner script (the span
                    // covers `[inner` and excludes the closing `]`; the
                    // content starts past the opening bracket).
                    tcl_lexer::TokenType::Cmd => {
                        let content_start = t.span.start() as usize + usize::from(t.content_offset);
                        let content_end = t.span.end() as usize;
                        if let Some(inner) = slice.get(content_start..content_end)
                            && inner.contains('$')
                            && let Some(found) = find_var(
                                inner,
                                abs_base + t.span.start() + u32::from(t.content_offset),
                                target,
                                config,
                                depth + 1,
                            )
                        {
                            return Some(found);
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        let target = crate::naming::split_element_ref(var).map_or(var, |(root, _)| root);
        let slice = source_slice(&self.source, stmt_span)?;
        find_var(&slice, stmt_span.start(), target, self.lexer_config(), 0)
    }

    /// W211 — unused-variable hint.
    ///
    /// Fires when an
    /// assignment's variable has no live uses **and** no other
    /// SSA version is live (so the variable is entirely unused
    /// — distinct from W220's overwritten-before-read case).
    ///
    /// Three filters apply:
    ///
    /// 1. **Scope aliases** (``global`` / ``upvar``) — writes
    ///    are visible in the aliased scope, so a "no local use"
    ///    verdict is unsafe.
    /// 2. **Textual references** — variable names that appear
    ///    inside a ``"$x"`` string interpolation or a
    ///    ``Return`` value are kept live; the def-use builder
    ///    doesn't track those reads.
    /// 3. **Empty spans** — synthetic IR statements with no
    ///    user-visible source text.
    ///
    /// "Did you mean…?" suggestions use case-insensitive
    /// matching against the function's defined-variable set.
    pub(super) fn emit_unused_variable_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        textually_referenced: &HashSet<String>,
        cell_facts: &super::helpers::DiagnosticCellFacts,
    ) {
        use crate::def_use::DefKind;
        // A dynamic read (`foreach v [info locals] {… [set $v] …}`) reaches
        // every local by a name no literal `$x` token spells, so "set but
        // never used" is unprovable.  Abstain toward silence.
        if fu.dynamic_names.reads {
            return;
        }
        let generation = self.analysis_context();
        let registry = self.registry.as_deref().unwrap_or(generation.commands());
        let declaration_layout = std::cell::OnceCell::new();
        // W211 is a per-variable verdict ("the variable is set but never
        // used"), not per-assignment: a variable set several times and never
        // read fires once, at its earliest definition. Collect the earliest
        // reportable span per variable, then emit a single W211 per unused
        // variable.
        let mut earliest: std::collections::HashMap<String, tcl_lexer::Span> =
            std::collections::HashMap::new();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let (cell_name, version) = &chain.key;
            let Some(symbol) = fu.ssa.cell_symbol(cell_name) else {
                continue;
            };
            if super::helpers::definition_has_cell_fact(
                fu,
                &chain.definition,
                cell_name,
                &cell_facts.externally_read,
                registry,
            ) {
                continue;
            }
            let var = fu.ssa.var_name(symbol);
            // Namespace storage is visible beyond this unit. Local keys with
            // qualified-looking labels do not acquire namespace membership.
            if cell_name.namespace_membership_for_advice().is_some() {
                continue;
            }
            if scope_aliases.contains(var) {
                continue;
            }
            if textually_referenced.contains(var)
                || original_unrepresented_use_advice(
                    fu,
                    &chain.definition,
                    var,
                    &self.profile_registry(),
                    &declaration_layout,
                )
            {
                continue;
            }
            // A synthetic may-def (base refresh / element fan) is not a
            // write the user made — the element's own chain reports.
            if fu.ssa.is_synthetic_def(
                &chain.definition.block,
                chain.definition.statement_index,
                cell_name,
            ) {
                continue;
            }
            // Interpreter-provided special variables (``auto_path``, ``env``,
            // …) are consumed by the runtime even when the script never reads
            // them, so a bare ``set auto_path …`` is not an unused variable.
            // Dialect-aware via the special-variable registry.
            if super::helpers::special_variable_definition(
                fu,
                &chain.definition,
                cell_name,
                registry,
            )
            .is_some_and(|simple| {
                tcl_registry::special_vars::is_externally_read(
                    &simple,
                    Some(self.analysis_context().context().authoring_query()),
                )
            }) {
                continue;
            }
            // Only emit when no other SSA version of this var is
            // live — the W220 path handles overwritten cases.
            let any_other_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == *cell_name && k.1 != *version && !c.is_dead());
            if any_other_live {
                continue;
            }
            let Some(cmd_span) = unused_variable_definition_span(fu, &chain.definition) else {
                continue;
            };
            // Anchor at the variable name (the assignment target), not the
            // command-start column.
            let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
            earliest
                .entry(var.to_owned())
                .and_modify(|s| {
                    if span.start() < s.start() {
                        *s = span;
                    }
                })
                .or_insert(span);
        }
        self.emit_unused_variable_spans(earliest, defined_vars);
    }

    fn emit_unused_variable_spans(
        &mut self,
        earliest: std::collections::HashMap<String, tcl_lexer::Span>,
        defined_vars: &HashSet<String>,
    ) {
        use std::fmt::Write as _;

        let mut entries: Vec<(String, tcl_lexer::Span)> = earliest.into_iter().collect();
        entries.sort_by_key(|(_, span)| span.start());
        // A variable that is set-but-never-used gets a W211 at its assignment's
        // name token. The dead-store pass (W220), which ran first, already
        // anchored a "never read" hint at the *same* token for that single
        // assignment — a redundant double-emit. W211 ("never used at all") is
        // the more informative message, so drop the co-located W220. Keyed on
        // the exact span, so a genuinely distinct dead store of a
        // multiply-assigned variable is untouched.
        let w211_spans: std::collections::HashSet<tcl_lexer::Span> =
            entries.iter().map(|(_, span)| *span).collect();
        self.result
            .diagnostics
            .retain(|d| !(d.code == DiagCode::W220 && w211_spans.contains(&d.span)));
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is set but never used");
            if let Some(similar) = find_case_mismatch(&var, defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W211,
                    span,
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// H300 — possible paste error (duplicate dead-store with
    /// identical literal).
    ///
    /// When two consecutive
    /// statements in the same block are both dead stores AND
    /// share the same paste-fingerprint
    /// (same variable name + same trimmed literal value), emit
    /// a Hint at the *second* statement's span — the duplicate
    /// is the one that's almost certainly a paste error.
    ///
    /// Variables whose names start with ``_`` are excluded from
    /// the heuristic on the assumption that the leading
    /// underscore signals the user has flagged them as
    /// intentional.
    pub(super) fn emit_possible_paste_error_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::def_use::DefKind;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: FxHashMap<&str, FxHashSet<usize>> = FxHashMap::default();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead() || chain.definition.kind != DefKind::Statement {
                continue;
            }
            let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
                continue;
            };
            dead_idx
                .entry(chain.definition.block.as_str())
                .or_default()
                .insert(idx);
        }

        for block in fu.cfg.blocks.values() {
            let Some(dead_indices) = dead_idx.get(block.name.as_str()) else {
                continue;
            };
            // Walk consecutive pairs (idx, idx + 1).  Only the
            // first must be dead — the second's
            // dead-status is irrelevant; what matters is whether
            // the value being assigned matches.
            for idx in 0..block.statements.len().saturating_sub(1) {
                if !dead_indices.contains(&idx) {
                    continue;
                }
                let Some(first) = super::utils::possible_paste_fingerprint(&block.statements[idx])
                else {
                    continue;
                };
                let Some(second) =
                    super::utils::possible_paste_fingerprint(&block.statements[idx + 1])
                else {
                    continue;
                };
                if first != second {
                    continue;
                }
                let (var_name, literal) = first;
                if var_name.starts_with('_') {
                    continue;
                }
                let span = fu.abs_span(block.statements[idx + 1].span());
                if span.is_empty() {
                    continue;
                }
                let pretty = super::utils::format_literal_for_message(&literal);
                let message = format!(
                    "Possible paste error: repeated assignment to '{var_name}' \
                     with static value '{pretty}'; \
                     did you mean to assign a different variable?"
                );
                self.result
                    .diagnostics
                    .push(crate::analyser::types::Diagnostic::new(
                        DiagCode::H300,
                        span,
                        message,
                        Severity::Hint,
                    ));
            }
        }
    }

    /// Absolute source spans of each formal parameter's *name*, in
    /// declaration order and index-aligned with `ir_proc.params`.
    ///
    /// The parameter-list word is the first word after the proc-name token
    /// (recovered from the recorded [`crate::analyser::types::ProcDef::name_span`]);
    /// its name spans are delegated to [`param_name_spans`] — the same helper
    /// go-to-definition/rename use, so W214's range matches them exactly.
    /// Returns an empty vec when the proc isn't in `all_procs` or the word
    /// can't be isolated, so the caller falls back to the whole-def span.
    fn param_name_spans_for(&self, ir_proc: &crate::ir::Procedure) -> Vec<tcl_lexer::Span> {
        let Some(pdef) = self.result.all_procs.get(&ir_proc.qualified_name) else {
            return Vec::new();
        };
        let bytes = self.source.as_bytes();
        // Skip whitespace between the proc name and the parameter-list word.
        let mut i = pdef.name_span.end() as usize;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        let end = if bytes.get(i) == Some(&b'{') {
            // Braced list — advance to the matching close brace.
            let mut level = 0u32;
            let mut j = i;
            while j < bytes.len() {
                match bytes[j] {
                    b'{' => level += 1,
                    b'}' => {
                        level -= 1;
                        if level == 0 {
                            j += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            j
        } else {
            // Bare single-word parameter list — to the next whitespace.
            let mut j = i;
            while j < bytes.len() && !bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            j
        };
        let Some(raw) = self.source.get(start..end) else {
            return Vec::new();
        };
        param_name_spans(raw, u32::try_from(start).unwrap_or(u32::MAX))
    }

    /// W214 — unused-parameter hint.
    ///
    /// For every parameter
    /// declared in `ir_proc.params`, check whether any def-use
    /// chain for the parameter (any SSA version) has live uses.
    /// When all chains are dead, the parameter is unused —
    /// emit a Hint at the parameter's *name* span (falling back to the
    /// proc's span when the name can't be located), so each unused param
    /// gets its own tight squiggle instead of stacking on the whole proc.
    pub(super) fn emit_unused_param_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: &crate::ir::Procedure,
    ) {
        // Empty-body procs (``proc foo {a b} {}``) are signature
        // placeholders — stubs declaring an API whose implementation
        // lives elsewhere.  Every parameter is necessarily "unused"
        // since there is no body to use it, so flagging is pure noise.
        if ir_proc.body.statements.is_empty() {
            return;
        }
        // Per-parameter name spans (index-aligned with `ir_proc.params`); an
        // empty result means we couldn't isolate the list and fall back to the
        // whole-definition span below.
        let param_spans = self.param_name_spans_for(ir_proc);
        let mut unused: Vec<(usize, String)> = Vec::new();
        for (idx, param) in ir_proc.params.iter().enumerate() {
            // Tcl's variadic ``args`` parameter is conventionally
            // declared even when unused (as a "consume the rest"
            // marker).  Skip it from W214.
            if param == "args" {
                continue;
            }
            // Positional keyword markers: a param *written* as a quoted
            // literal (snit-style ``{"as" ""}``) is a syntactic placeholder
            // consumed by being PRESENT in the call form, not read as a
            // variable.  Flagging it is noise.  The marker lives in the source
            // spelling, not the name: Tcl decodes the quotes away (tclsh 9.0
            // — ``proc p {"as" v} {}`` → ``info args p`` is ``as v``), so the
            // test is on the declaration text the name span covers.
            // Conservative: only a spelling that starts AND ends with ``"``.
            if param_spans
                .get(idx)
                .and_then(|span| self.source.get(span.as_range()))
                .is_some_and(|spelling| {
                    spelling.len() >= 2 && spelling.starts_with('"') && spelling.ends_with('"')
                })
            {
                continue;
            }
            let any_live = fu.def_use.chains.iter().any(|(key, chain)| {
                fu.ssa
                    .var_symbol(param)
                    .is_some_and(|symbol| fu.ssa.cell_key(symbol) == &key.0)
                    && !chain.is_dead()
            });
            if any_live {
                continue;
            }
            // Fallback: the def-use builder doesn't track variable
            // references inside ``[expr {...}]`` command
            // substitutions or arbitrary nested ``[cmd ...]``
            // bodies that don't lower into a structured IR.
            // If the body source contains a ``$param`` /
            // ``${param}`` reference anywhere, treat the parameter
            // as used and skip W214.  Saves the W214 over-emit on
            // ``proc f {x} { return [expr {$x + 1}] }``-style bodies.
            if let Some(body_source) = ir_proc.body_source.as_deref()
                && body_references_param(body_source, param, self.lexer_config())
            {
                continue;
            }
            unused.push((idx, param.clone()));
        }
        if unused.is_empty() {
            return;
        }
        // Dispatch-protocol suppression: when ≥3 peer procs in this
        // namespace share this proc's leading-param signature AND an
        // arity-compatible variable-command dispatcher exists, the leading
        // params are an external contract, not genuinely unused.  Computed
        // only when there is something to report.
        let ns = namespace_of(&ir_proc.qualified_name);
        let leading: Vec<String> = ir_proc
            .params
            .iter()
            .take_while(|p| *p != "args")
            .cloned()
            .collect();
        let protocol_params: HashSet<String> = if !leading.is_empty()
            && self
                .dispatch_protocol_signatures()
                .contains(&(ns, leading.clone()))
        {
            leading.into_iter().collect()
        } else {
            HashSet::new()
        };
        for (idx, param) in unused {
            if protocol_params.contains(&param) {
                continue;
            }
            let message = format!(
                "Parameter '{param}' of proc '{name}' is unused",
                name = ir_proc.qualified_name,
            );
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W214,
                    param_spans.get(idx).copied().unwrap_or(ir_proc.span),
                    message,
                    Severity::Hint,
                ));
        }
    }

    /// Identify `(namespace, leading-param-list)` pairs that look like a
    /// **dispatch protocol** — ≥3 peer procs in the same namespace sharing a
    /// leading-param signature dictated by an arity-compatible
    /// variable-command dispatcher.
    fn dispatch_protocol_signatures(&self) -> HashSet<(String, Vec<String>)> {
        // Group user procs by (namespace, leading-param-tuple stopping at `args`).
        let mut groups: FxHashMap<(String, Vec<String>), usize> = FxHashMap::default();
        for (qname, pdef) in &self.result.all_procs {
            let leading: Vec<String> = pdef
                .params
                .iter()
                .take_while(|p| p.name != "args")
                .map(|p| p.name.clone())
                .collect();
            if leading.is_empty() {
                continue;
            }
            *groups.entry((namespace_of(qname), leading)).or_insert(0) += 1;
        }
        let peer_protos: HashSet<(String, Vec<String>)> = groups
            .into_iter()
            .filter(|(_, n)| *n >= 3)
            .map(|(k, _)| k)
            .collect();
        if peer_protos.is_empty() {
            return HashSet::new();
        }
        // Dispatcher evidence: map each dispatcher namespace → the argument
        // counts observed at its variable-command sites.
        let mut dispatcher_ns_argc: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
        for site in &self.var_command_sites {
            let off = site.cmd_span.start();
            let dns = self
                .result
                .all_procs
                .iter()
                .find(|(_, p)| p.body_span.start() <= off && off <= p.body_span.end())
                .map_or_else(|| "::".to_string(), |(q, _)| namespace_of(q));
            dispatcher_ns_argc.entry(dns).or_default().insert(site.argc);
        }
        peer_protos
            .into_iter()
            .filter(|(ns_key, params)| {
                let min_argc = params.len();
                dispatcher_ns_argc.iter().any(|(dns, argcs)| {
                    (dns == ns_key || dns.starts_with(&format!("{ns_key}::")))
                        && argcs.iter().any(|&a| a >= min_argc)
                })
            })
            .collect()
    }

    /// W210 + W213 — read-before-set / unset on possibly-undefined.
    ///
    /// Walks every
    /// version-0 chain (`DefKind::Parameter`) in `fu.def_use`
    /// — those are the synthetic defs the def-use builder
    /// emits when a variable is used without a preceding def.
    ///
    /// Distinguishes real proc parameters from synthetic RBS
    /// reads via `ir_proc.params`.  Only emits inside procedures
    /// (i.e. when `ir_proc` is `Some`) — top-level RBS needs the
    /// `globals_written_by_procs` filter.
    ///
    /// Per use site:
    ///
    /// - **Phi-incoming uses** are skipped — they sit at block
    ///   boundaries and don't anchor on a real statement.
    /// - **`unset` without `-nocomplain`** emits W213 (the more
    ///   specific code) instead of W210.  W213 message tells
    ///   the user to add `-nocomplain` rather than initialise
    ///   the variable.
    /// - **`safe_on_uninit` calls** that initialise the variable
    ///   themselves (it's in their `defs`) are skipped —
    ///   commands like `lappend` / `incr` / `dict set` safely
    ///   initialise an uninitialised variable.
    /// - Everything else emits W210 with the canonical
    ///   "read before set" message + optional "did you mean…?"
    ///   suggestion.
    pub(super) fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        ctx: &ReadBeforeSetCtx<'_>,
    ) {
        let exists_guards =
            collect_existence_guards(fu, self.analysis_context().commands(), self.lexer_config());
        let mut w210_min = self.definite_missing_read_spans(fu, ctx, &exists_guards);
        let original_image = tcl_lexer::SourceImage::document(&self.source);
        let declared_flow = ir_proc.and_then(|procedure| {
            function_declaration_flow(fu, procedure, &original_image, &self.profile_registry())
        });

        // Unknown writers retire physical absence proofs. Declaration advice
        // must establish a particular original read; ownership of the whole
        // body does not close an unknown option grammar or caller frame.
        if fu.dynamic_names.writes {
            let potential = ir_proc.and_then(|procedure| {
                let declaration = crate::script_binds::authored_procedure_read_advice(
                    &original_image,
                    procedure,
                    &self.result,
                    &self.analysis_context(),
                )?;
                self.declaration_potential_read_spans(
                    fu,
                    procedure,
                    &declaration,
                    declared_flow.as_ref()?,
                    ctx,
                    DeclaredReadProofs {
                        exists_guards: &exists_guards,
                        proved: &w210_min,
                    },
                )
            });
            suppress_declared_read_warnings(fu, ctx, declared_flow.as_deref(), &mut w210_min);
            self.emit_w210_read_spans(w210_min, ctx);
            self.emit_declaration_potential_reads(potential);
            return;
        }

        // An authored declaration can delimit a diagnostic read without
        // overriding unknown writes, accepted operand layouts or SSA origins.
        let declaration_advice = ir_proc.and_then(|procedure| {
            crate::script_binds::authored_procedure_read_advice(
                &procedure.body.executed_source.as_ref().map_or_else(
                    || tcl_lexer::SourceImage::document(&self.source),
                    |source| source.origin.source_image().clone(),
                ),
                procedure,
                &self.result,
                &self.analysis_context(),
            )
        });

        // Physical namespace write advice retains exact keys. Authored
        // host-name advice remains separate in extra_known_defined.
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        self.collect_chain_read_before_set_spans(
            fu,
            ctx,
            params,
            &exists_guards,
            declaration_advice.as_ref(),
            &mut w210_min,
        );

        let potential = declaration_advice.as_ref().and_then(|declaration| {
            self.declaration_potential_read_spans(
                fu,
                ir_proc?,
                declaration,
                declared_flow.as_ref()?,
                ctx,
                DeclaredReadProofs {
                    exists_guards: &exists_guards,
                    proved: &w210_min,
                },
            )
        });
        suppress_declared_read_warnings(fu, ctx, declared_flow.as_deref(), &mut w210_min);
        self.emit_w210_read_spans(w210_min, ctx);
        self.emit_declaration_potential_reads(potential);
    }

    fn collect_chain_read_before_set_spans(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReadBeforeSetCtx<'_>,
        params: &HashSet<&str>,
        exists_guards: &[super::helpers::ExistenceGuard],
        declaration_advice: Option<&crate::script_binds::AuthoredProcedureReadAdvice>,
        w210_min: &mut std::collections::HashMap<String, tcl_lexer::Span>,
    ) {
        use crate::def_use::DefKind;

        // W210 fires **once per variable**, at the earliest read-before-set.
        // The def-use walk below
        // visits *every* version-0 use, so record the earliest passing span
        // per variable here and emit after the walk (W213, a distinct code,
        // stays inline).
        for chain in fu.def_use.chains.values() {
            // Version-0 synthetic defs are the undef origin; an
            // `unset`-killed real version, and a phi version that can reach
            // an undef origin (one-branch `set` / try-handler merge), are
            // undef at their reads too — all flow through the same
            // suppression + emission logic below.
            if chain.definition.kind != DefKind::Parameter
                && !ctx.supp.killed.contains(&chain.key)
                && !ctx.supp.can_undef.contains(&chain.key)
            {
                continue;
            }
            let (cell_name, version) = &chain.key;
            let Some(symbol) = fu.ssa.cell_symbol(cell_name) else {
                continue;
            };
            let var = fu.ssa.var_name(symbol);
            if params.contains(var) {
                continue;
            }
            // Tcl 8.x's registry-declared `tcl_precision` read trace
            // recreates its value after `unset`; an eager startup binding
            // (for example argv) deliberately does not get this exemption.
            let startup = startup_read_facts(
                cell_name,
                *version,
                ctx.supp.killed.contains(&chain.key),
                ctx.initial_global,
                ctx.global_aliases,
                Some(self.analysis_context().context().authoring_query()),
            );
            if startup.lazy_read && ctx.supp.killed.contains(&chain.key) {
                continue;
            }
            // `SPECIAL_VARS` distinguishes recognised runtime-sensitive names
            // from the subset the default host actually makes readable before
            // user code. That entry fact belongs only to this document's
            // initial global frame and version zero: local shadows and a
            // version killed by `unset` are still genuine W210 reads.
            // An element read (`$arr(a)`) of an array whose *base* is
            // defined, aliased, or a parameter anywhere in the function
            // stays silent: which elements a dynamic write / whole-array
            // command created is not statically knowable, so only a read
            // of a wholly-unwritten, unaliased array reports. Policy sets
            // are base-keyed, so the base is checked for those too.
            let (base, element) = crate::naming::split_array_name_braced(var, true);
            if element.is_some() {
                let base_defined = fu
                    .def_use
                    .chains
                    .keys()
                    .any(|(key, version)| *version > 0 && cell_name.is_member_of(key))
                    || fu.ssa.blocks.values().any(|block| {
                        block.statements.iter().any(|statement| {
                            statement
                                .defs
                                .keys()
                                .any(|symbol| cell_name.is_member_of(fu.ssa.cell_key(*symbol)))
                        })
                    });
                if !ctx.supp.killed.contains(&chain.key)
                    && (base_defined
                        || params.contains(base)
                        || ctx.scope_aliases.contains(base)
                        || ctx.extra_known_defined.contains(base)
                        || ctx.supp.suppresses(base))
                {
                    continue;
                }
            }
            // A fully-qualified read (`$::myVar`, `$ns::var`) explicitly
            // targets the global / a named namespace scope, whose definition
            // may live in another proc, another namespace, or — for a
            // multi-file project — another file entirely.  Single-unit
            // dataflow cannot see those writers, so an otherwise-unresolved
            // qualified read is conservatively exempt. A same-unit `unset`
            // records a killed chain, however, so it must still be reported.
            if cell_name.namespace_membership_for_advice().is_some()
                && !ctx.supp.killed.contains(&chain.key)
            {
                continue;
            }
            // A scope-aliased local (`global` / `variable` / `upvar` /
            // `namespace upvar` — literal *or* dynamic target) is bound to a
            // variable in another scope, so reading it is not read-before-set.
            // `upvar 1 $name local` and `upvar 1 outer local` are semantically
            // identical: both raise `can't read` only when the *caller*
            // variable is missing — a runtime condition, not a static one — so
            // the dynamic target is suppressed exactly like the literal one
            // (matching C Tcl and the "assume the may-run path does run" stance
            // the loop / cross-event passes already take). A known `unset` of
            // the alias wins over that conservative assumption; a genuinely
            // unrelated local is absent from `scope_aliases` and still fires.
            if ctx.scope_aliases.contains(var) && !ctx.supp.killed.contains(&chain.key) {
                continue;
            }
            if ctx.extra_known_defined.contains(var) {
                continue;
            }
            // `dict with`/`dict update` unpacking + qualified-`variable`
            // alias tails suppress version-0 reads of the unpacked / aliased
            // names (the `puts $a` inside `dict with d {…}` is not RBS).
            // Interproc constant propagation resolves an empty caller dict to
            // CONST("") (keys = ∅, not unknown), so the blanket variant fires
            // on a genuine missing-key read while still suppressing an
            // unknown-shape (mixed-caller / no-caller) dict.
            if ctx.supp.suppresses(var) {
                continue;
            }
            self.record_chain_w210_uses(
                fu,
                chain,
                &W210ChainCtx {
                    cell_facts: ctx.cell_facts,
                    exists_guards,
                    supp: ctx.supp,
                    startup,
                },
                declaration_advice,
                w210_min,
            );
        }
    }

    fn emit_declaration_potential_reads(
        &mut self,
        potential: Option<std::collections::HashMap<String, tcl_lexer::Span>>,
    ) {
        if let Some(potential) = potential {
            let mut potential: Vec<_> = potential.into_iter().collect();
            potential.sort_by_key(|(_, span)| span.start());
            for (name, span) in potential {
                self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W210, span,
                    format!("Variable '{name}' may be read before it is set in the declared local frame"),
                    Severity::Warning,
                ));
            }
        }
    }

    /// Original declared reads absent from physical SSA remain conditional
    /// diagnostic occurrences. This projection neither defines a variable nor
    /// withdraws unknown writers, dispatch or callback alternatives.
    fn declaration_potential_read_spans(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        declaration: &crate::script_binds::AuthoredProcedureReadAdvice,
        report: &crate::command_binding::DeclarationFlowReport,
        ctx: &ReadBeforeSetCtx<'_>,
        proofs: DeclaredReadProofs<'_>,
    ) -> Option<std::collections::HashMap<String, tcl_lexer::Span>> {
        let DeclaredReadProofs {
            exists_guards,
            proved,
        } = proofs;
        if !report
            .owns_original_procedure(procedure, &tcl_lexer::SourceImage::document(&self.source))
        {
            return None;
        }
        let mut reads = std::collections::HashMap::new();
        for (name, &span) in report.declared_absence_warning_occurrences() {
            if !declaration.owns(span)
                || !report.allows_declared_absence_warning(name, span)
                || proved.contains_key(name)
                || ctx.scope_aliases.contains(name)
                || report.has_declared_alias(name)
                || ctx.supp.suppresses(name)
                || ctx.extra_known_defined.contains(name)
                || super::helpers::original_array_scalar_read_at_span(
                    fu,
                    span,
                    &self.profile_registry(),
                )
            {
                continue;
            }
            let killed = super::helpers::original_read_occurrence_at_span(fu, span)
                .is_some_and(|value| ctx.supp.killed.contains(&value));
            let guarded = !killed
                && fu.ssa.blocks.iter().any(|(&block, data)| {
                    data.statements.iter().enumerate().any(|(index, _)| {
                        let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                        view.source_tokens().is_some_and(|tokens| {
                            tokens.variable_accesses.iter().any(|access| {
                                let original = fu.abs_span(access.source.span);
                                original.start() <= span.start()
                                    && span.end() <= original.end()
                                    && super::helpers::original_read_cell(
                                        access,
                                        &self.profile_registry(),
                                    )
                                    .is_some_and(|cell| {
                                        exists_guards.iter().any(|(guard, dominator)| {
                                            guard == &cell
                                                && block_dominated_by(&fu.ssa, block, *dominator)
                                        })
                                    })
                            })
                        })
                    })
                });
            if !guarded {
                reads.insert(name.clone(), span);
            }
        }
        Some(reads)
    }

    /// Potential missing contents require closed original read alternatives,
    /// independently of SSA versions. Unknown worlds and existing array roots
    /// cannot donate an undef finding.
    fn definite_missing_read_spans(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReadBeforeSetCtx<'_>,
        exists_guards: &[super::helpers::ExistenceGuard],
    ) -> std::collections::HashMap<String, tcl_lexer::Span> {
        let mut missing = std::collections::HashMap::new();
        let registry = self.profile_registry();
        let grammar = self.lexer_config().braced_var;
        for (&block, data) in &fu.ssa.blocks {
            if !fu
                .diagnostic_value_facts()
                .executable_blocks()
                .contains(&block)
            {
                continue;
            }
            for index in (0..data.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                for access in &tokens.variable_accesses {
                    if !view
                        .read_contents_presence_alternatives_at(
                            &access.source,
                            &access.original_spelling,
                            &registry,
                        )
                        .is_some_and(crate::ssa::SsaReadPresenceAlternatives::may_be_undefined)
                    {
                        continue;
                    }
                    // Reading a live array as a scalar is a wrong-kind error,
                    // rather than evidence that the root cell is undefined.
                    if super::helpers::original_read_is_live_array_scalar(access, &registry) {
                        continue;
                    }
                    let read_cell = super::helpers::original_read_cell(access, &registry);
                    let read_reference =
                        view.read_reference(&access.source, &access.original_spelling);
                    let killed = read_reference.is_some_and(|read| {
                        read.version.is_some_and(|version| {
                            ctx.supp
                                .killed
                                .contains(&(fu.ssa.cell_key(read.symbol).clone(), version))
                        })
                    });
                    let name = tcl_syntax::naming::var_reference_for_style(
                        &access.original_spelling,
                        grammar,
                    )
                    .to_owned();
                    if ctx.supp.suppresses(&name)
                        || !killed
                            && exists_guards.iter().any(|(guarded, dominator)| {
                                read_cell.as_ref().is_some_and(|cell| guarded == cell)
                                    && block_dominated_by(&fu.ssa, block, *dominator)
                            })
                    {
                        continue;
                    }
                    let incoming = access.context_alternatives().iter().all(|context| {
                        let place = access.place_in_context(context, &registry);
                        context.contents_origin(&place)
                            == crate::var_resolve::ContentsOrigin::Incoming
                    });
                    let startup_readable = read_cell.as_ref().is_some_and(|cell| {
                        let version = read_reference.and_then(|read| read.version).unwrap_or(0);
                        let facts = startup_read_facts(
                            cell,
                            version,
                            killed,
                            ctx.initial_global,
                            ctx.global_aliases,
                            Some(self.analysis_context().context().authoring_query()),
                        );
                        facts.lazy_read || incoming && facts.readable
                    });
                    if startup_readable
                        || read_cell
                            .as_ref()
                            .is_some_and(|cell| ctx.cell_facts.known_defined.contains(cell))
                        || ctx.extra_known_defined.contains(name.as_str())
                    {
                        continue;
                    }
                    let span = fu.abs_span(access.source.span);
                    missing
                        .entry(name)
                        .and_modify(|old: &mut tcl_lexer::Span| {
                            if span.start() < old.start() {
                                *old = span;
                            }
                        })
                        .or_insert(span);
                }
            }
        }
        missing
    }

    fn emit_w210_read_spans(
        &mut self,
        spans: std::collections::HashMap<String, tcl_lexer::Span>,
        ctx: &ReadBeforeSetCtx<'_>,
    ) {
        use std::fmt::Write as _;
        let mut entries: Vec<(String, tcl_lexer::Span)> = spans.into_iter().collect();
        entries.sort_by_key(|(_, s)| s.start());
        for (var, span) in entries {
            let mut message = format!("Variable '{var}' is read before it is set");
            if let Some(similar) = undefined_var_suggestion(&var, ctx.defined_vars) {
                let _ = write!(message, "; did you mean '{similar}'?");
            }
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

    /// Record the earliest read-before-set span for one undef def-use chain
    /// (and emit any W213 `unset`-without-`-nocomplain`).  Walks the chain's
    /// uses, skipping phi-incoming pseudo-uses, auto-creating read-modify-write
    /// targets, existence-guarded reads, and use sites that safely initialise
    /// the variable; survivors update `w210_min` with the earliest read span.
    #[allow(clippy::too_many_lines)] // one pass keeps W210's ordered exemptions and evidence together
    fn record_chain_w210_uses(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        chain: &crate::def_use::DefUseChain,
        ctx: &W210ChainCtx<'_>,
        declaration_advice: Option<&crate::script_binds::AuthoredProcedureReadAdvice>,
        w210_min: &mut std::collections::HashMap<String, tcl_lexer::Span>,
    ) {
        use crate::def_use::UseKind;
        use crate::ir::Statement;

        let (cell_name, _version) = &chain.key;
        let Some(symbol) = fu.ssa.cell_symbol(cell_name) else {
            return;
        };
        let var = fu.ssa.var_name(symbol);
        for use_site in &chain.uses {
            if matches!(use_site.kind, UseKind::PhiIncoming) {
                continue;
            }
            // A `UseClass::Quoted` use is carried only by a brace-quoted word
            // the statement does not substitute (`puts {$y}` prints `$y` and
            // reads nothing). The use exists so liveness stays conservative
            // about a word that may be evaluated later; it is not a read
            // here, so it can never be read-*before*-set.
            if use_site.class == crate::ssa::UseClass::Quoted {
                continue;
            }
            // An after-loop read of a variable the loop body defines on every
            // iteration is not read-before-set (see
            // `UndefSuppression::loop_entry_only_undef`): we assume a may-run
            // loop runs, matching C Tcl. A read *inside* the loop body still
            // fires.
            if ctx.supp.after_loop_defined(&chain.key, &use_site.block) {
                continue;
            }
            let Some(block) = fu.cfg.block_by_name(&use_site.block) else {
                continue;
            };
            let (span, stmt_opt): (tcl_lexer::Span, Option<&Statement>) =
                if use_site.statement_index == -1 {
                    let Some(span) = block
                        .terminator
                        .as_ref()
                        .and_then(crate::cfg::Terminator::span)
                    else {
                        continue;
                    };
                    (fu.abs_span(span), None)
                } else {
                    let Ok(idx) = usize::try_from(use_site.statement_index) else {
                        continue;
                    };
                    let Some(stmt) = block.statements.get(idx) else {
                        continue;
                    };
                    (fu.abs_span(stmt.span()), Some(stmt))
                };
            if span.is_empty() || declaration_advice.is_some_and(|advice| !advice.owns(span)) {
                continue;
            }
            let Some(use_id) = fu.ssa.block_id(&use_site.block) else {
                continue;
            };
            let use_index = usize::try_from(use_site.statement_index).unwrap_or(usize::MAX);
            let source_view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, use_id, use_index);
            if source_view.owns_activation_cell(symbol) == Some(false)
                && !super::helpers::original_read_cells_at(
                    &fu.ssa,
                    (use_id, use_index),
                    cell_name,
                    &self.profile_registry(),
                )
                .contains(cell_name)
            {
                continue;
            }
            if super::helpers::read_has_cell_fact(
                fu,
                (use_id, use_index),
                cell_name,
                &ctx.cell_facts.known_defined,
                &self.profile_registry(),
            ) {
                continue;
            }
            if let Ok(index) = usize::try_from(use_site.statement_index) {
                let tokens =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, use_id, index).source_tokens();
                let array_read = tokens
                    .into_iter()
                    .flat_map(|tokens| &tokens.variable_accesses)
                    .filter(|access| {
                        fu.ssa
                            .read_reference_at(
                                use_id,
                                index,
                                &access.source,
                                &access.original_spelling,
                            )
                            .is_some_and(|read| fu.ssa.cell_key(read.symbol) == cell_name)
                    })
                    .any(|access| {
                        super::helpers::original_read_is_live_array_scalar(
                            access,
                            &self.profile_registry(),
                        )
                    });
                if array_read {
                    continue;
                }
            }
            // A `$var` read inside an opaque body-role script that also
            // defines `var` earlier in the same script reads that script's
            // *own* local, not the outer variable — so it is not a read of an
            // undefined outer name. `interp eval PATH { set x 1; expr {$x + 1}
            // }` runs its body in a child interpreter whose statements are
            // never flattened into this function's CFG, leaving the whole
            // script scanned as one `Statement::Barrier` value: the `$x` read
            // and the body-local `set x` collapse onto that single statement,
            // so the version-0 chain shows a read with no visible def and
            // W210 would false-fire. This is the only place the body-local
            // write is visible.
            if barrier_body_locally_sets(
                stmt_opt,
                var,
                &self.source,
                &self.result,
                &self.analysis_context(),
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, use_id, use_index).source_tokens(),
            ) {
                continue;
            }
            // A statement the lowering synthesised to carry an effect — the
            // `<cond>` placeholder holding a branch condition's substitution
            // reads, or `<upvar-invalidate>` holding a word's — has no source
            // word to anchor a read at: its span is the whole `if` or the whole
            // host statement. And the reads it carries are precisely the
            // existence-tolerant ones — `[info exists x]` is the idiom for a
            // name that may be unset, `[incr n]` creates its target on 8.5+ —
            // so a read recorded there is not evidence of a read-before-set.
            // Recording them made W210 fire on
            // `if {[info exists q]} {…} else {…}`, which tclsh runs cleanly
            // (#2132).
            if stmt_opt.is_some_and(statement_is_synthetic_effect) {
                continue;
            }
            // Skip the existence-query word itself and
            // reads narrowed by an enclosing `[info exists X]` guard.
            if existence_exempt(
                stmt_opt,
                cell_name,
                ctx.exists_guards,
                fu,
                use_site,
                self.analysis_context().commands(),
            ) {
                continue;
            }
            // ``unset`` without ``-nocomplain`` → W213.
            if let Some(Statement::Call {
                command,
                args,
                tokens,
                ..
            }) = stmt_opt
                && command == "unset"
                && !args.iter().any(|a| a == "-nocomplain")
            {
                // Eager startup bindings (`argv`, `tcl_version`, …) already
                // exist when their first `unset` runs. A lazy read trace is
                // different: its first `unset` still errors until an earlier
                // actual read has materialised its value in this block.
                if ctx.startup.initially_bound
                    || (ctx.startup.lazy_read
                        && chain.uses.iter().any(|prior| {
                            prior.block == use_site.block
                                && prior.statement_index >= 0
                                && prior.statement_index < use_site.statement_index
                                && prior.class != crate::ssa::UseClass::Quoted
                        }))
                {
                    continue;
                }
                let message = format!(
                    "Variable '{var}' may not exist; \
                         use 'unset -nocomplain' to suppress the error",
                );
                // Narrow the squiggle to the offending variable word (so
                // `unset a b c` flags only the missing name), and attach a
                // quick fix that inserts `-nocomplain` right after `unset` —
                // the same fix the LSP layer synthesises, carried on the
                // diagnostic itself so every editor surfaces it uniformly.
                let (diag_span, fixes) = w213_span_and_fix(fu, tokens.as_ref(), var, span);
                self.result.diagnostics.push(
                    crate::analyser::types::Diagnostic::new(
                        DiagCode::W213,
                        diag_span,
                        message,
                        Severity::Warning,
                    )
                    .with_fixes(fixes),
                );
                continue;
            }
            // A use site that itself safely initialises the variable
            // (`safe_on_uninit` calls like `lappend`/`dict set`, or an
            // `incr` of its own target) is not read-before-set.
            if use_site_safe_initialises(
                stmt_opt,
                fu,
                use_site,
                cell_name,
                &self.profile_registry(),
            ) {
                continue;
            }
            // This is an ordinary initial read, not the destructive `unset`
            // target handled above.  Its startup fact is registry-owned and
            // applies to the initial global frame, a qualified global, or a
            // registry-declared global alias — never a same-named local.
            if ctx.startup.readable {
                continue;
            }
            // Anchor at the `$var` read token; fall back to the command
            // span when the read is nested inside a quoted/compound word.
            let read_span = self.narrow_to_read_var(span, var).unwrap_or(span);
            w210_min
                .entry(var.to_owned())
                .and_modify(|s| {
                    if read_span.start() < s.start() {
                        *s = read_span;
                    }
                })
                .or_insert(read_span);
        }
    }

    /// W210 on `return $v` reads where `v`'s reaching version can be
    /// undefined on some executable path (phi-from-undef / `unset`-killed).
    /// Companion to [`Self::emit_read_before_set_diagnostics`]; see its
    /// trailing call site for why the def-use-chain pass cannot catch
    /// these (return values are terminator reads, not recorded uses).
    pub(super) fn emit_return_phi_undef_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReturnUndefCtx<'_>,
    ) {
        use crate::var_refs::{VarReferenceScanner, VarScanOptions};
        use std::fmt::Write as _;

        // `defined_vars` / `considered` are used directly here; the remaining
        // sets are consulted by `return_read_fires_w210` through `ctx`.
        let defined_vars = ctx.defined_vars;
        let considered = ctx.considered;

        let Some(registry) = self.registry.as_deref() else {
            return;
        };

        let (phi_def, phi_block, killed) = build_phi_undef_index(&fu.ssa, considered, registry);
        let phi_idx = PhiUndefIndex {
            phi_def: &phi_def,
            phi_block: &phi_block,
            killed: &killed,
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

        let mut reported: FxHashSet<String> = FxHashSet::default();
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
                reported.insert(name.clone());
                let mut message = format!("Variable '{name}' is read before it is set");
                if let Some(similar) = undefined_var_suggestion(&name, defined_vars) {
                    let _ = write!(message, "; did you mean '{similar}'?");
                }
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
    /// known-defined / qualified / suppressed name or be proven
    /// defined by a dominating existence guard.  Version-0 reads are handled by
    /// the def-use `DefKind::Parameter` emitter, so they never fire here.
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
        let undef_ctx = super::helpers::PhiUndefCtx {
            phi_def: phi_idx.phi_def,
            phi_block: phi_idx.phi_block,
            killed: phi_idx.killed,
            considered: ctx.considered,
            executable_edges: fu.diagnostic_value_facts().executable_edges(),
            exists_guards: ctx.exists_guards,
            initial_global: ctx.initial_global,
            global_aliases: ctx.global_aliases,
            dialect: ctx.dialect,
            ssa: &fu.ssa,
        };
        let cell_name = fu.ssa.cell_key(symbol);
        if !phi_can_undef(cell_name, ver, &undef_ctx, memo) {
            return false;
        }
        // A killed SSA version is concrete same-unit evidence that overrides
        // the conservative external-scope assumptions for `global`/`upvar`
        // aliases and qualified names.
        let known_killed = phi_idx.killed.contains(&(cell_name.to_owned(), ver));
        if ctx.params.contains(name)
            || (ctx.scope_aliases.contains(name) && !known_killed)
            || super::helpers::read_has_cell_fact(
                fu,
                (bn, usize::MAX),
                cell_name,
                &ctx.cell_facts.known_defined,
                &ctx.registry,
            )
            || ctx.extra_known_defined.contains(name)
            || (cell_name.namespace_membership_for_advice().is_some() && !known_killed)
            || ctx.supp.suppresses(name)
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
        let original_cells = super::helpers::original_read_cells_at(
            &fu.ssa,
            (bn, usize::MAX),
            cell_name,
            &ctx.registry,
        );
        if !known_killed
            && ctx.exists_guards.iter().any(|(guard, block)| {
                (original_cells.contains(guard)
                    || fu.ssa.point_contexts.is_none() && guard == cell_name)
                    && block_dominated_by(&fu.ssa, bn, *block)
            })
        {
            return false;
        }
        true
    }

    /// W210 from a selected original matcher that leaves its targets unchanged.
    /// Logical source advice follows the same represented SSA value; Native
    /// emission additionally requires independent closed read-absence proof.
    pub(super) fn emit_provably_unset_w210(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<crate::cfg::BlockId>,
        ctx: &ReadBeforeSetCtx<'_>,
        params: &HashSet<&str>,
    ) {
        // naming.diagnostics.original-matcher-output-retention
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
        let Some(input) = self.result.resolved_input.as_ref() else {
            return;
        };
        let context = input.context_registry();
        let logical = self.result.allows_retained_logical_declaration_advice();
        let origins = self.collect_no_match_origins(fu, considered, &context, logical);
        let mut spans = std::collections::HashMap::new();
        for &block in considered {
            let Some(data) = fu.ssa.blocks.get(&block) else {
                continue;
            };
            for (index, statement) in data.statements.iter().enumerate() {
                let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                for (&symbol, &version) in &statement.uses {
                    if statement.quoted_uses.contains(&symbol)
                        || statement.name_only_uses.contains(&symbol)
                        || !origins.get(&symbol).is_some_and(|candidates| {
                            candidates.iter().any(|origin| {
                                origin.version == version && origin.reaches(fu, block, index)
                            })
                        })
                    {
                        continue;
                    }
                    let name = fu.ssa.var_name(symbol);
                    let span = if logical {
                        if params.contains(name)
                            || ctx.scope_aliases.contains(name)
                            || ctx.extra_known_defined.contains(name)
                            || ctx.supp.suppresses(name)
                            || ctx
                                .cell_facts
                                .known_defined
                                .contains(fu.ssa.cell_key(symbol))
                            || fu.dynamic_names.writes
                        {
                            continue;
                        }
                        let startup = startup_read_facts(
                            fu.ssa.cell_key(symbol),
                            0,
                            false,
                            ctx.initial_global,
                            ctx.global_aliases,
                            Some(context.context().authoring_query()),
                        );
                        if startup.readable {
                            continue;
                        }
                        let Some(statement) = fu
                            .cfg
                            .blocks
                            .get(&block)
                            .and_then(|data| data.statements.get(index))
                        else {
                            continue;
                        };
                        let Some(span) =
                            self.narrow_to_read_var(fu.abs_span(statement.span()), name)
                        else {
                            continue;
                        };
                        span
                    } else {
                        let Some(span) = original_missing_matcher_read_span(
                            fu,
                            view,
                            symbol,
                            context.commands(),
                        ) else {
                            continue;
                        };
                        span
                    };
                    spans
                        .entry(name.to_owned())
                        .and_modify(|earlier: &mut tcl_lexer::Span| {
                            if span.start() < earlier.start() {
                                *earlier = span;
                            }
                        })
                        .or_insert(span);
                }
            }
        }
        self.emit_w210_read_spans(spans, ctx);
    }

    fn collect_no_match_origins(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        considered: &HashSet<crate::cfg::BlockId>,
        context: &tcl_registry::model::ContextRegistry,
        logical: bool,
    ) -> std::collections::HashMap<crate::ssa::Symbol, Vec<NoMatchOutputOrigin>> {
        use crate::registry_invocation::source_structure::original_registry_words_for_tokens;
        let mut origins = std::collections::HashMap::new();
        for &block in considered {
            let Some(data) = fu.cfg.blocks.get(&block) else {
                continue;
            };
            for index in 0..data.statements.len() {
                let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let Some(words) =
                    original_registry_words_for_tokens(&self.source, &self.result, tokens)
                else {
                    continue;
                };
                record_no_match_outputs(
                    fu,
                    NoMatchCallSite {
                        producer: block,
                        index,
                        dominator: block,
                        branch_entry: false,
                    },
                    &selected_matcher_no_match_outputs(&words, context),
                    logical,
                    &mut origins,
                );
            }
            if let Some(crate::cfg::Terminator::Branch {
                condition,
                condition_base,
                true_target,
                false_target,
                ..
            }) = &data.terminator
                && let Some((words, negated)) =
                    self.original_condition_matcher(condition, *condition_base)
            {
                let outputs = selected_matcher_no_match_outputs(&words, context);
                let head = words
                    .head_source()
                    .and_then(|head| head.word())
                    .map(tcl_lexer::NativeWord::span);
                let index = (0..data.statements.len())
                    .find(|&index| {
                        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                            .source_tokens()
                            .and_then(|tokens| tokens.words().first())
                            .is_some_and(|word| Some(word.source().span) == head)
                    })
                    .unwrap_or(data.statements.len());
                record_no_match_outputs(
                    fu,
                    NoMatchCallSite {
                        producer: block,
                        index,
                        dominator: if negated { *true_target } else { *false_target },
                        branch_entry: true,
                    },
                    &outputs,
                    logical,
                    &mut origins,
                );
            }
        }
        origins
    }

    /// The condition's retained absolute source base owns expression offsets.
    /// Detached text, transformed expressions and multiple commands decline.
    fn original_condition_matcher(
        &self,
        condition: &ExprNode,
        base: Option<u32>,
    ) -> Option<(
        crate::registry_invocation::source_structure::OriginalRegistryWords,
        bool,
    )> {
        let (command, negated) = match condition {
            ExprNode::Command { .. } => (condition, false),
            ExprNode::Unary {
                op: UnaryOp::Not | UnaryOp::WordNot,
                operand,
            } if matches!(operand.as_ref(), ExprNode::Command { .. }) => (operand.as_ref(), true),
            _ => return None,
        };
        let ExprNode::Command { text, start, end } = command else {
            return None;
        };
        let start = base?.checked_add(*start)?;
        let end = base?.checked_add(*end)?;
        if self
            .source
            .get(usize::try_from(start).ok()?..usize::try_from(end).ok()?)?
            != text
        {
            return None;
        }
        let inner = text.strip_prefix('[')?.strip_suffix(']')?;
        let config = self.result.body_lexer_config?;
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            inner,
            start.checked_add(1)?,
            config,
        );
        let [command] = commands.as_slice() else {
            return None;
        };
        crate::registry_invocation::source_structure::source_registry_words(
            &self.source,
            &self.result,
            command,
        )
        .map(|words| (words, negated))
    }

    /// I230 / I231 — constant branch / switch-arm condition.
    ///
    /// For every
    /// branch SCCP folded to a constant, when the *not-taken*
    /// target is also unreachable (i.e. SCCP confirmed only one
    /// path is feasible), emit an Info-level diagnostic so the
    /// LSP can highlight the dead arm.
    ///
    /// Code selection:
    /// - Block name starts with ``switch_`` → I231 (switch-arm).
    /// - Block name starts with ``if_`` → I230 (constant if).
    /// - Otherwise → I230 with the generic
    ///   ``"Branch condition '...' is constant"`` message.
    ///
    /// Severity is mapped to ``Hint`` because the
    /// [`Severity`] enum has no ``Info`` variant — ``Hint`` is
    /// the closest non-actionable level.
    pub(super) fn emit_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        for branch in fu.diagnostic_value_facts().constant_branches() {
            // A branch is dead when the not-taken target is
            // unreachable.  SCCP exposes
            // ``executable_blocks`` (the complement); a block
            // is unreachable iff it's in ``cfg.blocks`` but
            // NOT in ``executable_blocks``.
            if fu.cfg.block_id(&branch.not_taken_target).is_some_and(|id| {
                fu.diagnostic_value_facts()
                    .executable_blocks()
                    .contains(&id)
            }) {
                continue;
            }
            // Locate the branch's terminator span.
            let Some(block) = fu.cfg.block_by_name(&branch.block) else {
                continue;
            };
            let Some(crate::cfg::Terminator::Branch {
                span: Some(span), ..
            }) = &block.terminator
            else {
                continue;
            };
            let span = fu.abs_span(*span);

            let names = [
                branch.block.as_str(),
                branch.taken_target.as_str(),
                branch.not_taken_target.as_str(),
            ];
            let is_switch = names.iter().any(|n| n.starts_with("switch_"));
            let is_if = names.iter().any(|n| n.starts_with("if_"));
            let is_loop = names.iter().any(|n| {
                n.starts_with("while_") || n.starts_with("for_") || n.starts_with("foreach_")
            });
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
                         the alternate branch is unreachable",
                        branch.condition,
                    )
                };
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

    /// I230 — fold `[info exists X]` / `[array exists X]` conditions.
    ///
    /// SCCP can't fold these (the predicate lowers to an
    /// opaque `ExprNode::Command`, and SCCP has no parameter/existence
    /// facts), so the fold is computed by
    /// [`crate::sccp::existence_constant_branches`] using the frame's formal
    /// parameters — the same helper whose result
    /// `FunctionUnit::build` appends to `sccp.constant_branches` for the
    /// optimiser's O101 fold / DCE.  Emitting the I230 here (rather than
    /// via [`Self::emit_constant_branch_diagnostics`]) is deliberate:
    /// that emitter gates on the not-taken arm being unreachable in
    /// `executable_blocks`, which these post-pass folds don't update, so
    /// it skips them and there is no double emission.
    ///
    /// `frame` supplies the typed entry facts for whichever kind of body
    /// this is: a procedure contributes its parameters, a
    /// `TclOO` method body contributes its parameters **and** its class's
    /// instance variables, on which the fold must abstain.  Both halves come
    /// from the same IR the optimiser's copy of the fold reads, so the two
    /// consumers cannot drift.
    pub(super) fn emit_existence_constant_branch_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        frame: crate::sccp::ExistenceFrame<'_>,
    ) {
        // The fold consults the registry's scope-alias roles to skip
        // out-of-frame-linked locals; a registry-less analyser falls back to
        // the cached default registry (the same convention as
        // `command_takes_regex_pattern` — direct handler calls in unit
        // tests), so the alias skip stays sound there too.
        let branches = {
            // Scoped borrow: `self.registry.as_deref()` must release before the
            // `&mut self` diagnostic pushes below.
            let generation = self.analysis_context();
            let registry = self.registry.as_deref().unwrap_or(generation.commands());
            crate::sccp::existence_constant_branches_with_ssa(
                &fu.cfg,
                frame,
                registry,
                self.lexer_config(),
                &fu.ssa,
            )
        };
        for cb in branches {
            let Some(span) = cb.span.map(|s| fu.abs_span(s)) else {
                continue;
            };
            let message = if cb.value {
                format!(
                    "Condition '{}' is always true; the alternate branch is unreachable",
                    cb.condition,
                )
            } else {
                format!(
                    "Condition '{}' is always false; the alternate branch is unreachable",
                    cb.condition,
                )
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::I230,
                    span,
                    message,
                    // I230 is observational (LSP `Information`).
                    Severity::Info,
                ));
        }
    }

    /// W126 — channel-argument validation.
    ///
    /// Walks every
    /// SSA-annotated `Call` statement for commands that declare
    /// `ArgRole::Channel` arguments; for each channel-position
    /// argument, checks the SSA type lattice to determine whether
    /// the value is genuinely a channel.  Two failure modes:
    ///
    /// - **`$var` reference** with `TypeKind::Known` and a non-
    ///   `TclType::Channel` type — emits "passed as channel … has
    ///   type X, not CHANNEL".
    /// - **String literal** that isn't `stdin` / `stdout` /
    ///   `stderr` and contains no substitutions — emits
    ///   "String literal 'X' used as channel argument".
    ///
    /// The standard channels (`stdin`, `stdout`, `stderr`) are
    /// always accepted.  Unknown / overdefined types skip the
    /// check (could be anything).
    pub(super) fn emit_channel_diagnostics(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        let context = self.analysis_context();
        for (&bn, block) in &fu.ssa.blocks {
            for (statement_index, ssa_stmt) in block.statements.iter().enumerate() {
                let crate::ir::Statement::Call { command, .. } = &ssa_stmt.statement else {
                    continue;
                };
                let Some(tokens) =
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, bn, statement_index)
                        .source_tokens()
                else {
                    continue;
                };
                let arguments = super::original_roles::channel_arguments(
                    &self.source,
                    &self.result,
                    tokens,
                    std::sync::Arc::clone(&context),
                );
                for argument in arguments {
                    let message = if let Some((name, reference_span)) =
                        super::original_roles::scalar_variable_reference(&argument.word)
                    {
                        let mut reads = tokens
                            .variable_accesses
                            .iter()
                            .filter(|access| access.source.span == reference_span)
                            .filter_map(|access| {
                                fu.ssa.read_reference_at(
                                    bn,
                                    statement_index,
                                    &access.source,
                                    &access.original_spelling,
                                )
                            });
                        let Some(read) = reads.next() else {
                            continue;
                        };
                        if reads.any(|other| other != read) {
                            continue;
                        }
                        let sym = read.symbol;
                        let Some(version) = read.version else {
                            continue;
                        };
                        let Some(var_type) = fu.types.get(&(sym, version)) else {
                            continue;
                        };
                        let Some(type_label) = non_channel_union_label(var_type) else {
                            continue;
                        };
                        format!(
                            "Variable '${name}' passed as channel to '{command}' has type {type_label}, not CHANNEL."
                        )
                    } else {
                        let Some(literal) = argument.literal else {
                            continue;
                        };
                        if matches!(literal.as_slice(), b"stdin" | b"stdout" | b"stderr") {
                            continue;
                        }
                        let Ok(literal) = std::str::from_utf8(&literal) else {
                            continue;
                        };
                        format!(
                            "String literal '{literal}' used as channel argument to '{command}' — expected a channel from open/socket/chan create."
                        )
                    };
                    self.result.diagnostics.push(
                        crate::analyser::types::Diagnostic::new(
                            DiagCode::W126,
                            argument.word.span(),
                            message,
                            Severity::Warning,
                        )
                        .with_subject(argument.subject),
                    );
                }
            }
        }
    }

    /// W124 — invalid IP address literal.
    ///
    /// Walks every
    /// SSA-tracked constant string in the function's SCCP
    /// values; regex-searches for IPv4 dotted-quad and IPv6
    /// candidates and validates each.
    ///
    /// **Validation:**
    /// - **IPv4** — each octet must be 0..255; leading-zero
    ///   octets emit a Warning (interpreted as octal in some
    ///   contexts); over-255 octets emit an Error.  Patterns
    ///   preceded by ``/`` (CIDR / version-number context) are
    ///   skipped.
    /// - **IPv6** — parsed via [`std::net::Ipv6Addr`]; failure
    ///   emits an Error.
    ///
    /// Diagnostic anchors at the SSA def site (the assignment
    /// statement's span); seen-offsets dedup avoids duplicate
    /// emissions when multiple SSA versions share a def.
    /// **W233.** Division / modulo by a provably-zero divisor — raises
    /// "divide by zero" at runtime.  Delegates to the canonical
    /// interval-bounds analysis [`crate::interval_bounds::find_divide_by_zero`]
    /// (the single source of truth, shared with the interval-bounds index
    /// checks): a `/` or `%` whose divisor's interval — guard-narrowed at the
    /// use site and seeded from the SCCP lattice — is exactly `[0, 0]`, on the
    /// always-evaluated spine of an executable expression.
    ///
    /// (Verified against tclsh 8.4–9.0: integer `1/0` and `5%0` raise "divide
    /// by zero"; float division such as `1.0/0` yields `Inf` and does not
    /// error. The interval domain is integer, matching that boundary for the
    /// common cases.)
    pub(super) fn emit_w233_divide_by_zero(&mut self, fu: &crate::compilation_unit::FunctionUnit) {
        // The block set SCCP proved reachable; fall back to every SSA block
        // when SCCP produced nothing (e.g. a trivial function) so the check
        // still runs.
        let executable: HashSet<crate::cfg::BlockId> =
            if fu.diagnostic_value_facts().executable_blocks().is_empty() {
                fu.ssa.blocks.keys().copied().collect()
            } else {
                fu.diagnostic_value_facts().executable_blocks().clone()
            };
        let context = self.analysis_context();
        let registry = context.commands();
        for finding in crate::interval_bounds::find_divide_by_zero_with_entered_operands(
            &fu.cfg,
            &fu.ssa,
            fu.diagnostic_value_facts().values(),
            &executable,
            // The document's own numeral grammar: a divisor literal means what
            // this dialect says it means (`0755` is 493 up to 8.6, 755 from
            // 9.0), and this process analyses documents of several dialects.
            crate::intervals::numbers_for_dialect(Some(self.profile)),
            crate::interval_bounds::BoundsSemantics {
                registry,
                context: Some(context.as_ref().into()),
                grammar: self.grammar(),
            },
        ) {
            let span = fu.abs_span(finding.span);
            if span.is_empty() {
                continue;
            }
            let verb = if finding.op == "/" {
                "Division"
            } else {
                "Modulo"
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    DiagCode::W233,
                    span,
                    format!(
                        "{verb} by a provably-zero divisor — raises 'divide by zero' at runtime."
                    ),
                    Severity::Warning,
                ));
        }
    }

    /// **W230 / W231 / W232 (dynamic).** Interval-driven out-of-range index
    /// detection for a `$var` index whose [`crate::intervals`] range — guard-
    /// narrowed at the use site — proves the access is wholly out of range
    /// against a statically-established container length.  Complements the
    /// syntactic bounds checks (literal index + literal container only); the
    /// two never double-fire because the syntactic checks back off on any
    /// `$var` index.  Restricted to SCCP-reachable blocks so a dynamic index
    /// in dead code does not warn.
    pub(super) fn emit_interval_bounds_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        let executable: HashSet<crate::cfg::BlockId> =
            if fu.diagnostic_value_facts().executable_blocks().is_empty() {
                fu.ssa.blocks.keys().copied().collect()
            } else {
                fu.diagnostic_value_facts()
                    .executable_blocks()
                    .iter()
                    .copied()
                    .collect()
            };
        let context = self.analysis_context();
        let registry = context.commands();
        let findings = crate::interval_bounds::find_interval_bounds_resolved(
            &fu.cfg,
            &fu.ssa,
            fu.diagnostic_value_facts().values(),
            &executable,
            self.profile.character_model(),
            crate::intervals::numbers_for_dialect(Some(self.profile)),
            crate::interval_bounds::BoundsSemantics {
                registry,
                context: Some(context.as_ref().into()),
                grammar: self.grammar(),
            },
        );
        for f in findings {
            if f.span.is_empty() {
                continue;
            }
            let bound = if f.reason == "negative" {
                "below 0".to_string()
            } else {
                format!("past the end ({})", f.length)
            };
            let rng = if f.reason == "negative" {
                "negative".to_string()
            } else if f.index_interval.lo == f.index_interval.hi {
                format!("is {}", f.index_interval.lo.unwrap_or(0))
            } else {
                let lo = f
                    .index_interval
                    .lo
                    .map_or("-inf".to_string(), |l| l.to_string());
                let hi = f
                    .index_interval
                    .hi
                    .map_or("+inf".to_string(), |h| h.to_string());
                format!("is in [{lo}, {hi}]")
            };
            let outcome = if f.code == DiagCode::W231 {
                "raises 'index out of range' at runtime"
            } else {
                "silently returns the empty string"
            };
            self.result
                .diagnostics
                .push(crate::analyser::types::Diagnostic::new(
                    f.code,
                    fu.abs_span(f.span),
                    format!(
                        "{}: index ${} {rng}, {bound} \u{2014} {outcome}.",
                        f.command, f.index_var
                    ),
                    Severity::Warning,
                ));
        }
    }

    pub(super) fn emit_invalid_ip_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::analyses::{ConstValue, LatticeValue};
        use std::net::Ipv6Addr;
        use std::str::FromStr;

        let mut seen_offsets: FxHashSet<u32> = FxHashSet::default();
        for (key, lv) in fu.diagnostic_value_facts().values() {
            let Some(text) = (match lv {
                LatticeValue::Const(ConstValue::String(s)) => Some(s.as_str()),
                _ => None,
            }) else {
                continue;
            };

            // IPv4 candidates.
            for quad in find_dotted_quads(text, 4) {
                let bytes = text.as_bytes();
                if quad.start > 0 && bytes[quad.start - 1] == b'/' {
                    continue;
                }
                // Skip OID-like patterns: the matched quad is a slice of a
                // longer dotted-digit chain (LDAP/SNMP OIDs like
                // ``1.3.6.1.4.1.4203.1.11.3``).  Detect a ``digit.<quad>``
                // before or a ``<quad>.digit`` after.
                let before_dot_digit = quad.start >= 2
                    && bytes[quad.start - 1] == b'.'
                    && bytes[quad.start - 2].is_ascii_digit();
                let after_dot_digit = quad.end + 1 < bytes.len()
                    && bytes[quad.end] == b'.'
                    && bytes[quad.end + 1].is_ascii_digit();
                if before_dot_digit || after_dot_digit {
                    continue;
                }
                let octets = quad.octets;
                let mut diag: Option<(String, Severity)> = None;
                for (i, octet) in octets.iter().enumerate() {
                    let v: u32 = octet.parse().unwrap_or(0);
                    if v > 255 {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) exceeds 255 — this is not a valid IP address.",
                                i + 1,
                                octet,
                            ),
                            Severity::Error,
                        ));
                        break;
                    }
                    if octet.len() > 1
                        && octet.starts_with('0')
                        && octet.bytes().all(|b| (b'0'..=b'7').contains(&b))
                    {
                        diag = Some((
                            format!(
                                "IPv4 octet {} ({}) has a leading zero — may be interpreted as octal in some contexts.",
                                i + 1,
                                octet,
                            ),
                            Severity::Warning,
                        ));
                        break;
                    }
                }
                if let Some((msg, sev)) = diag {
                    let literal = &text[quad.start..quad.end];
                    self.emit_ip_diag_at_def(fu, *key, &msg, sev, Some(literal), &mut seen_offsets);
                    break;
                }
            }

            // IPv6 candidates.
            for candidate in find_ipv6_candidates(text) {
                if Ipv6Addr::from_str(candidate).is_err() {
                    let msg = format!("Invalid IPv6 address '{candidate}'.");
                    self.emit_ip_diag_at_def(
                        fu,
                        *key,
                        &msg,
                        Severity::Error,
                        Some(candidate),
                        &mut seen_offsets,
                    );
                    break;
                }
            }
        }
    }

    /// Helper for [`Self::emit_invalid_ip_diagnostics`].
    ///
    /// Anchors on the offending IP `literal` within the def statement when
    /// it appears verbatim in the source (with non-address bytes on both
    /// sides), falling back to the whole statement span when the constant
    /// was folded from parts the source never spells out.
    fn emit_ip_diag_at_def(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        key: crate::ssa::ValueKey,
        message: &str,
        severity: Severity,
        literal: Option<&str>,
        seen_offsets: &mut FxHashSet<u32>,
    ) {
        let (sym, version) = key;
        let var_name = fu.ssa.var_name(sym);
        let Some(chain) = fu.def_use.chain_for(var_name, version) else {
            return;
        };
        let Some(block) = fu.cfg.block_by_name(&chain.definition.block) else {
            return;
        };
        let Ok(idx) = usize::try_from(chain.definition.statement_index) else {
            return;
        };
        let Some(stmt) = block.statements.get(idx) else {
            return;
        };
        let stmt_span = fu.abs_span(stmt.span());
        if stmt_span.is_empty() {
            return;
        }
        if !seen_offsets.insert(stmt_span.start()) {
            return;
        }
        // Tight anchor: the literal's own bytes inside the statement, when
        // the source spells it out directly.
        let span = literal
            .and_then(|lit| {
                let slice = source_slice(&self.source, stmt_span)?;
                let is_addr_byte = |b: u8| b.is_ascii_hexdigit() || b == b'.' || b == b':';
                let mut from = 0;
                while let Some(off) = slice[from..].find(lit) {
                    let start = from + off;
                    let end = start + lit.len();
                    let left_ok = start == 0 || !is_addr_byte(slice.as_bytes()[start - 1]);
                    let right_ok = end >= slice.len() || !is_addr_byte(slice.as_bytes()[end]);
                    if left_ok && right_ok {
                        return Some(tcl_lexer::Span::new(
                            stmt_span.start() + u32::try_from(start).ok()?,
                            stmt_span.start() + u32::try_from(end).ok()?,
                        ));
                    }
                    from = start + 1;
                }
                None
            })
            .unwrap_or(stmt_span);
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W124,
                span,
                message.to_string(),
                severity,
            ));
    }

    /// Apply F5 storage policy to the shared point-resolved Tcl cells.
    /// Aliases and absolute namespace spellings retain the same host effects.
    pub(super) fn emit_irules_cell_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        qname: &str,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if !self.profile.is_irules() {
            return;
        }
        let Some(points) = fu.ssa.point_contexts.as_ref() else {
            return;
        };
        let context = self.analysis_context();
        if context.commands().snapshot().semantic_key() != registry.snapshot().semantic_key() {
            return;
        }
        let mut emitted = FxHashSet::default();
        for (&id, block) in &fu.cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                let before = points.before_statement(id, index);
                let after = points.after_statement(id, index);
                for access in irules_cell_accesses(statement, before, after, &context) {
                    self.emit_irules_cell_access(fu, qname, statement, access, &mut emitted);
                }
                for name in irules_possible_namespace_writes(statement, before, after, &context) {
                    let span = fu.abs_span(statement.span());
                    if span.is_empty()
                        || !emitted.insert((DiagCode::Irule6001, span.start(), name.clone()))
                    {
                        continue;
                    }
                    self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Irule6001,
                        span,
                        format!("Writing through namespace variable '{name}' may select shared global state, forcing CMP compatibility mode and pinning the virtual server to a single TMM."),
                        Severity::Warning,
                    ));
                }
            }
        }
    }

    fn emit_irules_cell_access(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        qname: &str,
        statement: &crate::ir::Statement,
        access: IrulesCellAccess,
        emitted: &mut FxHashSet<(DiagCode, u32, String)>,
    ) {
        use tcl_registry::f5::{BigIpExecutionContext, VariableStorageDomain};
        let IrulesCellAccess {
            place,
            import,
            implicit_destruction,
        } = access;
        if place.dynamic || !place.is_global() {
            return;
        }
        let domain =
            tcl_registry::f5::namespace_storage_domain(BigIpExecutionContext::TmmIRule, &place.ns);
        let code = match domain {
            VariableStorageDomain::WorkerNamespace
                if crate::ir::when_event_name(qname) != "RULE_INIT" && import.is_none() =>
            {
                DiagCode::Irule4001
            }
            VariableStorageDomain::CmpGlobal if !implicit_destruction => DiagCode::Irule6001,
            _ => return,
        };
        let target = crate::naming::qualify(&place.ns, &place.name);
        let span = fu.abs_span(statement.span());
        if span.is_empty() || !emitted.insert((code, span.start(), target.clone())) {
            return;
        }
        let message = if code == DiagCode::Irule4001 {
            format!(
                "Writing to '{target}' outside RULE_INIT changes persistent state for \
                     connections on this TMM. The update is not propagated to other TMMs."
            )
        } else if let Some(import) = import {
            format!(
                "'{import}' imports global namespace variable '{target}', forcing CMP compatibility mode and pinning the virtual server to a single TMM. Use 'static::{}' instead.",
                place.name
            )
        } else {
            format!(
                "Global namespace variable '{target}' forces CMP compatibility mode, \
                     pinning the virtual server to a single TMM. Use 'static::{}' instead.",
                place.name
            )
        };
        let fixes = if code == DiagCode::Irule6001 {
            self.irules_global_name_fix(fu, qname, statement, &place)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        self.result.diagnostics.push(
            crate::analyser::types::Diagnostic::new(code, span, message, Severity::Warning)
                .with_fixes(fixes),
        );
    }

    fn irules_global_name_fix(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        qname: &str,
        statement: &crate::ir::Statement,
        place: &crate::place::Place,
    ) -> Option<crate::analyser::types::CodeFix> {
        let (crate::ir::Statement::AssignConst { name, .. }
        | crate::ir::Statement::AssignValue { name, .. }
        | crate::ir::Statement::AssignExpr { name, .. }
        | crate::ir::Statement::Incr { name, .. }) = statement
        else {
            return None;
        };
        if !name.starts_with("::") && crate::ir::when_event_name(qname) != "RULE_INIT" {
            return None;
        }
        let (base, index) = crate::naming::split_array_name(name);
        let qualified = crate::naming::qualify("::", base);
        let (namespace, tail) = crate::naming::key_holder_and_tail(&qualified);
        if namespace != "::" || tail != place.name {
            return None;
        }
        let span = self.narrow_to_assigned_name(fu.abs_span(statement.span()))?;
        let replacement = index.map_or_else(
            || format!("static::{tail}"),
            |index| format!("static::{tail}({index})"),
        );
        Some(crate::analyser::types::CodeFix {
            span,
            new_text: replacement.clone(),
            description: format!("Replace '{name}' with '{replacement}'"),
            safety: crate::irules_checks::FixSafety::RequiresReview,
        })
    }

    /// IRULE4003: lifecycle concerns for actual shared connection cells.
    pub(super) fn emit_connection_scope_concerns(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: &str,
        concerns: &std::collections::HashMap<crate::connection_scope::EventCell, HashSet<String>>,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let points = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            crate::connection_scope::event_resolve_context(event),
            registry,
        );
        let mut emitted = FxHashSet::default();
        for (&id, block) in &fu.cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                if crate::connection_scope::statement_destroys(statement, registry) {
                    continue;
                }
                for place in crate::place_bridge::def_places(
                    statement,
                    points.before_statement(id, index),
                    registry,
                ) {
                    if place.dynamic || place.ns != crate::place::LOCAL_NS {
                        continue;
                    }
                    let Some(cell) = crate::connection_scope::cell_from_place(&place) else {
                        continue;
                    };
                    let Some(notes) = concerns.get(&cell) else {
                        continue;
                    };
                    let span = fu.abs_span(statement.span());
                    if span.is_empty() || !emitted.insert((span.start(), place.name.clone())) {
                        continue;
                    }
                    let mut notes: Vec<_> = notes.iter().map(String::as_str).collect();
                    notes.sort_unstable();
                    self.result
                        .diagnostics
                        .push(crate::analyser::types::Diagnostic::new(
                            DiagCode::Irule4003,
                            span,
                            format!("Variable '{}': {}", place.name, notes.join("; ")),
                            Severity::Hint,
                        ));
                }
            }
        }
    }

    /// IRULE4005: an actual worker cell write may be observed in another event.
    pub(super) fn emit_racy_static_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        event: &str,
        racy_cells: &HashSet<crate::connection_scope::EventCell>,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if self.disabled_diagnostics.contains("IRULE4005") {
            return;
        }
        let fallback_points;
        let points = if let Some(points) = &fu.ssa.point_contexts {
            points
        } else {
            fallback_points = crate::place_bridge::build_point_resolve_contexts_with_entry(
                &fu.cfg,
                crate::connection_scope::event_resolve_context(event),
                registry,
            );
            &fallback_points
        };
        let mut emitted = FxHashSet::default();
        for (&id, block) in &fu.cfg.blocks {
            for (index, statement) in block.statements.iter().enumerate() {
                if crate::connection_scope::statement_destroys(statement, registry) {
                    continue;
                }
                for place in crate::place_bridge::def_places(
                    statement,
                    points.before_statement(id, index),
                    registry,
                ) {
                    if place.dynamic || !place.is_global() {
                        continue;
                    }
                    let Some(cell) = crate::connection_scope::cell_from_place(&place) else {
                        continue;
                    };
                    if !racy_cells.contains(&cell) {
                        continue;
                    }
                    // Presentation follows the exact worker-cell match.
                    let name = crate::naming::qualify(&place.ns, &place.name);
                    let span = fu.abs_span(statement.span());
                    if span.is_empty() || !emitted.insert((span.start(), name.clone())) {
                        continue;
                    }
                    self.result.diagnostics.push(crate::analyser::types::Diagnostic::new(
                        DiagCode::Irule4005, span,
                        format!("Persistent state: '{name}' is written outside RULE_INIT and read in \
                                 another event. Connections on this TMM share the value; updates \
                                 are not propagated to other TMMs."),
                        Severity::Warning,
                    ));
                }
            }
        }
    }
}

/// Domain diagnostics retain physical mutation and how an operand selected its
/// namespace. An implicit current-frame destruction creates no global value.
struct IrulesCellAccess {
    place: crate::place::Place,
    import: Option<String>,
    implicit_destruction: bool,
}

/// Value writes come from the shared place bridge. Binding declarations use
/// the shared successful-continuation context and registry alias transition.
fn irules_cell_accesses(
    statement: &crate::ir::Statement,
    before: &crate::var_resolve::ResolveContext,
    after: &crate::var_resolve::ResolveContext,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<IrulesCellAccess> {
    let registry = context.commands();
    let mut accesses: Vec<_> = crate::place_bridge::statement_mutation_places_with_continuation(
        statement, before, after, registry,
    )
    .into_iter()
    .map(|place| IrulesCellAccess {
        place,
        import: None,
        implicit_destruction: false,
    })
    .collect();
    let crate::ir::Statement::Call {
        command,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        return accesses;
    };

    let Some(normal) =
        crate::registry_invocation::normal_transfer_invocation_in_context(context, tokens)
    else {
        return accesses;
    };
    if normal
        .variable_traits()
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
    {
        for (index, role) in normal.variable_roles() {
            if role != tcl_registry::ArgRole::VarWrite {
                continue;
            }
            let Some(name) = normal.argument_literal(index) else {
                continue;
            };
            if crate::var_resolve::literal_namespace_access_origin(
                &name,
                before,
                registry,
                tcl_registry::TraceOperation::Unset,
            ) != crate::var_resolve::NamespaceAccessOrigin::CurrentFrame
            {
                continue;
            }
            let target = crate::var_resolve::resolve_literal_access(
                &name,
                before,
                false,
                registry,
                tcl_registry::TraceOperation::Unset,
            );
            for access in &mut accesses {
                if crate::var_resolve::cell_key(&access.place)
                    == crate::var_resolve::cell_key(&target)
                {
                    access.implicit_destruction = true;
                }
            }
        }
    }
    for alias in normal.variable_alias_transitions() {
        if alias.writes_value {
            continue;
        }
        let Some(local) = alias.local.literal() else {
            continue;
        };
        let place = crate::var_resolve::resolve_place(local, after, false, registry);
        let import = format!("{command} {local}");
        accesses.push(IrulesCellAccess {
            place,
            import: Some(import),
            implicit_destruction: false,
        });
    }
    accesses
}

/// Uncertain contents addresses retain their proved namespace-only domain.
/// This diagnostic evidence supplies neither a physical cell nor a value definition.
fn irules_possible_namespace_writes(
    statement: &crate::ir::Statement,
    before: &crate::var_resolve::ResolveContext,
    after: &crate::var_resolve::ResolveContext,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    let registry = context.commands();
    let Some(tokens) = statement.tokens() else {
        return Vec::new();
    };

    let Some(normal) =
        crate::registry_invocation::normal_transfer_invocation_in_context(context, tokens)
    else {
        return irules_possible_handler_namespace_writes(tokens, before, after, context);
    };
    let state = match normal.variable_binding_phase() {
        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments => before,
        tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation => after,
        tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol => {
            return Vec::new();
        }
    };
    normal
        .definition_names()
        .into_iter()
        .filter(|name| {
            crate::var_resolve::resolve_place(name, state, false, registry).kind
                == crate::place::PlaceKind::Unknown
                && crate::var_resolve::possible_access_domain(name, state, registry)
                    == crate::var_resolve::PossibleVariableAccessDomain::NamespaceOnly
        })
        .collect()
}

/// Candidate output lookup can remain opaque (for example a channel callback)
/// while a namespace-only destination hazard is still possible. This query
/// creates neither a normal store nor a physical cell identity.
fn irules_possible_handler_namespace_writes(
    tokens: &crate::ir::CommandTokens,
    before: &crate::var_resolve::ResolveContext,
    after: &crate::var_resolve::ResolveContext,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    let registry = context.commands();
    let Some(possible) =
        crate::registry_invocation::possible_variable_name_operands_in_context(context, tokens)
    else {
        return Vec::new();
    };
    possible
        .phased_operands()
        .filter_map(|(role, word, phase, destroys)| {
            if role != tcl_registry::ArgRole::VarWrite {
                return None;
            }
            let name = word.as_registry_word().literal()?;
            let context = match phase {
                tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments => before,
                tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation => after,
                tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol => return None,
            };
            if destroys
                && crate::var_resolve::literal_namespace_access_origin(
                    name,
                    context,
                    registry,
                    tcl_registry::TraceOperation::Unset,
                ) == crate::var_resolve::NamespaceAccessOrigin::CurrentFrame
            {
                return None;
            }
            let place = crate::var_resolve::resolve_place(name, context, false, registry);
            if place.is_global()
                && tcl_registry::f5::namespace_storage_domain(
                    tcl_registry::f5::BigIpExecutionContext::TmmIRule,
                    &place.ns,
                ) == tcl_registry::f5::VariableStorageDomain::WorkerNamespace
            {
                return None;
            }
            (crate::var_resolve::possible_access_domain(name, context, registry)
                == crate::var_resolve::PossibleVariableAccessDomain::NamespaceOnly)
                .then(|| name.to_owned())
        })
        .collect()
}

/// Collect the bracketed text of every `[…]` command-substitution node in
/// an `expr` AST (recursing operands but stopping at the substitution
/// boundary). Used to recover
/// variable reads hidden inside `if`/`while` conditions and `expr` values.
#[cfg(test)]
fn collect_expr_command_texts(node: &ExprNode, out: &mut Vec<String>) {
    // Entry point: the top of an expression tree is nesting depth 0 (the
    // recursion cap lives in [`collect_expr_command_texts_at`]).
    collect_expr_command_texts_at(node, out, 0);
}

#[cfg(test)]
fn collect_expr_command_texts_at(node: &ExprNode, out: &mut Vec<String>, depth: u32) {
    // Native-stack safety net: walks the `ExprNode` tree, one native frame
    // per level. Past the cap, stop descending — a collector
    // that returns the command texts gathered so far is the safe fallback
    // (substitutions buried deeper than the cap are not collected; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, .. } => out.push(text.clone()),
        ExprNode::Binary { left, right, .. } => {
            collect_expr_command_texts_at(left, out, depth + 1);
            collect_expr_command_texts_at(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => collect_expr_command_texts_at(operand, out, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_command_texts_at(condition, out, depth + 1);
            collect_expr_command_texts_at(true_branch, out, depth + 1);
            collect_expr_command_texts_at(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_command_texts_at(arg, out, depth + 1);
            }
        }
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => {}
    }
}

/// Find every IPv6 *candidate* substring — `\b[hex]{1,4}(:[hex]{0,4}){2,7}\b`
/// — in `text` (the caller validates each via `Ipv6Addr::from_str`).
/// Replaces the regex; each candidate begins at a word boundary, has a
/// 1-4 hex-digit first group, 2-7 following `:`-groups (each 0-4 hex),
/// and ends on a hex digit at a trailing word boundary.
pub(super) fn find_ipv6_candidates(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let boundary_before = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary_before
            && bytes[i].is_ascii_hexdigit()
            && let Some(end) = match_ipv6_candidate(bytes, i)
        {
            out.push(&text[i..end]);
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// Read up to `max` contiguous hex-digit bytes from `start`, returning
/// the count.
fn hex_run_len(bytes: &[u8], start: usize, max: usize) -> usize {
    let mut k = 0;
    while k < max && start + k < bytes.len() && bytes[start + k].is_ascii_hexdigit() {
        k += 1;
    }
    k
}

/// Match an IPv6 candidate starting at `start`, returning the end offset
/// of the longest `hex(:hex?){2,7}` run that ends on a hex digit and is
/// followed by a word boundary, or `None`.
fn match_ipv6_candidate(bytes: &[u8], start: usize) -> Option<usize> {
    let first = hex_run_len(bytes, start, 4);
    if first == 0 {
        return None;
    }
    let mut pos = start + first;
    let mut groups = 0usize;
    let mut best: Option<usize> = None;
    while groups < 7 && bytes.get(pos) == Some(&b':') {
        let after_colon = pos + 1;
        let h = hex_run_len(bytes, after_colon, 4);
        pos = after_colon + h;
        groups += 1;
        // A valid `\b`-terminated end: ≥2 groups, ends on a hex digit,
        // and is followed by a non-word byte (or end of input).
        if groups >= 2 && h >= 1 && (pos >= bytes.len() || !is_word_byte(bytes[pos])) {
            best = Some(pos);
        }
    }
    best
}

/// The suggestion name for an undefined-variable "; did you mean 'X'?"
/// suffix (W210): a case-insensitive twin among `defined_vars` wins at
/// any edit distance ([`find_case_mismatch`] — the established W210/W211/
/// W220 behaviour), otherwise the closest *other* defined name within
/// the length-scaled edit budget ([`crate::text::scaled_max_distance`],
/// so a short typo can't fish an unrelated short name). `None` when
/// nothing is close — the message then stays suffix-free.
fn undefined_var_suggestion<'a>(
    variable: &str,
    defined_vars: &'a HashSet<String>,
) -> Option<&'a str> {
    if let Some(similar) = find_case_mismatch(variable, defined_vars) {
        return Some(similar);
    }
    // The read variable can itself appear in `defined_vars` when it is
    // assigned later in the function (`puts $x; set x 1`) — never
    // suggest the typo as its own correction.
    crate::text::suggest_similar(
        variable,
        defined_vars
            .iter()
            .map(String::as_str)
            .filter(|name| *name != variable),
        1,
        crate::text::scaled_max_distance_strict(variable),
    )
    .first()
    .copied()
}

/// Find a defined variable that differs from `variable` only in case.
/// Returns the lexicographically smallest other-cased variant —
/// deterministic across runs.
fn find_case_mismatch<'a>(variable: &str, defined_vars: &'a HashSet<String>) -> Option<&'a str> {
    let lower = variable.to_lowercase();
    let mut matches: Vec<&str> = defined_vars
        .iter()
        .filter(|n| n.as_str() != variable && n.to_lowercase() == lower)
        .map(String::as_str)
        .collect();
    matches.sort_unstable();
    matches.into_iter().next()
}

/// Adjacent CFG statements must also retain the exact original store interval;
/// a name-level hidden read elsewhere cannot observe an overwritten version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverwriteDiagnostic {
    Closed,
    Conditional,
}

fn original_overwrite_advice(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    variable: &str,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OverwriteDiagnostic> {
    let block = fu.cfg.block_by_name(&definition.block)?;
    let Ok(index) = usize::try_from(definition.statement_index) else {
        return None;
    };
    let block_id = fu.cfg.block_id(&definition.block)?;
    block.statements.get(index + 1)?;
    let first =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, index).source_tokens()?;
    let next =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block_id, index + 1).source_tokens()?;
    if crate::registry_invocation::overwritten_local_store_advice(registry, first, next)
        .is_some_and(|advice| advice.name() == variable && advice.owns(first, next))
    {
        return Some(OverwriteDiagnostic::Closed);
    }
    crate::registry_invocation::conditional_overwritten_local_store_advice(registry, first, next)
        .filter(|advice| advice.name() == variable && advice.owns(first, next))
        .map(|_| OverwriteDiagnostic::Conditional)
}

fn function_declaration_flow(
    fu: &crate::compilation_unit::FunctionUnit,
    procedure: &crate::ir::Procedure,
    image: &tcl_lexer::SourceImage,
    registry: &tcl_registry::CommandRegistry,
) -> Option<std::sync::Arc<crate::command_binding::DeclarationFlowReport>> {
    fu.ssa.blocks.iter().find_map(|(&block, data)| {
        (0..data.statements.len())
            .chain(std::iter::once(usize::MAX))
            .find_map(|index| {
                let tokens = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                    .source_tokens()?;
                let report = tokens
                    .source_binding
                    .as_ref()?
                    .declaration_flow_report(registry)?;
                report
                    .owns_original_procedure(procedure, image)
                    .then_some(report)
            })
    })
}

/// A declared alias or a defined original diagnostic path can suppress a
/// local-absence warning. Known physical unset versions still report; runtime
/// zero-trip/callback alternatives and all variable/SSA state remain unchanged.
fn suppress_declared_read_warnings(
    fu: &crate::compilation_unit::FunctionUnit,
    ctx: &ReadBeforeSetCtx<'_>,
    report: Option<&crate::command_binding::DeclarationFlowReport>,
    reads: &mut std::collections::HashMap<String, tcl_lexer::Span>,
) {
    let Some(report) = report else {
        return;
    };
    reads.retain(|name, span| {
        !(report.has_declared_alias(name) || report.declared_read_is_defined(name, *span))
            || super::helpers::original_read_occurrence_at_span(fu, *span)
                .is_some_and(|value| ctx.supp.killed.contains(&value))
    });
}

/// Original uses missing from SSA only suppress store diagnostics. The query
/// changes neither physical liveness nor store-removal permission.
fn original_unrepresented_use_advice(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    variable: &str,
    registry: &tcl_registry::CommandRegistry,
    layout: &std::cell::OnceCell<
        Option<std::sync::Arc<crate::command_binding::DeclarationFlowReport>>,
    >,
) -> bool {
    let Some(block) = fu.cfg.block_id(&definition.block) else {
        return false;
    };
    let Ok(index) = usize::try_from(definition.statement_index) else {
        return false;
    };
    let Some(tokens) =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
    else {
        return false;
    };
    let Some(report) = layout.get_or_init(|| {
        tokens
            .source_binding
            .as_ref()?
            .declaration_flow_report(registry)
    }) else {
        return false;
    };
    if !tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
        .is_some_and(|site| report.owns_invocation(site))
    {
        return false;
    }
    if report.has_declared_alias(variable)
        || report.has_named_use(variable)
        || report.has_quoted_use(variable)
    {
        return true;
    }
    report.authored_use_spans(variable).iter().any(|span| {
        !fu.def_use
            .chains
            .values()
            .filter(|chain| {
                fu.ssa
                    .cell_symbol(&chain.key.0)
                    .is_some_and(|symbol| fu.ssa.var_name(symbol) == variable)
            })
            .flat_map(|chain| &chain.uses)
            .any(|usage| {
                let Some(block) = fu.cfg.block_id(&usage.block) else {
                    return false;
                };
                let index = usize::try_from(usage.statement_index).unwrap_or(usize::MAX);
                crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                    .source_tokens()
                    .is_some_and(|tokens| {
                        tokens.words().iter().any(|word| {
                            word.source().span.start() <= span.start()
                                && span.end() <= word.source().span.end()
                        })
                    })
            })
    })
}

fn original_unread_store_advice(
    fu: &crate::compilation_unit::FunctionUnit,
    definition: &crate::def_use::DefSite,
    variable: &str,
    registry: &tcl_registry::CommandRegistry,
    layout: &std::cell::OnceCell<
        Option<std::sync::Arc<crate::command_binding::DeclarationFlowReport>>,
    >,
) -> bool {
    let Some(block) = fu.cfg.block_id(&definition.block) else {
        return false;
    };
    let Ok(index) = usize::try_from(definition.statement_index) else {
        return false;
    };
    let Some(tokens) =
        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()
    else {
        return false;
    };
    let report = layout.get_or_init(|| {
        tokens
            .source_binding
            .as_ref()?
            .declaration_flow_report(registry)
    });
    let Some(report) = report else {
        return false;
    };
    crate::registry_invocation::conditional_unread_local_store_advice(registry, tokens, report)
        .is_some_and(|advice| advice.name() == variable && advice.owns(tokens))
}

#[cfg(test)]
pub(crate) fn report_original_store_diagnostic_gates(
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
) {
    for chain in fu.def_use.chains.values() {
        if chain.definition.kind != crate::def_use::DefKind::Statement {
            continue;
        }
        let Some(symbol) = fu.ssa.cell_symbol(&chain.key.0) else {
            continue;
        };
        let variable = fu.ssa.var_name(symbol);
        let Some(block) = fu.cfg.block_id(&chain.definition.block) else {
            continue;
        };
        let Ok(index) = usize::try_from(chain.definition.statement_index) else {
            continue;
        };
        let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index);
        let statement = &fu.ssa.blocks[&block].statements[index].statement;
        let tokens = view.source_tokens();
        eprintln!(
            "store diagnostic key={:?} dead={} synthetic={} reportable={} overwrite={:?} source={:?} declaration={} normal={}",
            chain.key,
            chain.is_dead(),
            fu.ssa.is_synthetic_def(
                &chain.definition.block,
                chain.definition.statement_index,
                &chain.key.0
            ),
            reportable_dead_assignment(
                statement,
                registry,
                fu.invocation_metadata_context(registry)
            ),
            original_overwrite_advice(fu, &chain.definition, variable, registry),
            tokens.map(|tokens| &tokens.argv_texts),
            tokens
                .and_then(|tokens| tokens
                    .source_binding
                    .as_ref()?
                    .declaration_operand_layout_advice(tokens))
                .is_some(),
            tokens
                .and_then(|tokens| {
                    crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                        registry,
                        fu.invocation_metadata_context(registry),
                        tokens,
                    )
                })
                .is_some(),
        );
        eprintln!(
            "store remaining gates dynamic_reads={} executable={} source_kind={:?}",
            fu.dynamic_names.reads,
            fu.diagnostic_value_facts()
                .executable_blocks()
                .contains(&block),
            statement,
        );
    }
}

/// Synthetic effect statements have no original argv for a diagnostic anchor.
fn statement_is_synthetic_effect(stmt: &crate::ir::Statement) -> bool {
    match stmt {
        crate::ir::Statement::Call { tokens, .. }
        | crate::ir::Statement::Barrier { tokens, .. } => tokens
            .as_ref()
            .is_some_and(|tokens| tokens.synthetic.is_some()),
        _ => false,
    }
}

/// Suppress a lexical body read when unchanged conditional source advice
/// owns the name in its own frame. This proves no executed store or successor
/// value. Opaque foreign bodies retain their separate lexical suppression.
fn barrier_body_locally_sets(
    stmt: Option<&crate::ir::Statement>,
    var: &str,
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    context: &tcl_registry::model::ContextRegistry,
    original_tokens: Option<&crate::ir::CommandTokens>,
) -> bool {
    let registry = context.commands();
    let Some(config) = analysis.body_lexer_config else {
        return false;
    };
    let Some(tokens) = original_tokens else {
        return false;
    };
    if crate::registry_invocation::source_structure::original_segment_for_tokens(
        source, analysis, tokens,
    )
    .is_none()
    {
        return false;
    }
    if !matches!(stmt, Some(crate::ir::Statement::Barrier { .. })) {
        return false;
    }
    let Some(words) =
        crate::registry_invocation::source_structure::original_registry_words_for_tokens(
            source, analysis, tokens,
        )
    else {
        return false;
    };
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    let metadata = crate::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        Some(input.unit_profile()),
    );
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    let Some(footprint) =
        binding.original_materialized_footprint(tokens, source, registry, metadata)
    else {
        return false;
    };
    // Possible local ownership uses the selected installer's source world.
    // Ordinary interpolation supplies no binding; an unavailable child table
    // cannot borrow this installer's command descriptors or aliases.
    let root = crate::naming::split_element_ref(var).map_or(var, |(root, _)| root);
    words
        .source_script_bodies_for(
            context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        )
        .iter()
        .any(|body| {
            let names = footprint.body_name_ownership(body);
            names
                .names
                .iter()
                .chain(&names.read_names)
                .any(|name| name == root)
        })
}

/// Variables this statement queries *only for
/// existence* (`info exists X` / `array exists X`, whether a bare call
/// or a `[...]` command substitution inside an assignment / argument).
/// Such a reference is not a value read, so it must not raise W210.
fn existence_query_cells(
    stmt: &crate::ir::Statement,
    registry: &tcl_registry::CommandRegistry,
    config: tcl_lexer::LexerConfig,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Vec<crate::var_resolve::VariableCellKey> {
    let Some(tokens) = stmt.tokens() else {
        return Vec::new();
    };
    let mut queries = Vec::new();
    if let Some(query) = crate::existence_query::in_tokens_for_diagnostics_with_metadata_context(
        tokens, registry, metadata,
    ) {
        queries.push(query);
    }
    for call in crate::word_subst::checked_lifted_calls(tokens, config).unwrap_or_default() {
        if let Some(nested) = call.tokens.as_ref()
            && let Some(query) =
                crate::existence_query::in_tokens_for_diagnostics_with_metadata_context(
                    nested, registry, metadata,
                )
        {
            queries.push(query);
        }
    }
    queries
        .into_iter()
        .filter_map(|(query, context)| {
            let place =
                crate::var_resolve::resolve_literal_place(&query.var, &context, false, registry);
            crate::var_resolve::canonical_place_key(&place)
        })
        .collect()
}

/// True when a read of `var` at `use_block` is exempt
/// from W210 because it is the existence-query word itself, or because
/// it sits in a region guarded by an enclosing `[info exists var]`.
fn existence_exempt(
    stmt_opt: Option<&crate::ir::Statement>,
    cell: &crate::var_resolve::VariableCellKey,
    exists_guards: &[super::helpers::ExistenceGuard],
    fu: &crate::compilation_unit::FunctionUnit,
    use_site: &crate::def_use::UseSite,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let ssa = &fu.ssa;
    let Some(use_id) = ssa.block_id(&use_site.block) else {
        return false;
    };
    let index = usize::try_from(use_site.statement_index).unwrap_or(usize::MAX);
    if use_site.kind == crate::def_use::UseKind::VariableName
        && let Some(stmt) = stmt_opt
        && existence_query_cells(
            stmt,
            registry,
            fu.source_lexer_config(),
            fu.invocation_metadata_context(registry),
        )
        .iter()
        .any(|query| query == cell)
    {
        return true;
    }
    let original_cells =
        super::helpers::original_read_cells_at(ssa, (use_id, index), cell, registry);
    exists_guards.iter().any(|(guarded, block)| {
        (original_cells.contains(guarded) || ssa.point_contexts.is_none() && guarded == cell)
            && block_dominated_by(ssa, use_id, *block)
    })
}

/// True when a read of `var` at this use-site statement is in fact a safe
/// self-initialisation, not a read-before-set: a `safe_on_uninit` call (e.g.
/// `lappend`/`dict set`/`append`) that defines `var`, or an `incr` of its own
/// target (which initialises an unset var to 0 in Tcl 8.5+).
fn use_site_safe_initialises(
    stmt: Option<&crate::ir::Statement>,
    fu: &crate::compilation_unit::FunctionUnit,
    use_site: &crate::def_use::UseSite,
    cell: &crate::var_resolve::VariableCellKey,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let safe = match stmt {
        Some(
            crate::ir::Statement::Call { safe_on_uninit, .. }
            | crate::ir::Statement::Incr { safe_on_uninit, .. },
        ) => *safe_on_uninit,
        _ => false,
    };
    if !safe || use_site.kind != crate::def_use::UseKind::VariableName {
        return false;
    }
    let Some(block) = fu.cfg.block_id(&use_site.block) else {
        return false;
    };
    let Ok(index) = usize::try_from(use_site.statement_index) else {
        return false;
    };
    super::helpers::original_definition_places(fu, block, index, registry)
        .iter()
        .any(|place| crate::var_resolve::canonical_place_key(place).as_ref() == Some(cell))
}

/// The namespace of a fully-qualified name: everything up to the last `::`,
/// or `::` for a top-level name.
fn namespace_of(qualified_name: &str) -> String {
    match qualified_name.rsplit_once("::") {
        Some((ns, _)) if !ns.is_empty() => ns.to_string(),
        _ => "::".to_string(),
    }
}

/// Compute W213's diagnostic span and quick fix for an `unset` of a
/// possibly-missing `var`.
///
/// The span narrows to the offending variable's own word (so `unset a b c`
/// squiggles just the missing name), falling back to the whole-command `span`
/// when the tokens aren't available or the name can't be located. The fix
/// inserts ` -nocomplain` immediately after the `unset` command word — a
/// zero-width insertion — turning `unset x` into `unset -nocomplain x`.
fn w213_span_and_fix(
    fu: &crate::compilation_unit::FunctionUnit,
    tokens: Option<&crate::ir::CommandTokens>,
    var: &str,
    span: tcl_lexer::Span,
) -> (tcl_lexer::Span, Vec<super::types::CodeFix>) {
    let Some(toks) = tokens else {
        return (span, Vec::new());
    };
    // Narrow to the argument word whose text is this variable (argv[0] is the
    // `unset` command word, so the names start at index 1).
    let diag_span = toks
        .argv_texts
        .iter()
        .zip(&toks.argv)
        .skip(1)
        .find(|(text, _)| text.as_str() == var)
        .map_or(span, |(_, &word)| fu.abs_span(word));
    // Insert ` -nocomplain` right after the `unset` word.
    let fixes = toks.argv.first().map_or_else(Vec::new, |&cmd_word| {
        let at = fu.abs_span(cmd_word).end();
        vec![super::types::CodeFix {
            span: tcl_lexer::Span::new(at, at),
            new_text: " -nocomplain".to_string(),
            description: "Add '-nocomplain' to unset".to_string(),
            // W213: `-nocomplain` stops `unset` raising on a missing variable.
            // Suppressing that error is the point of the fix, and a program
            // relying on it (a `catch`ed probe) observes the change.
            safety: crate::irules_checks::FixSafety::BehaviourHardening,
        }]
    });
    (diag_span, fixes)
}

/// Tcl ARE metacharacters: a pattern free of these reduces to a literal
/// substring search.
const TCL_REGEX_METACHARS: &str = r"\^$.|?*+()[]{}";

/// `regexp` switches that don't change match-vs-no-match for a pure-literal
/// pattern.
fn is_regexp_literal_safe_switch(opt: &str) -> bool {
    matches!(
        opt,
        "-indices" | "-inline" | "-all" | "-line" | "-lineanchor" | "-linestop" | "-start" | "--"
    )
    // `-expanded` is handled separately (whitespace/comment-gated) by the
    // caller, so it is intentionally not listed here.
}

/// True iff `regexp PATTERN INPUT` provably returns 0.  Sound only when
/// `pat` is a pure-literal pattern (no ARE metacharacters), reducing the
/// match to substring search.  Unknown / unsafe switches bail (return
/// `false` = cannot prove no-match).
fn regexp_literal_no_match(pat: &str, inp: &str, options: &[String]) -> bool {
    if pat.chars().any(|c| TCL_REGEX_METACHARS.contains(c)) {
        return false;
    }
    let mut nocase = false;
    let mut expanded = false;
    for opt in options {
        if !opt.starts_with('-') {
            continue; // an option value (e.g. after `-start`)
        }
        if opt == "-nocase" {
            nocase = true;
            continue;
        }
        if opt == "-expanded" {
            expanded = true;
            continue;
        }
        if is_regexp_literal_safe_switch(opt) {
            continue;
        }
        return false; // unknown / unsafe switch
    }
    // `-expanded` makes Tcl ignore unescaped whitespace and `#`-comments in
    // the pattern, so a pattern containing either is NOT a plain substring
    // (`regexp -expanded {a b} {ab}` matches).  Bail in that case so the
    // no-match proof stays sound — a whitespace/comment-free literal is
    // still safe.
    if expanded && pat.chars().any(|c| c.is_whitespace() || c == '#') {
        return false;
    }
    if nocase {
        !inp.to_lowercase().contains(&pat.to_lowercase())
    } else {
        !inp.contains(pat)
    }
}

/// One unchanged output in the represented source-value projection.
struct NoMatchOutputOrigin {
    dominator: crate::cfg::BlockId,
    index: usize,
    version: crate::ssa::Version,
    branch_entry: bool,
}
impl NoMatchOutputOrigin {
    fn reaches(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        block: crate::cfg::BlockId,
        index: usize,
    ) -> bool {
        if block == self.dominator {
            self.branch_entry || index > self.index
        } else {
            block_dominated_by(&fu.ssa, block, self.dominator)
        }
    }
}
struct NoMatchCallSite {
    producer: crate::cfg::BlockId,
    index: usize,
    dominator: crate::cfg::BlockId,
    branch_entry: bool,
}

fn record_no_match_outputs(
    fu: &crate::compilation_unit::FunctionUnit,
    site: NoMatchCallSite,
    outputs: &[String],
    logical: bool,
    origins: &mut std::collections::HashMap<crate::ssa::Symbol, Vec<NoMatchOutputOrigin>>,
) {
    let Some(data) = fu.ssa.blocks.get(&site.producer) else {
        return;
    };
    let point = if site.index == data.statements.len() {
        usize::MAX
    } else {
        site.index
    };
    let view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, site.producer, point);
    for name in outputs {
        let symbol = view.symbol(name).or_else(|| {
            logical
                .then(|| {
                    fu.ssa
                        .cell_symbol(&crate::var_resolve::VariableCellKey::Authored(name.clone()))
                })
                .flatten()
        });
        let Some(symbol) = symbol else {
            continue;
        };
        let before = data.statements[..site.index]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, statement)| {
                fu.ssa
                    .value_clobbers
                    .get(&site.producer)
                    .and_then(|markers| markers.get(&index))
                    .and_then(|versions| versions.get(&symbol))
                    .map(|&(_, fresh)| fresh)
                    .or_else(|| statement.defs.get(&symbol).copied())
            })
            .or_else(|| data.entry_versions.get(&symbol).copied())
            .unwrap_or(0);
        // Failure retains a preceding value, and any later version is separate.
        if before != 0 {
            continue;
        }
        let version = data
            .statements
            .get(site.index)
            .and_then(|statement| statement.defs.get(&symbol))
            .copied()
            .unwrap_or(before);
        origins
            .entry(symbol)
            .or_default()
            .push(NoMatchOutputOrigin {
                dominator: site.dominator,
                index: site.index,
                version,
                branch_entry: site.branch_entry,
            });
    }
}

fn original_missing_matcher_read_span(
    fu: &crate::compilation_unit::FunctionUnit,
    view: crate::ssa::SsaSourceView<'_>,
    symbol: crate::ssa::Symbol,
    registry: &tcl_registry::CommandRegistry,
) -> Option<tcl_lexer::Span> {
    view.source_tokens()?
        .variable_accesses
        .iter()
        .filter_map(|access| {
            let read = view.read_reference(&access.source, &access.original_spelling)?;
            if read.symbol != symbol
                || !view
                    .read_contents_presence_alternatives_at(
                        &access.source,
                        &access.original_spelling,
                        registry,
                    )
                    .is_some_and(crate::ssa::SsaReadPresenceAlternatives::may_be_undefined)
                || super::helpers::original_read_is_live_array_scalar(access, registry)
            {
                return None;
            }
            Some(fu.abs_span(access.source.span))
        })
        .min_by_key(|span| span.start())
}

/// Complete selected source matcher layout and decoded effective literal argv.
/// A handler descriptor identifies the authored protocol; it never proves an
/// installed handler, physical target, normal completion or current absence.
fn selected_matcher_no_match_outputs(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    use tcl_registry::variable_output::NativeVariableOutputSpec;
    // naming.diagnostics.original-matcher-output-retention
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
    let Some(layout) = SourceMatcherLayout::from_original(words, context) else {
        return Vec::new();
    };
    let pattern = layout.arguments[layout.pattern];
    let input = layout.arguments[layout.input];
    if !pattern.is_ascii()
        || !input.is_ascii()
        || pattern.contains('\0')
        || input.contains('\0')
        || pattern.len() > 1024
        || input.len() > 8192
    {
        return Vec::new();
    }
    let no_match = match layout.protocol {
        NativeVariableOutputSpec::Regexp => {
            let options = layout.arguments[..layout.pattern]
                .iter()
                .map(|option| (*option).to_owned())
                .collect::<Vec<_>>();
            !options.iter().any(|option| option == "-start")
                && regexp_literal_no_match(pattern, input, &options)
        }
        NativeVariableOutputSpec::Scan => {
            scan_literal_no_match(pattern, input, layout.outputs.len())
        }
    };
    if !no_match {
        return Vec::new();
    }
    layout
        .outputs
        .into_iter()
        .filter_map(|index| {
            let name = *layout.arguments.get(index)?;
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

struct SourceMatcherLayout<'a> {
    protocol: tcl_registry::variable_output::NativeVariableOutputSpec,
    arguments: Vec<&'a str>,
    outputs: Vec<usize>,
    pattern: usize,
    input: usize,
}
impl<'a> SourceMatcherLayout<'a> {
    fn from_original(
        words: &'a crate::registry_invocation::source_structure::OriginalRegistryWords,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<Self> {
        use tcl_registry::{
            ArgRole, native_compilation::SuccessfulHandlerSpec,
            variable_output::NativeVariableOutputSpec,
        };
        let facts = words.with_source_schema(context, |invocation| invocation.facts())?;
        let SuccessfulHandlerSpec::ConditionalVariableOperands(protocol) =
            facts.successful_handler?
        else {
            return None;
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        let arguments = words
            .arguments()
            .iter()
            .map(|word| std::str::from_utf8(word.literal_bytes()?).ok())
            .collect::<Option<Vec<_>>>()?;
        if facts.frozen_argument_count != Some(arguments.len()) {
            return None;
        }
        let indices = |role| {
            facts.arg_roles.iter().filter_map(move |&(index, found)| {
                (role == found).then_some(facts.argument_offset + usize::from(index))
            })
        };
        let outputs = indices(ArgRole::VarWrite).collect::<Vec<_>>();
        if outputs.is_empty() {
            return None;
        }
        let (pattern, input) = match protocol {
            NativeVariableOutputSpec::Regexp => {
                let pattern = indices(ArgRole::Pattern).next()?;
                (pattern, pattern.checked_add(1)?)
            }
            NativeVariableOutputSpec::Scan => {
                let pattern = indices(ArgRole::ScanFormat).next()?;
                (pattern, pattern.checked_sub(1)?)
            }
        };
        arguments.get(pattern)?;
        arguments.get(input)?;
        Some(Self {
            protocol,
            arguments,
            outputs,
            pattern,
            input,
        })
    }
}

/// The shared parser validates the complete dialect-common plain conversion
/// subset before the existing first-conversion no-match kernel is consulted.
fn scan_literal_no_match(format: &str, input: &str, output_count: usize) -> bool {
    let chars = format.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut conversions = 0;
    while index < chars.len() {
        if chars[index] != '%' {
            index += 1;
            continue;
        }
        index += 1;
        let Ok(conversion) = tcl_syntax::scan::parse_conversion(&chars, &mut index) else {
            return false;
        };
        if conversion.suppress
            || conversion.width.is_some()
            || conversion.size.is_some()
            || conversion.xpg_index.is_some()
            || !matches!(conversion.verb, '%' | 'd' | 'o' | 'x' | 'X' | 's' | 'c')
        {
            return false;
        }
        conversions += usize::from(conversion.verb != '%');
    }
    conversions == output_count && crate::scan_predicate::scan_provably_no_match(format, input)
}

/// Return ``true`` when ``body`` contains a ``$param`` /
/// ``${param}`` substitution.  Used as a fallback by the W214
/// (unused-parameter) emitter to suppress the warning when the
/// parameter is read inside a ``[expr {...}]`` / ``[cmd ...]``
/// substitution that the IR lowerer doesn't track as a use.
///
/// Conservative — false negatives are fine (W214 still fires
/// when the param genuinely isn't referenced), but false
/// positives would cause the over-emit this guard exists to
/// prevent.  The bare-name match enforces a non-identifier
/// boundary on each side so ``$abc`` doesn't match ``$ab``,
/// and skips the variable when it follows a ``\\`` escape.
/// True when the proc body textually references the parameter `$param` /
/// `${param}`, scanning command-by-command so a `namespace eval` body — which
/// runs in the *namespace* frame, not the caller's — does **not** falsely
/// recover a read of the caller's parameter.  Other bodies (`eval`, `if`,
/// loops) run in the caller frame, so their `$param` reads still count.
pub(super) fn body_references_param(
    body: &str,
    param: &str,
    config: tcl_lexer::LexerConfig,
) -> bool {
    if param.is_empty() {
        return false;
    }
    let cmds = crate::segmenter::segment_commands_with_offset_and_config(body, 0, config);
    for cmd in &cmds {
        // `namespace eval NS BODY` — the trailing body word evaluates in NS's
        // frame, so exclude it; the NS-name word (e.g. `namespace eval $x …`)
        // is still substituted in the caller frame and is scanned.
        let is_ns_eval = cmd.texts.first().map(String::as_str) == Some("namespace")
            && cmd.texts.get(1).map(String::as_str) == Some("eval");
        let skip_last = is_ns_eval && cmd.texts.len() >= 4;
        let last_idx = cmd.texts.len().saturating_sub(1);
        for (i, word) in cmd.texts.iter().enumerate() {
            if skip_last && i == last_idx {
                continue;
            }
            if word_references_param(word, param) {
                return true;
            }
        }
    }
    false
}

/// True when a single word textually references `$param` / `${param}`.  Flat
/// byte scan with identifier-boundary and `\$` escape handling.
fn word_references_param(body: &str, param: &str) -> bool {
    if param.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let plen = param.len();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c != b'$' {
            i += 1;
            continue;
        }
        // Skip backslash-escaped ``\$``.
        if i > 0 && bytes[i - 1] == b'\\' {
            i += 1;
            continue;
        }
        // ``${name}`` form.
        if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            let start = i + 2;
            if start + plen <= bytes.len()
                && &bytes[start..start + plen] == param.as_bytes()
                && start + plen < bytes.len()
                && bytes[start + plen] == b'}'
            {
                return true;
            }
        } else {
            // ``$name`` form — bare identifier match.
            let start = i + 1;
            if start + plen <= bytes.len() && &bytes[start..start + plen] == param.as_bytes() {
                let after = start + plen;
                let next_ok = after >= bytes.len() || !is_ident_continue(bytes[after]);
                if next_ok {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Must-policy over a channel argument's type union: `Some(label)` when
/// EVERY member is a non-channel — whatever path ran, the value cannot be a
/// channel — with the members rendered `"INT | STRING"` for the message. A
/// union with any Channel member, or an Unknown / Overdefined node, returns
/// `None`: some path may be fine.
fn non_channel_union_label(var_type: &crate::types::TypeLattice) -> Option<String> {
    use crate::types::{TypeKind, TypeShape};
    if !matches!(var_type.kind(), TypeKind::Known | TypeKind::Shimmered) {
        return None;
    }
    let member_types: Vec<tcl_registry::TclType> =
        var_type.shapes().iter().map(TypeShape::coarse).collect();
    if member_types.is_empty()
        || member_types
            .iter()
            .any(|t| matches!(t, tcl_registry::TclType::Channel))
    {
        return None;
    }
    Some(
        member_types
            .iter()
            .map(|t| format!("{t:?}").to_uppercase())
            .collect::<Vec<_>>()
            .join(" | "),
    )
}

/// Whether the def site's statement is an array-element write
/// (`set a(k) …` / `set a($k) …` / `incr a(k)`) — the base def such a
/// write records carries no reportable liveness of its own.
fn def_is_element_write(
    fu: &crate::compilation_unit::FunctionUnit,
    def: &crate::def_use::DefSite,
) -> bool {
    use crate::ir::Statement;
    fu.cfg
        .block_by_name(&def.block)
        .and_then(|b| {
            usize::try_from(def.statement_index)
                .ok()
                .and_then(|i| b.statements.get(i))
        })
        .is_some_and(|stmt| {
            matches!(
                stmt,
                Statement::AssignConst { name, .. }
                    | Statement::AssignExpr { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. }
                    if name.contains('(')
            )
        })
}

#[cfg(test)]
mod issue996_tests {
    use super::*;

    /// Depth coverage: `collect_expr_command_texts` recurses once per
    /// `ExprNode` level, so without a depth cap it overflows the native stack
    /// (SIGABRT) in the low thousands of levels on a 2 MiB thread.  A tree
    /// built directly is unbounded (the Pratt parser caps its own output at
    /// 256); 3000 is past that crash range and past `MAX_EXPR_NODE_DEPTH`
    /// (256); the assertion is that it returns at all.
    #[test]
    fn deeply_nested_collect_expr_command_texts_survives() {
        let mut node = ExprNode::Command {
            text: "[x]".into(),
            start: 0,
            end: 3,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let mut out = Vec::new();
        collect_expr_command_texts(&node, &mut out);
    }

    #[test]
    fn opaque_body_local_set_requires_original_body_owner() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        // Conditional source suppression only; no entered interpreter or store.
        let source = "interp eval {} {set x 1; puts $x}\n";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let config = analysis.body_lexer_config.unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segments[0],
        );
        analysis
            .retained_command_realm()
            .unwrap()
            .stamp_original_tokens(&mut tokens);
        let stmt = crate::ir::Statement::Barrier {
            span: tokens.argv[0],
            reason: "original current-interpreter body".to_owned(),
            command: "interp".to_owned(),
            canonical_command: Some("::interp".to_owned()),
            args: vec![
                "eval".to_owned(),
                String::new(),
                "set x 1; puts $x".to_owned(),
            ],
            tokens: Some(tokens.clone()),
        };
        assert!(barrier_body_locally_sets(
            Some(&stmt),
            "x",
            source,
            &analysis,
            &context,
            Some(&tokens)
        ));
        assert!(!barrier_body_locally_sets(
            Some(&stmt),
            "x",
            source,
            &analysis,
            &context,
            None
        ));
        assert!(!barrier_body_locally_sets(
            Some(&stmt),
            "x",
            "interp eval {} {set y 1; puts $y}\n",
            &analysis,
            &context,
            Some(&tokens)
        ));
    }

    #[test]
    fn original_body_ownership_keeps_installer_horizon_and_child_refusal() {
        // naming.diagnostic.original-materialized-write-footprint
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
        // Conditional suppression, never an entered child or completed store.
        for (source, target, expected) in [
            ("interp eval {} {puts $missing}", "missing", false),
            ("interp eval {} {set missing}", "missing", true),
            (
                "interp alias {} put {} set {$literal}; interp eval {} {put VALUE}",
                "$literal",
                true,
            ),
            (
                "interp alias {} put {} set café; rename set {}; interp eval {} {put VALUE}",
                "café",
                false,
            ),
            (
                "interp create child; interp eval child {set x VALUE}",
                "x",
                false,
            ),
            ("interp eval {} {set {café(open} VALUE}", "café(open", true),
        ] {
            let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
            let config = analysis.body_lexer_config.unwrap();
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            let segments =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                segments.last().unwrap(),
            );
            analysis
                .retained_command_realm()
                .unwrap()
                .stamp_original_tokens(&mut tokens);
            let stmt = crate::ir::Statement::Barrier {
                span: segments.last().unwrap().span,
                reason: "conditional body".into(),
                command: segments.last().unwrap().texts[0].clone(),
                canonical_command: None,
                args: Vec::new(),
                tokens: Some(tokens.clone()),
            };
            assert_eq!(
                barrier_body_locally_sets(
                    Some(&stmt),
                    target,
                    source,
                    &analysis,
                    &context,
                    Some(&tokens)
                ),
                expected,
                "{source}"
            );
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(!barrier_body_locally_sets(
                Some(&stmt),
                target,
                source,
                &missing,
                &context,
                Some(&tokens)
            ));
            let foreign = tcl_registry::model::ingress::resolve_environment("tcl9.0")
                .default_context_registry();
            assert!(!barrier_body_locally_sets(
                Some(&stmt),
                target,
                source,
                &analysis,
                &foreign,
                Some(&tokens)
            ));
        }
    }
}

#[cfg(test)]
mod source_body_purpose_tests {
    use super::*;
    use std::sync::Arc;
    use tcl_registry::{
        ArgRole, Arity, CommandRegistry, CommandSpec, InvocationArguments, ScriptTiming,
    };

    fn reference_only(_arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
        vec![(0, ScriptTiming::ReferenceOnly)]
    }

    #[test]
    fn original_reference_only_body_cannot_supply_local_store_suppression() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        // Diagnostic suppression capability; no executed store or child entry.
        let source = "reference-script {set x 1; puts $x}\nrun-script {set x 1; puts $x}";
        let mut registry = CommandRegistry::build_default();
        for (name, timing) in [
            (
                "reference-script",
                Some(reference_only as tcl_registry::ScriptTimingResolver),
            ),
            ("run-script", None),
        ] {
            registry.insert(CommandSpec {
                name,
                arity: Arity::exact(1),
                arg_roles: &[(0, ArgRole::Body)],
                script_timing_resolver: timing,
                ..CommandSpec::DEFAULT
            });
        }
        let profile = tcl_dialect::DialectProfile::find("tcl").unwrap();
        let context = Arc::new(
            tcl_registry::model::context_for_profile(profile)
                .with_command_store(Arc::new(registry)),
        );
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl");
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        for (index, expected) in [(0, false), (1, true)] {
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                &segments[index],
            );
            analysis
                .retained_command_realm()
                .unwrap()
                .stamp_original_tokens(&mut tokens);
            let words =
                crate::registry_invocation::source_structure::original_registry_words_for_tokens(
                    source, &analysis, &tokens,
                )
                .expect("original source producer");
            assert_eq!(
                words.source_script_bodies(&context).len(),
                1,
                "both bodies remain original syntax"
            );
            let stmt = crate::ir::Statement::Barrier {
                span: tokens.argv[0],
                reason: "authored foreign body".into(),
                command: segments[index].texts[0].clone(),
                canonical_command: None,
                args: segments[index].texts[1..].to_vec(),
                tokens: Some(tokens.clone()),
            };
            assert_eq!(
                barrier_body_locally_sets(
                    Some(&stmt),
                    "x",
                    source,
                    &analysis,
                    &context,
                    Some(&tokens)
                ),
                expected
            );
        }
    }
}

#[cfg(test)]
mod original_variable_anchor_tests {
    fn anchor(source: &str, dialect: &str, target: &str) -> Option<tcl_lexer::Span> {
        let environment = tcl_registry::model::resolve_environment(dialect);
        let profile = environment.unit_profile();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analyser = crate::analyser::Analyser::new().with_resolved_input(input);
        analyser.source = source.to_owned();
        analyser.narrow_to_read_var(
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            target,
        )
    }

    #[test]
    fn variable_read_anchor_preserves_literal_name_and_complete_reference_geometry() {
        // naming.diagnostics.original-variable-name-anchor
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
        // Lexical anchor controls, not variable existence, W210 admission or
        // successful native reads. A braced scalar may contain an unmatched '('.
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            for (source, target, reference) in [
                ("puts $a ${a(b}", "a(b", "${a(b}"),
                ("puts ${literal} ${$literal}", "$literal", "${$literal}"),
                ("puts ${é}", "é", "${é}"),
                ("puts $a(k)", "a(k)", "$a(k)"),
                ("puts ${a(k)}", "a(k)", "${a(k)}"),
                ("puts ${}", "", "${}"),
                ("set marker 1; puts [list ${a(b}]", "a(b", "${a(b}"),
            ] {
                let start = u32::try_from(source.find(reference).unwrap()).unwrap();
                let expected =
                    tcl_lexer::Span::new(start, start + u32::try_from(reference.len()).unwrap());
                assert_eq!(
                    anchor(source, dialect, target),
                    Some(expected),
                    "{dialect}: {source:?}"
                );
            }
            for (source, target) in [
                ("puts $a", "a(b"),
                ("puts ${literal}", "$literal"),
                ("puts {${a(b}}", "a(b"),
                ("puts ${unclosed", "unclosed"),
                ("puts $a(", "a("),
            ] {
                assert_eq!(
                    anchor(source, dialect, target),
                    None,
                    "{dialect}: {source:?}"
                );
            }
        }
    }

    #[test]
    fn variable_read_anchor_uses_retained_braced_and_bare_name_grammar() {
        // naming.diagnostics.original-variable-name-anchor
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-variable-name-anchor.md
        // Exact source/configuration discrimination, not a native cell or
        // completion observation. Bare Unicode is Jim syntax only.
        let source = "puts ${a{b}c}";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "jim"] {
            assert_eq!(anchor(source, dialect, "a{b}c"), None);
            assert_eq!(
                anchor(source, dialect, "a{b"),
                Some(tcl_lexer::Span::new(5, 11))
            );
        }
        for dialect in ["tcl9.0", "tcl9.1"] {
            assert_eq!(
                anchor(source, dialect, "a{b}c"),
                Some(tcl_lexer::Span::new(5, 13))
            );
            assert_eq!(anchor(source, dialect, "a{b"), None);
        }
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            assert_eq!(anchor("puts $é", dialect, "é"), None);
        }
        assert_eq!(
            anchor("puts $é", "jim", "é"),
            Some(tcl_lexer::Span::new(5, 8))
        );
    }
}

#[cfg(test)]
mod original_matcher_admission_tests {
    use super::*;

    fn output_names(source: &str, dialect: &str) -> Vec<String> {
        let analysis = Analyser::new().analyse(source, dialect);
        let config = analysis.body_lexer_config.unwrap();
        let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let words = crate::registry_invocation::source_structure::source_registry_words(
            source,
            &analysis,
            commands.last().unwrap(),
        );
        words.map_or_else(Vec::new, |words| {
            selected_matcher_no_match_outputs(
                &words,
                &analysis.resolved_input.as_ref().unwrap().context_registry(),
            )
        })
    }

    #[test]
    fn selected_original_matcher_advice_keeps_alias_arguments_and_known_provider_barriers() {
        // naming.diagnostics.original-matcher-output-retention
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
        // Selected conditional source protocol and decoded original argv only;
        // none of these Native analyses grants current output cells or reads.
        for dialect in [
            "tcl", "tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim",
        ] {
            for source in [
                "regexp x y v",
                "scan abc %d v",
                "regexp x y {$v}",
                "interp alias {} matcher {} regexp x y; matcher v",
                "interp alias {} matcher {} regexp -nocase x; matcher Y v",
                "rename regexp matcher; matcher x y v",
                "interp alias {} matcher {} regexp x y; rename matcher moved; moved v",
            ] {
                let expected = if source.contains("{$v}") { "$v" } else { "v" };
                assert_eq!(
                    output_names(source, dialect),
                    vec![expected],
                    "{dialect}: {source}"
                );
            }
            for source in [
                "namespace eval app {proc regexp args {}}; ::app::regexp x y v",
                "proc regexp args {}; regexp x y v",
                "proc scan args {}; scan abc %d v",
                "interp alias {} matcher {} regexp x y; rename regexp gone; proc regexp args {}; matcher v",
                "interp alias {} matcher {} regexp -inline x y; matcher v",
                "regexp -bogus x y v",
                "regexp -about x y v",
                "regexp -nocase x X v",
                "regexp -expanded {a b} ab v",
                "regexp -start end x y v",
                "regexp {$p} y v",
                "scan abc %b v",
                "scan abc {%d %d} v",
                "scan abc %2d v",
                "scan abc %n v",
                "scan abc %bad v",
                "regexp é y v",
            ] {
                assert!(
                    output_names(source, dialect).is_empty(),
                    "{dialect}: {source}"
                );
            }
        }
    }

    #[test]
    fn original_matcher_query_refuses_missing_foreign_and_stale_source_inputs() {
        // naming.diagnostics.original-matcher-output-retention
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
        let source = "regexp x y v";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let config = analysis.body_lexer_config.unwrap();
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let words = crate::registry_invocation::source_structure::source_registry_words(
            source, &analysis, &command,
        )
        .unwrap();
        assert_eq!(
            selected_matcher_no_match_outputs(
                &words,
                &analysis.resolved_input.as_ref().unwrap().context_registry()
            ),
            vec!["v"]
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        assert!(selected_matcher_no_match_outputs(&words, foreign).is_empty());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            crate::registry_invocation::source_structure::source_registry_words(
                source, &missing, &command
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::source_structure::source_registry_words(
                "regexp x z v",
                &analysis,
                &command
            )
            .is_none()
        );
        let mut changed = analysis.clone();
        changed.body_lexer_config = Some(tcl_lexer::LexerConfig::for_profile(Some(
            tcl_dialect::DialectProfile::find("jim").unwrap(),
        )));
        assert!(
            crate::registry_invocation::source_structure::source_registry_words(
                source, &changed, &command
            )
            .is_none()
        );
    }

    #[test]
    fn logical_no_match_diagnostics_keep_prior_and_intervening_values_and_literal_names() {
        // naming.diagnostics.original-matcher-output-retention
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-matcher-output-retention.md
        // Emitted conditional Logical diagnostics, not Native current contents.
        for (source, expected) in [
            ("proc f {} {regexp x y v; puts $v}", Some("$v")),
            ("proc f {} {scan abc %d v; puts $v}", Some("$v")),
            ("proc f {} {if {![regexp x y v]} {puts $v}}", Some("$v")),
            ("proc f {} {regexp x y {$v}; puts ${$v}}", Some("${$v}")),
            ("proc f {} {set v OLD; regexp x y v; puts $v}", None),
            ("proc f {v} {regexp x y v; puts $v}", None),
            ("proc f {} {regexp x y v; set v NEW; puts $v}", None),
            (
                "proc f {} {if {![regexp x y v]} {set v NEW; puts $v}}",
                None,
            ),
            (
                "namespace eval app {proc regexp {pattern input name} {upvar 1 $name output; set output CUSTOM; return 0}}; proc f {} {if {![::app::regexp x y v]} {puts $v}}",
                None,
            ),
            (
                "set marker PRELUDE\nproc f {} {if {![regexp x y v]} {puts $v}}",
                Some("$v"),
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            assert!(analysis.allows_retained_logical_declaration_advice());
            let spans = analysis
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == DiagCode::W210)
                .map(|diagnostic| &source[diagnostic.span.as_range()])
                .collect::<Vec<_>>();
            assert_eq!(spans, expected.into_iter().collect::<Vec<_>>(), "{source}");
        }
    }
}
