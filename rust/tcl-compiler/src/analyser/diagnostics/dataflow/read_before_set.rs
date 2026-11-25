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

//! W210 and W213: the reads of each undefined def-use chain, and the guards, existence queries and barriers that excuse one.

use super::super::helpers::{UndefSuppression, block_dominated_by, collect_existence_guards};
use super::{
    DeclaredReadProofs, ReadBeforeSetCtx, StartupReadFacts, StaticReadUseScope, rbs_read_option,
    rbs_trace_count, rbs_trace_enabled, rbs_trace_read, reportable, startup_read_facts,
    undefined_var_suggestion,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::compilation_unit::ExistencePoint;
use std::collections::HashSet;
use tcl_core_types::DiagCode;
use tcl_registry::value_transfer::{Existence, FactView};

/// The existence fact the read at `index` of `block` finds at `var`'s place:
/// the statement's own read, or — for the terminator, `index` -1
/// — the block's exit, as the unit's typed view
/// ([`crate::compilation_unit::FunctionUnit::existence`]): `Pending`
/// where the run never reached it, and `Unavailable` below the deep tier or
/// past the complexity ceiling, neither of which is bound or unbound.
fn place_fact(
    fu: &crate::compilation_unit::FunctionUnit,
    block: &str,
    index: i32,
    cell: &crate::var_resolve::VariableCellKey,
) -> FactView {
    let (Some(block), Some(symbol)) = (fu.cfg.block_id(block), fu.ssa.cell_symbol(cell)) else {
        return FactView::Pending;
    };
    let point = match usize::try_from(index) {
        Ok(index) => ExistencePoint::Before(block, index),
        Err(_) => ExistencePoint::Exit(block),
    };
    fu.existence(symbol, point)
}

/// Destruction at an original normal transfer, selected under the actual
/// function metadata. A spelling or reporting name cannot select this policy.
fn destroys_variable(
    fu: &crate::compilation_unit::FunctionUnit,
    site: &crate::def_use::UseSite,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::registry_invocation::NormalTransferInvocation> {
    let block = fu.cfg.block_id(&site.block)?;
    let index = usize::try_from(site.statement_index).ok()?;
    let metadata = fu.invocation_metadata_context(registry)?;
    let tokens = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).source_tokens()?;
    let selected = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )?;
    selected
        .variable_traits()
        .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
        .then_some(selected)
}

/// Declaration source advice and a separately sealed possible static lookup.
#[derive(Clone, Copy)]
struct W210DeclarationReads<'a> {
    advice: Option<&'a crate::script_binds::AuthoredProcedureReadAdvice>,
    statics: Option<&'a StaticReadUseScope<'a>>,
    pub(super) scope_aliases: &'a HashSet<String>,
}

/// Facts used while recording the read sites of one undef def-use chain.
struct W210ChainCtx<'a> {
    pub(super) cell_facts: &'a super::super::helpers::DiagnosticCellFacts,
    pub(super) exists_guards: &'a [super::super::helpers::ExistenceGuard],
    pub(super) supp: &'a UndefSuppression,
    startup: StartupReadFacts,
}

