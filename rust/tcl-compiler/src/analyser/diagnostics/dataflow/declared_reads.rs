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

//! Reads of declared variables that the SSA form does not carry: static alternatives, conditional occurrences and definitely missing contents.

use super::super::helpers::block_dominated_by;
use super::{
    DeclaredReadProofs, ReadBeforeSetCtx, StaticReadUseScope, function_declaration_flow,
    rbs_read_option, rbs_trace_read, startup_read_facts,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::Severity;
use crate::compilation_unit::ExistencePoint;
use tcl_core_types::DiagCode;
use tcl_registry::value_transfer::{DomainFact, Existence, FactView};

impl Analyser {
    pub(super) fn static_read_alternatives<'a>(
        &self,
        fu: &'a crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        unit: Option<&crate::compilation_unit::CompilationUnit>,
    ) -> Option<StaticReadUseScope<'a>> {
        let registry = self.profile_registry();
        let input = fu.source_metadata_input()?;
        fu.invocation_metadata_context(&registry)?;
        let source = rbs_read_option(
            "static_lookup_alternatives",
            unit.and_then(|unit| {
                unit.original_native_declaration_static_read_alternatives(fu, &registry)
            }),
        )?;
        source
            .matches_original_procedure(
                procedure,
                &tcl_lexer::SourceImage::document(&self.source),
                input,
            )
            .then_some(())?;
        Some(StaticReadUseScope {
            ssa: &fu.ssa,
            source,
        })
    }

    pub(super) fn declared_read_flow_for_procedure(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        original_image: &tcl_lexer::SourceImage,
    ) -> Option<std::sync::Arc<crate::command_binding::DeclarationFlowReport>> {
        rbs_read_option(
            "function_flow",
            function_declaration_flow(fu, procedure, original_image, &self.profile_registry()),
        )
        .or_else(|| {
            rbs_read_option(
                "fallback_read_flow",
                self.head_identities
                    .source_bindings_ref()
                    .original_procedure_read_flow(
                        procedure,
                        original_image,
                        &self.profile_registry(),
                        self.result.resolved_input.as_ref()?,
                    ),
            )
        })
    }

    pub(super) fn authored_read_advice_for_procedure(
        &self,
        procedure: &crate::ir::Procedure,
    ) -> Option<crate::script_binds::AuthoredProcedureReadAdvice> {
        rbs_read_option(
            "read_recipe",
            crate::script_binds::authored_procedure_read_advice(
                &procedure.body.executed_source.as_ref().map_or_else(
                    || tcl_lexer::SourceImage::document(&self.source),
                    |source| source.origin.source_image().clone(),
                ),
                procedure,
                &self.result,
                &self.analysis_context(),
            ),
        )
    }

    pub(super) fn emit_declaration_potential_reads(
        &mut self,
        potential: Option<std::collections::HashMap<String, tcl_lexer::Span>>,
    ) {
        if let Some(potential) = potential {
            let mut potential: Vec<_> = potential.into_iter().collect();
            potential.sort_by_key(|(_, span)| span.start());
            for (name, span) in potential {
                rbs_trace_read("emit_declared_potential", &name, span);
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
    pub(super) fn declaration_potential_read_spans(
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
        let alternatives = self.static_read_alternatives(fu, procedure, ctx.original_unit);
        let mut reads = std::collections::HashMap::new();
        for (name, &span) in report.declared_absence_warning_occurrences() {
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
                eprintln!(
                    "RBS_READ_OWNER stage=declared_absence_candidate name={name} span={span:?}"
                );
            }
            if !declaration.owns(span)
                || !report.allows_declared_absence_warning(name, span)
                || alternatives
                    .as_ref()
                    .is_some_and(|alternative| alternative.may_select_static(name, span))
                || proved.contains_key(name)
                || ctx.scope_aliases.contains(name)
                || report.has_declared_alias(name)
                || ctx.supp.suppresses(name)
                || ctx.extra_known_defined.contains(name)
                || super::super::helpers::original_read_cannot_be_undefined_at_span(
                    fu,
                    span,
                    &self.profile_registry(),
                )
            {
                continue;
            }
            let killed = super::super::helpers::original_read_occurrence_at_span(fu, span)
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
                                    && super::super::helpers::original_read_cell(
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

    /// Keep existing flow-report projection and source-only occurrences distinct.
    pub(super) fn conditional_read_spans_for_purpose(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        declaration: &crate::script_binds::AuthoredProcedureReadAdvice,
        report: Option<&crate::command_binding::DeclarationFlowReport>,
        ctx: &ReadBeforeSetCtx<'_>,
        proofs: DeclaredReadProofs<'_>,
    ) -> Option<std::collections::HashMap<String, tcl_lexer::Span>> {
        match report {
            Some(report) => self.declaration_potential_read_spans(
                fu,
                procedure,
                declaration,
                report,
                ctx,
                proofs,
            ),
            None => self.original_declared_source_potential_reads(
                fu,
                procedure,
                declaration,
                ctx,
                proofs,
            ),
        }
    }

    /// Add source-owned conditional occurrences without lending report facts to
    /// physical warning suppression or other diagnostic families.
    fn original_declared_source_potential_reads(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        procedure: &crate::ir::Procedure,
        declaration: &crate::script_binds::AuthoredProcedureReadAdvice,
        ctx: &ReadBeforeSetCtx<'_>,
        proofs: DeclaredReadProofs<'_>,
    ) -> Option<std::collections::HashMap<String, tcl_lexer::Span>> {
        let input = fu.source_metadata_input()?;
        let registry = self.profile_registry();
        fu.invocation_metadata_context(&registry)?;
        let image = tcl_lexer::SourceImage::document(&self.source);
        let occurrences = fu.ssa.blocks.iter().find_map(|(&block, data)| {
            (0..data.statements.len())
                .chain(std::iter::once(usize::MAX))
                .find_map(|index| {
                    let tokens = crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index)
                        .source_tokens()?;
                    tokens
                        .source_binding
                        .as_ref()?
                        .original_declared_procedure_read_occurrences(
                            tokens, input, procedure, &image, &registry,
                        )
                })
        })?;
        let DeclaredReadProofs {
            exists_guards,
            proved,
        } = proofs;
        let mut reads = std::collections::HashMap::new();
        for (name, &span) in occurrences.occurrences() {
            if !declaration.owns(span)
                || proved.contains_key(name)
                || ctx.scope_aliases.contains(name)
                || ctx.supp.suppresses(name)
                || ctx.extra_known_defined.contains(name)
                || super::super::helpers::original_read_cannot_be_undefined_at_span(
                    fu, span, &registry,
                )
            {
                continue;
            }
            let killed = super::super::helpers::original_read_occurrence_at_span(fu, span)
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
                                    && super::super::helpers::original_read_cell(access, &registry)
                                        .is_some_and(|cell| {
                                            exists_guards.iter().any(|(guard, dominator)| {
                                                guard == &cell
                                                    && block_dominated_by(
                                                        &fu.ssa, block, *dominator,
                                                    )
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

    fn original_read_startup_readable(
        &self,
        access: &crate::command_binding::SourceVariableAccess,
        read_cell: Option<&crate::var_resolve::VariableCellKey>,
        read_reference: Option<crate::ssa::SsaReadReference>,
        killed: bool,
        ctx: &ReadBeforeSetCtx<'_>,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        let incoming = access.context_alternatives().iter().all(|context| {
            let place = access.place_in_context(context, registry);
            context.contents_origin(&place) == crate::var_resolve::ContentsOrigin::Incoming
        });
        read_cell.is_some_and(|cell| {
            let version = read_reference.and_then(|read| read.version).unwrap_or(0);
            let facts = startup_read_facts(
                cell,
                version,
                killed,
                ctx.initial_global,
                ctx.global_aliases,
                Some(self.analysis_context().context().authoring_query()),
                &self.profile_registry(),
            );
            facts.lazy_read || incoming && facts.readable
        })
    }

    fn original_read_reference_was_killed(
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReadBeforeSetCtx<'_>,
        read_reference: Option<crate::ssa::SsaReadReference>,
    ) -> bool {
        read_reference.is_some_and(|read| {
            read.version.is_some_and(|version| {
                fu.ssa
                    .cell_key(read.symbol)
                    .is_some_and(|cell| ctx.supp.killed.contains(&(cell.clone(), version)))
            })
        })
    }

    /// The existence rung has the last word: a place the solver proves bound
    /// at a read is not missing, whatever the merged alternatives say (an
    /// `info exists` fold removes the edge the unbound alternative arrives
    /// on). `index` is the statement, or `usize::MAX` for the terminator.
    fn read_is_proven_bound(
        fu: &crate::compilation_unit::FunctionUnit,
        read_reference: Option<crate::ssa::SsaReadReference>,
        block: crate::cfg::BlockId,
        index: usize,
    ) -> bool {
        let point = if index == usize::MAX {
            ExistencePoint::Exit(block)
        } else {
            ExistencePoint::Before(block, index)
        };
        read_reference.is_some_and(|read| {
            matches!(
                fu.existence(read.symbol, point),
                FactView::Domain(DomainFact::Existence(Existence::Bound(_)))
            )
        })
    }

    /// Potential missing contents require closed original read alternatives,
    /// independently of SSA versions. Unknown worlds and existing array roots
    /// cannot donate an undef finding.
    pub(super) fn definite_missing_read_spans(
        &self,
        fu: &crate::compilation_unit::FunctionUnit,
        ctx: &ReadBeforeSetCtx<'_>,
        exists_guards: &[super::super::helpers::ExistenceGuard],
    ) -> std::collections::HashMap<String, tcl_lexer::Span> {
        let mut missing = std::collections::HashMap::new();
        let registry = self.profile_registry();
        let grammar = self.lexer_config().braced_var;
        let condition_writes = super::super::helpers::collect_condition_writes(fu);
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
                    let Some(presence) = view
                        .read_contents_presence_alternatives_at(
                            &access.source,
                            &access.original_spelling,
                            &registry,
                        )
                        .filter(|presence| presence.may_be_undefined())
                    else {
                        continue;
                    };
                    // Reading a live array as a scalar is a wrong-kind error,
                    // rather than evidence that the root cell is undefined.
                    if super::super::helpers::original_read_is_live_array_scalar(access, &registry)
                    {
                        continue;
                    }
                    let read_cell = super::super::helpers::original_read_cell(access, &registry);
                    if read_cell.as_ref().is_some_and(|cell| {
                        super::super::helpers::original_condition_read_is_guarded(
                            fu,
                            block,
                            fu.abs_span(access.source.span),
                            cell,
                            &registry,
                        )
                    }) {
                        continue;
                    }
                    let read_reference =
                        view.read_reference(&access.source, &access.original_spelling);
                    let killed = Self::original_read_reference_was_killed(fu, ctx, read_reference);
                    let name = tcl_syntax::naming::var_reference_for_style(
                        &access.original_spelling,
                        grammar,
                    )
                    .to_owned();
                    if !killed && Self::read_is_proven_bound(fu, read_reference, block, index) {
                        continue;
                    }
                    if ctx.supp.suppresses(&name)
                        || !killed
                            && exists_guards.iter().any(|(guarded, dominator)| {
                                read_cell.as_ref().is_some_and(|cell| guarded == cell)
                                    && block_dominated_by(&fu.ssa, block, *dominator)
                            })
                        || condition_writes.covers(&name, &fu.ssa, block, presence.may_be_defined())
                    {
                        continue;
                    }
                    let startup_readable = self.original_read_startup_readable(
                        access,
                        read_cell.as_ref(),
                        read_reference,
                        killed,
                        ctx,
                        &registry,
                    );
                    if startup_readable
                        || read_cell
                            .as_ref()
                            .is_some_and(|cell| ctx.cell_facts.known_defined.contains(cell))
                        || ctx.extra_known_defined.contains(name.as_str())
                    {
                        continue;
                    }
                    let span = fu.abs_span(access.source.span);
                    rbs_trace_read("physical_candidate", &name, span);
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
}
