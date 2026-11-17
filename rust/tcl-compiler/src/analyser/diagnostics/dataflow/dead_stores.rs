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

//! W220 dead stores, the conditional declared-store warning and the H300 paste-error fingerprint.

use super::{find_case_mismatch, function_declaration_flow};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_dialect::model::SurfaceQuery;

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

/// Namespace storage its writer names qualified (`::g`, `ns::v`), which
/// code outside the module may read. A frame's own unqualified store, the
/// global frame's `set g 1` included, answers to the module's reads alone.
pub(super) fn written_namespace_storage(
    cell_name: &crate::var_resolve::VariableCellKey,
    var: &str,
) -> bool {
    cell_name.namespace_membership_for_advice().is_some()
        && tcl_syntax::naming::is_qualified(var.as_bytes())
}

/// A cell of an object's own storage: every method body of the object reads
/// it, so one body's reads alone never prove a store unused.
pub(super) fn written_object_storage(cell_name: &crate::var_resolve::VariableCellKey) -> bool {
    matches!(
        cell_name.root(),
        crate::var_resolve::VariableCellKey::Instance { .. }
            | crate::var_resolve::VariableCellKey::AllocatedInstance { .. }
    )
}

/// Scope facts that make a store observable outside its local SSA chain.
struct DeadStoreVisibility<'a> {
    pub(super) scope_aliases: &'a HashSet<String>,
    cross_event_vars: &'a HashSet<String>,
    pub(super) dialect: Option<SurfaceQuery<'a>>,
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
    if written_namespace_storage(cell_name, var) || written_object_storage(cell_name) {
        return true;
    }
    // Interpreter-provided special variables (``auto_path``, ``env``,
    // ``tcl_precision``, …) are read by the runtime / auto-loader even
    // when the script never reads them back, so ``set auto_path …`` is
    // not a dead store.  Dialect-aware: the iRules set differs.
    if super::super::helpers::special_variable_definition(fu, definition, cell_name, registry)
        .is_some_and(|simple| {
            tcl_registry::special_vars::is_externally_read(&simple, visibility.dialect)
        })
    {
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

fn dead_store_message(
    var: &str,
    conditional_unread: bool,
    overwrite: Option<OverwriteDiagnostic>,
    defined_vars: &HashSet<String>,
) -> String {
    use std::fmt::Write as _;
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
    message
}

impl Analyser {
    fn emit_dead_store_diagnostic(
        &mut self,
        var: &str,
        cmd_span: tcl_lexer::Span,
        conditional_unread: bool,
        overwrite: Option<OverwriteDiagnostic>,
        defined_vars: &HashSet<String>,
    ) {
        // Anchor at the variable name (the assignment target), not the
        // command-start column.
        let span = self.narrow_to_assigned_name(cmd_span).unwrap_or(cmd_span);
        let message = dead_store_message(var, conditional_unread, overwrite, defined_vars);
        self.result
            .diagnostics
            .push(crate::analyser::types::Diagnostic::new(
                DiagCode::W220,
                span,
                message,
                Severity::Hint,
            ));
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
    pub(in super::super) fn emit_dead_store_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        defined_vars: &HashSet<String>,
        scope_aliases: &HashSet<String>,
        cross_event_vars: &HashSet<String>,
        cell_facts: &super::super::helpers::DiagnosticCellFacts,
        module: Option<&crate::interprocedural::ModuleProcedures<'_>>,
    ) {
        use crate::def_use::DefKind;

        // A dynamic read (`[set $name]`, `subst $tmpl`) can observe *any*
        // store, so "this assignment is never read" is unprovable anywhere in
        // the function.  Abstain toward silence.
        let hidden_reads = self.substitution_hidden_reads(fu);
        // Array-element / dict-path writes the
        // name-level SSA mis-folds but that a read actually observes.
        let place_suppressed = self.place_suppressed_dead_stores(fu);
        let generation = self.analysis_context();
        let registry = generation.commands();
        let metadata = match module {
            Some(module) => module.invocation_metadata_context_for_function(fu, registry),
            None => fu.invocation_metadata_context(registry),
        };
        let Some(metadata) = metadata else {
            return;
        };
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
            let (cell_name, version) = &chain.key;
            let Some(symbol) = fu.def_use.symbol(&fu.ssa, *cell_name) else {
                continue;
            };
            let Some(cell_name) = fu.ssa.cell_key(symbol) else {
                continue;
            };
            if super::super::helpers::definition_has_cell_fact(
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
            let overwrite = original_overwrite_advice(fu, &chain.definition, var, registry, module);
            let conditional_unread = fu.dynamic_names.reads
                && chain.is_dead()
                && !hidden_reads.contains(var)
                && original_unread_store_advice(
                    fu,
                    &chain.definition,
                    var,
                    registry,
                    &unread_layout,
                    module,
                );
            if !chain.is_dead() && overwrite.is_none() {
                continue;
            }
            // A name read inside a command substitution / expr / branch
            // condition the version-precise `used` set can't see keeps every
            // write of it alive (`set i 0` before `[incr i $j]`). Suppress at
            // name level.
            if fu.ssa.name_is_observed_by_unseen_call(var, *version) {
                continue;
            }
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
                Some(metadata),
                &place_suppressed,
            ) else {
                continue;
            };
            self.emit_dead_store_diagnostic(
                var,
                cmd_span,
                conditional_unread,
                overwrite,
                defined_vars,
            );
        }
    }

    /// Authored setter ordering may supply a conditional warning even when an
    /// earlier native read fails. It supplies no physical definition or edit.
    pub(in super::super) fn emit_conditional_declared_store_diagnostics(
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
            .source_bindings_ref()
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
                || super::super::helpers::special_variable_definition_at_span(fu, span, registry)
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
    pub(in super::super) fn emit_possible_paste_error_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
    ) {
        use crate::def_use::DefKind;

        // Pre-compute, per block, the set of statement indices
        // that are dead stores.  Walk every dead Statement-kind
        // chain in def_use, bucket by block.
        let mut dead_idx: FxHashMap<&str, FxHashSet<usize>> = FxHashMap::default();
        for chain in fu.def_use.chains.values() {
            if !chain.is_dead()
                || chain.definition.kind != DefKind::Statement
                || fu.def_use.symbol(&fu.ssa, chain.key.0).is_none()
            {
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
                let Some(first) =
                    super::super::utils::possible_paste_fingerprint(&block.statements[idx])
                else {
                    continue;
                };
                let Some(second) =
                    super::super::utils::possible_paste_fingerprint(&block.statements[idx + 1])
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
                let pretty = super::super::utils::format_literal_for_message(&literal);
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
    module: Option<&crate::interprocedural::ModuleProcedures<'_>>,
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
    let first_metadata = original_store_metadata(fu, first, registry, module)?;
    let next_metadata = original_store_metadata(fu, next, registry, module)?;
    // Each point must retain its own actual generation. The second query does
    // not repair the first from an adjacent spelling or a shared frame label.
    if first_metadata.source_analysis_input() != next_metadata.source_analysis_input() {
        return None;
    }
    if crate::registry_invocation::overwritten_local_store_advice(
        registry,
        Some(first_metadata),
        first,
        next,
    )
    .is_some_and(|advice| advice.name() == variable && advice.owns(first, next))
    {
        return Some(OverwriteDiagnostic::Closed);
    }
    crate::registry_invocation::conditional_overwritten_local_store_advice(
        registry,
        Some(first_metadata),
        first,
        next,
    )
    .filter(|advice| advice.name() == variable && advice.owns(first, next))
    .map(|_| OverwriteDiagnostic::Conditional)
}

pub(super) fn original_store_metadata<'a>(
    function: &crate::compilation_unit::FunctionUnit,
    tokens: &'a crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    module: Option<&crate::interprocedural::ModuleProcedures<'_>>,
) -> Option<crate::registry_invocation::InvocationMetadataContext<'a>> {
    if let Some(module) = module {
        return module.original_invocation_metadata_for_function(function, tokens, registry);
    }
    tokens
        .source_binding
        .as_ref()?
        .original_invocation_metadata_for_function(tokens, function, registry)?
        .context()
}

/// Original uses missing from SSA only suppress store diagnostics. The query
/// changes neither physical liveness nor store-removal permission.
pub(super) fn original_unrepresented_use_advice(
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
                fu.def_use
                    .symbol(&fu.ssa, chain.key.0)
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
    module: Option<&crate::interprocedural::ModuleProcedures<'_>>,
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
    let Some(metadata) = original_store_metadata(fu, tokens, registry, module) else {
        return false;
    };
    crate::registry_invocation::conditional_unread_local_store_advice(
        registry,
        Some(metadata),
        tokens,
        report,
    )
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
        let Some(symbol) = fu.def_use.symbol(&fu.ssa, chain.key.0) else {
            continue;
        };
        let Some(cell) = fu.ssa.cell_key(symbol) else {
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
                cell
            ),
            reportable_dead_assignment(
                statement,
                registry,
                fu.invocation_metadata_context(registry)
            ),
            original_overwrite_advice(fu, &chain.definition, variable, registry, None),
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