impl Analyser {
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
    pub(in super::super) fn emit_read_before_set_diagnostics(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&crate::ir::Procedure>,
        ctx: &ReadBeforeSetCtx<'_>,
    ) -> HashSet<String> {
        if rbs_trace_enabled() {
            eprintln!(
                "RBS_READ_OWNER stage=entry procedure={} dynamic_writes={} body_kind={:?}",
                ir_proc.is_some(),
                fu.dynamic_names.writes,
                ir_proc.map(|procedure| procedure.body_kind)
            );
        }
        let exists_guards = collect_existence_guards(
            fu,
            self.analysis_context().commands(),
            fu.source_lexer_config(),
        );
        let mut w210_min = self.definite_missing_read_spans(fu, ctx, &exists_guards);
        rbs_trace_count("physical_missing", w210_min.len());
        let original_image = tcl_lexer::SourceImage::document(&self.source);
        let declared_flow = ir_proc.and_then(|procedure| {
            self.declared_read_flow_for_procedure(fu, procedure, &original_image)
        });

        // Unknown writers retire physical absence proofs. Declaration advice
        // must establish a particular original read; ownership of the whole
        // body does not close an unknown option grammar or caller frame.
        if fu.dynamic_names.writes {
            let potential = ir_proc.and_then(|procedure| {
                let declaration = rbs_read_option(
                    "read_recipe",
                    crate::script_binds::authored_procedure_read_advice(
                        &original_image,
                        procedure,
                        &self.result,
                        &self.analysis_context(),
                    ),
                )?;
                let proofs = DeclaredReadProofs {
                    exists_guards: &exists_guards,
                    proved: &w210_min,
                };
                rbs_read_option(
                    "potential_reads",
                    self.conditional_read_spans_for_purpose(
                        fu,
                        procedure,
                        &declaration,
                        rbs_read_option("required_flow", declared_flow.as_deref()),
                        ctx,
                        proofs,
                    ),
                )
            });
            suppress_declared_read_warnings(fu, ctx, declared_flow.as_deref(), &mut w210_min);
            let reported = w210_min.keys().cloned().collect();
            rbs_trace_count("map_dynamic", w210_min.len());
            self.emit_w210_read_spans(w210_min, ctx);
            self.emit_declaration_potential_reads(potential);
            return reported;
        }

        // An authored declaration can delimit a diagnostic read without
        // overriding unknown writes, accepted operand layouts or SSA origins.
        let declaration_advice =
            ir_proc.and_then(|procedure| self.authored_read_advice_for_procedure(procedure));

        // Physical namespace write advice retains exact keys. Authored
        // host-name advice remains separate in extra_known_defined.
        let _chain_read_owners = self.collect_declared_chain_read_spans(
            fu,
            ir_proc,
            ctx,
            &exists_guards,
            declaration_advice.as_ref(),
            &mut w210_min,
        );

        let potential = declaration_advice.as_ref().and_then(|declaration| {
            rbs_read_option(
                "potential_reads",
                self.declaration_potential_read_spans(
                    fu,
                    ir_proc?,
                    declaration,
                    rbs_read_option("required_flow", declared_flow.as_ref())?,
                    ctx,
                    DeclaredReadProofs {
                        exists_guards: &exists_guards,
                        proved: &w210_min,
                    },
                ),
            )
        });
        suppress_declared_read_warnings(fu, ctx, declared_flow.as_deref(), &mut w210_min);
        let reported = w210_min.keys().cloned().collect();
        rbs_trace_count("map_chain", w210_min.len());
        self.emit_w210_read_spans(w210_min, ctx);
        self.emit_declaration_potential_reads(potential);
        reported
    }

    fn collect_declared_chain_read_spans<'a, 'b>(
        &mut self,
        fu: &'a crate::compilation_unit::FunctionUnit,
        ir_proc: Option<&'b crate::ir::Procedure>,
        ctx: &ReadBeforeSetCtx<'_>,
        exists_guards: &[super::super::helpers::ExistenceGuard],
        declaration_advice: Option<&crate::script_binds::AuthoredProcedureReadAdvice>,
        w210_min: &mut std::collections::HashMap<String, tcl_lexer::Span>,
    ) -> (Option<StaticReadUseScope<'a>>, HashSet<&'b str>) {
        let params_owned: HashSet<&str> = match ir_proc {
            Some(p) => p.params.iter().map(String::as_str).collect(),
            None => HashSet::new(),
        };
        let params = &params_owned;

        let alternatives = ir_proc
            .and_then(|procedure| self.static_read_alternatives(fu, procedure, ctx.original_unit));
        self.collect_chain_read_before_set_spans(
            fu,
            ctx,
            params,
            exists_guards,
            W210DeclarationReads {
                advice: declaration_advice,
                statics: alternatives.as_ref(),
                scope_aliases: ctx.scope_aliases,
            },
            w210_min,
        );
        (alternatives, params_owned)
    }

    fn collect_chain_read_before_set_spans(
        &mut self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReadBeforeSetCtx<'_>,
        params: &HashSet<&str>,
        exists_guards: &[super::super::helpers::ExistenceGuard],
        declaration: W210DeclarationReads<'_>,
        w210_min: &mut std::collections::HashMap<String, tcl_lexer::Span>,
    ) {
        use crate::def_use::DefKind;

        // W210 fires **once per variable**, at the earliest read-before-set.
        // The def-use walk below
        // visits *every* version-0 use, so record the earliest passing span
        // per variable here and emit after the walk (W213, a distinct code,
        // stays inline).
        for chain in fu.def_use.chains.values() {
            let Some(physical_key) = fu.def_use.physical_value_key(&fu.ssa, chain.key) else {
                continue;
            };
            // Version-0 synthetic defs are the undef origin; an
            // `unset`-killed real version, and a phi version that can reach
            // an undef origin (one-branch `set` / try-handler merge), are
            // undef at their reads too — all flow through the same
            // suppression + emission logic below.
            if chain.definition.kind != DefKind::Parameter
                && !ctx.supp.killed.contains(&physical_key)
                && !ctx.supp.can_undef.contains(&physical_key)
            {
                continue;
            }
            let (cell_name, version) = &chain.key;
            let Some(symbol) = fu.def_use.symbol(&fu.ssa, *cell_name) else {
                continue;
            };
            let Some(cell_name) = fu.ssa.cell_key(symbol) else {
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
                ctx.supp.killed.contains(&physical_key),
                ctx.initial_global,
                ctx.global_aliases,
                Some(self.analysis_context().context().authoring_query()),
                &self.profile_registry(),
            );
            if startup.lazy_read && ctx.supp.killed.contains(&physical_key) {
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
                let base_defined = fu.def_use.chains.keys().any(|(key, version)| {
                    *version > 0
                        && fu
                            .def_use
                            .cell_key(&fu.ssa, *key)
                            .is_some_and(|key| cell_name.is_member_of(key))
                }) || fu.ssa.blocks.values().any(|block| {
                    block.statements.iter().any(|statement| {
                        statement.defs.keys().any(|symbol| {
                            fu.ssa
                                .cell_key(*symbol)
                                .is_none_or(|cell| cell_name.is_member_of(cell))
                        })
                    })
                });
                if !ctx.supp.killed.contains(&physical_key)
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
                && !ctx.supp.killed.contains(&physical_key)
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
            if ctx.scope_aliases.contains(var) && !ctx.supp.killed.contains(&physical_key) {
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
            if ctx.supp.suppresses_read(var, &physical_key) {
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
                declaration,
                w210_min,
            );
        }
    }

    pub(super) fn emit_w210_read_spans(
        &mut self,
        spans: std::collections::HashMap<String, tcl_lexer::Span>,
        ctx: &ReadBeforeSetCtx<'_>,
    ) {
        use std::fmt::Write as _;
        let mut entries: Vec<(String, tcl_lexer::Span)> = spans.into_iter().collect();
        entries.sort_by_key(|(_, s)| s.start());
        for (var, span) in entries {
            rbs_trace_read("emit_read_map", &var, span);
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
        declaration: W210DeclarationReads<'_>,
        w210_min: &mut std::collections::HashMap<String, tcl_lexer::Span>,
    ) {
        use crate::def_use::UseKind;
        use crate::ir::Statement;

        let Some(physical_key) = fu.def_use.physical_value_key(&fu.ssa, chain.key) else {
            return;
        };
        let (cell_name, version) = &chain.key;
        let Some(symbol) = fu.def_use.symbol(&fu.ssa, *cell_name) else {
            return;
        };
        let Some(cell_name) = fu.ssa.cell_key(symbol) else {
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
            // A definition its statement left untouched holds the prior
            // version on the paths the solver's outcome runs, and only there:
            // a read in a block that outcome makes unreachable — the match
            // arm of a `regexp` that never matches — reads a written value.
            if ctx.supp.preserved_undef.contains(&physical_key)
                && fu
                    .cfg
                    .block_id(&use_site.block)
                    .is_none_or(|id| !fu.sccp.executable_blocks.contains(&id))
            {
                continue;
            }
            // An after-loop read of a variable the loop body defines on every
            // iteration is not read-before-set (see
            // `UndefSuppression::loop_entry_only_undef`): we assume a may-run
            // loop runs, matching C Tcl. A read *inside* the loop body still
            // fires.
            if ctx.supp.after_loop_defined(&physical_key, &use_site.block) {
                continue;
            }
            // A read at or after an `expr` substitution's write of the name.
            if fu.cfg.block_id(&use_site.block).is_some_and(|block| {
                ctx.supp.written_by_substitution_before(
                    var,
                    &fu.ssa,
                    block,
                    use_site.statement_index,
                )
            }) {
                continue;
            }
            // A name nothing in the function assigns, read after code the module
            // cannot see: the code may have set it, as a sourced file or a
            // command the module does not define may set a global.
            if *version == 0
                && fu.cfg.block_id(&use_site.block).is_some_and(|block| {
                    ctx.supp
                        .unseen_call_before(&fu.ssa, block, use_site.statement_index)
                })
            {
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
            if span.is_empty() || declaration.advice.is_some_and(|advice| !advice.owns(span)) {
                continue;
            }
            // The effects a statement's substitutions state read each name
            // they may write, so the store before stays live where the write
            // does not happen (`set rc [catch {set fd [open $f]}]`); that use
            // is no read the source makes (#2323).
            if let Some(Statement::Call { defs, .. }) = stmt_opt
                && matches!(
                    stmt_opt.and_then(Statement::synthetic_marker),
                    Some(
                        crate::ir::SyntheticMarker::WordEffects(_)
                            | crate::ir::SyntheticMarker::Condition
                    )
                )
                && defs.iter().any(|defined| defined == var)
            {
                continue;
            }
            let Some(use_id) = fu.ssa.block_id(&use_site.block) else {
                continue;
            };
            let use_index = usize::try_from(use_site.statement_index).unwrap_or(usize::MAX);
            let source_view = crate::ssa::SsaSourceView::at_statement(&fu.ssa, use_id, use_index);
            if source_view.owns_activation_cell(symbol) == Some(false)
                && !super::super::helpers::original_read_cells_at(
                    &fu.ssa,
                    (use_id, use_index),
                    cell_name,
                    &self.profile_registry(),
                )
                .contains(cell_name)
            {
                continue;
            }
            if super::super::helpers::read_has_cell_fact(
                fu,
                (use_id, use_index),
                cell_name,
                &ctx.cell_facts.known_defined,
                &self.profile_registry(),
            ) {
                continue;
            }
            if use_site.class == crate::ssa::UseClass::Name
                && let Some(statement @ (Statement::AssignValue { .. } | Statement::Call { .. })) =
                    stmt_opt
                && super::super::helpers::original_argument_use_has_no_read(
                    fu,
                    statement,
                    (use_id, use_index),
                    cell_name,
                    &self.profile_registry(),
                )
            {
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
                            .is_some_and(|read| fu.ssa.cell_key(read.symbol) == Some(cell_name))
                    })
                    .any(|access| {
                        super::super::helpers::original_read_is_live_array_scalar(
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
            // reads, or `<word-effects>` holding a word's — has no source
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
            if is_existence_query_word(
                stmt_opt,
                cell_name,
                ctx.exists_guards,
                fu,
                use_site,
                self.analysis_context().commands(),
            ) {
                continue;
            }
            if use_site.kind == crate::def_use::UseKind::VariableName
                && stmt_opt.is_some_and(|stmt| {
                    substitution_unbinds(stmt, var, fu, &self.profile_registry())
                })
            {
                continue;
            }
            // An unbind of the place — a command the registry declares
            // `DESTROYS_VARIABLE`, under the spelling the source uses —
            // reads the existence fact where it runs: W213 is
            // definite on an unbound place, "may not exist" on a may-bound
            // one, and nothing on a bound one or where the run computed no
            // fact. The `-nocomplain` form raises nothing, so it reports
            // neither W213 nor, since destroying is no read, W210.
            if let Some(Statement::Call { tokens, .. }) = stmt_opt
                && let Some(selected) = destroys_variable(fu, use_site, &self.profile_registry())
            {
                let targets = selected.variable_roles();
                if (0..selected.argument_count()).any(|ordinal| {
                    !targets.iter().any(|(index, _)| *index == ordinal)
                        && selected.argument_literal(ordinal).as_deref() == Some("-nocomplain")
                }) {
                    continue;
                }
                let Some(fact) = reportable(&place_fact(
                    fu,
                    &use_site.block,
                    use_site.statement_index,
                    cell_name,
                )) else {
                    continue;
                };
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
                let message = if fact == Existence::Unbound {
                    format!(
                        "Variable '{var}' does not exist here; \
                         use 'unset -nocomplain' to suppress the error",
                    )
                } else {
                    format!(
                        "Variable '{var}' may not exist; \
                         use 'unset -nocomplain' to suppress the error",
                    )
                };
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
            // The existence rung has the last word: a read at a place
            // bound there, or where the run computed no fact, is no
            // read-before-set.
            if reportable(&place_fact(
                fu,
                &use_site.block,
                use_site.statement_index,
                cell_name,
            ))
            .is_none()
            {
                continue;
            }
            // Anchor at the `$var` read token; fall back to the command
            // span when the read is nested inside a quoted/compound word.
            let read_span = if use_index == usize::MAX
                && matches!(
                    block.terminator.as_ref(),
                    Some(
                        crate::cfg::Terminator::Branch { .. }
                            | crate::cfg::Terminator::Return { expr: Some(_), .. }
                    )
                ) {
                let original: Vec<_> = source_view
                    .source_tokens()
                    .into_iter()
                    .flat_map(|tokens| &tokens.variable_accesses)
                    .filter(|access| {
                        source_view
                            .read_reference(&access.source, &access.original_spelling)
                            .is_some_and(|read| fu.ssa.cell_key(read.symbol) == Some(cell_name))
                    })
                    .collect();
                let surviving = original
                    .iter()
                    .filter_map(|access| {
                        let read = fu.abs_span(access.source.span);
                        let guarded = super::super::helpers::original_read_cell(
                            access,
                            &self.profile_registry(),
                        )
                        .is_some_and(|cell| {
                            super::super::helpers::original_condition_read_is_guarded(
                                fu,
                                use_id,
                                read,
                                &cell,
                                &self.profile_registry(),
                            )
                        });
                        (!guarded).then_some(read)
                    })
                    .min_by_key(|span| span.start());
                if !original.is_empty() && surviving.is_none() {
                    continue;
                }
                surviving.unwrap_or_else(|| self.narrow_to_read_var(span, var).unwrap_or(span))
            } else {
                self.narrow_to_read_var(span, var).unwrap_or(span)
            };
            // Only this exact use's own activation can have the alternate
            // static lookup. Explicit foreign/up-frame reads and missing point
            // contexts survive; later surviving uses still enter the minimum.
            let own_activation = source_view.owns_activation_cell(symbol);
            let native_alternative = own_activation == Some(true)
                && matches!(
                    cell_name,
                    crate::var_resolve::VariableCellKey::Activation { simple, .. }
                        if simple.as_bytes() == var.as_bytes()
                )
                && !declaration.scope_aliases.contains(var)
                && declaration.statics.is_some_and(|alternatives| {
                    fu.ssa
                        .point_contexts
                        .as_ref()
                        .and_then(|points| points.context_before(use_id, use_index))
                        .is_some_and(|context| {
                            alternatives
                                .may_select_static_in_declared_frame(var, read_span, context)
                        })
                });
            let static_alternative = native_alternative
                || (!declaration.scope_aliases.contains(var)
                    && declaration.statics.is_some_and(|alternatives| {
                        alternatives.may_select_direct_source_read(
                            source_view,
                            symbol,
                            var,
                            read_span,
                        )
                    }));
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
                let storage_kind = match cell_name.root() {
                    crate::var_resolve::VariableCellKey::Authored(_) => "authored",
                    crate::var_resolve::VariableCellKey::Activation { .. } => "activation",
                    _ => "other",
                };
                eprintln!(
                    "RBS_READ_OWNER stage=static_lookup_at_use name={var} storage_kind={storage_kind} own_activation={own_activation:?} native_alternative={native_alternative} static_alternative={static_alternative} use_class={:?} use_kind={:?} statement_index={}",
                    use_site.class, use_site.kind, use_site.statement_index,
                );
                if let Some(alternatives) = declaration.statics {
                    alternatives.source.trace_static_read_cell(cell_name, var);
                }
            }
            if static_alternative {
                continue;
            }
            rbs_trace_read("chain_candidate", var, read_span);
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
            || super::super::helpers::original_read_occurrence_at_span(fu, *span)
                .is_some_and(|value| ctx.supp.killed.contains(&value))
    });
}

/// Synthetic effect statements have no original argv for a diagnostic anchor.
fn statement_is_synthetic_effect(stmt: &crate::ir::Statement) -> bool {
    stmt.synthetic_marker().is_some()
}

/// Suppress a lexical body read when unchanged conditional source advice
/// owns the name in its own frame. This proves no executed store or successor
/// value. Opaque foreign bodies retain their separate lexical suppression.
pub(super) fn barrier_body_locally_sets(
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

/// Whether a `[…]` substitution of `stmt` unbinds `var`: a nested command
/// the registry declares `DESTROYS_VARIABLE` names it. Destroying is no read,
/// so the host statement has no read-before-set of that name.
fn substitution_unbinds(
    stmt: &crate::ir::Statement,
    var: &str,
    fu: &crate::compilation_unit::FunctionUnit,
    registry: &tcl_registry::CommandRegistry,
) -> bool {
    let (Some(tokens), Some(metadata)) = (stmt.tokens(), fu.invocation_metadata_context(registry))
    else {
        return false;
    };
    crate::word_subst::checked_lifted_calls(tokens, fu.source_lexer_config())
        .unwrap_or_default()
        .iter()
        .filter_map(|call| call.tokens.as_ref())
        .filter_map(|nested| {
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                registry,
                Some(metadata),
                nested,
            )
        })
        .any(|selected| {
            selected
                .variable_traits()
                .contains(tcl_registry::Traits::DESTROYS_VARIABLE)
                && selected
                    .variable_roles()
                    .iter()
                    .any(|(index, _)| selected.argument_literal(*index).as_deref() == Some(var))
        })
}

/// True when a read of `var` is the word of an existence query in the
/// statement itself (`info exists var`), which W210 never reports.
fn is_existence_query_word(
    stmt_opt: Option<&crate::ir::Statement>,
    cell: &crate::var_resolve::VariableCellKey,
    exists_guards: &[super::super::helpers::ExistenceGuard],
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
        super::super::helpers::original_read_cells_at(ssa, (use_id, index), cell, registry);
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
    super::super::helpers::original_definition_places(fu, block, index, registry)
        .iter()
        .any(|place| crate::var_resolve::canonical_place_key(place).as_ref() == Some(cell))
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
) -> (tcl_lexer::Span, Vec<super::super::types::CodeFix>) {
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
        vec![super::super::types::CodeFix {
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
