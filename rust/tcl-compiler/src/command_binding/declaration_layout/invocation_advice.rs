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

//! Declaration advice at one invocation: call layouts, read occurrences, operands and alias frames.

use super::super::{
    CommandAllocationSite, OriginalCompilationLookupAdvice, SourceCommandBindings,
    SourceLookupSnapshot,
};
use super::variable_symbols::original_local_primary_is_byte_unique;
use super::{
    DeclarationReadOccurrenceAdvice, DeclarationVariableOperandAdvice,
    OriginalBodyParameterSourceScope, OriginalDeclarationCallLayoutAdvice,
    OriginalDiagnosticFrameEntry, original_declaration_layouts,
};
use std::sync::Arc;

impl OriginalDeclarationCallLayoutAdvice {
    pub(crate) fn targets(&self) -> &[super::super::SourceCommandTarget] {
        self.layout.targets()
    }

    pub(crate) fn dialect(&self) -> tcl_native_model::InvocationDialect {
        self.layout.dialect()
    }

    /// Original Standalone declaration targets for a conditional source report.
    /// This reborrows the retained row's metadata, never actual point metadata.
    /// Observer applicability, successful lookup and body entry stay separate.
    pub(crate) fn original_authored_source_targets(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&[super::super::SourceCommandTarget]> {
        if !self.layout.is_authored_standalone_source() || !self.source_targets_are_closed() {
            return None;
        }
        let metadata = self
            .layout
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        if metadata.is_some_and(|metadata| !metadata.is_standalone()) {
            return None;
        }
        Some(self.targets())
    }

    /// Readonly closure of the original source candidates. This never closes
    /// actual handler, operand, provider or activation alternatives.
    pub(crate) fn source_targets_are_closed(&self) -> bool {
        self.layout.closed_lookup() && !self.layout.has_opaque_handler_alternatives()
    }
}

impl DeclarationReadOccurrenceAdvice {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn source(&self) -> &tcl_source::words::SourceSite {
        &self.source
    }

    pub(crate) fn spelling(&self) -> &str {
        &self.spelling
    }

    /// Existing symbolic value at this diagnostic boundary, conditional on
    /// the original declaration-local name retaining that value. No SSA
    /// definition, use, physical address or executable dependency is added.
    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_site()? != &self.invocation
            || !binding
                .declaration_read_occurrences(registry, tokens)?
                .contains(self)
        {
            return None;
        }
        symbolic_version(ssa, block, index, &self.name)
    }
}

impl DeclarationVariableOperandAdvice {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let binding = tokens.source_binding.as_ref()?;
        if binding.invocation_site()? != &self.invocation
            || binding
                .declaration_variable_operand_advice(registry, tokens, &self.word)?
                .name
                != self.name
        {
            return None;
        }
        symbolic_version(ssa, block, index, &self.name)
    }
}

pub(in super::super) fn symbolic_version(
    ssa: &crate::ssa::SsaFunction,
    block: crate::cfg::BlockId,
    index: usize,
    name: &str,
) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
    let symbol = ssa.cell_symbol(name)?;
    let block = ssa.blocks.get(&block)?;
    let version = if index == usize::MAX {
        block.exit_versions.get(&symbol).copied()?
    } else {
        block.statements.get(index)?;
        block.statements[..index]
            .iter()
            .rev()
            .find_map(|statement| statement.defs.get(&symbol).copied())
            .or_else(|| block.entry_versions.get(&symbol).copied())?
    };
    Some((symbol, version))
}

impl SourceCommandBindings {
    /// Existential declaration layout with unanimous candidate identities.
    /// The original invocation and body recipe must still match. Unknown
    /// runtime alternatives are deliberately not removed by this projection.
    pub(crate) fn declaration_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let mut binding = tokens.source_binding.as_ref()?.clone();
        if binding.declaration_layout_observations.is_none() {
            let site = binding.invocation_site()?;
            binding.declaration_layout_observations = self
                .declaration_layouts
                .get(site)
                .map(|observations| Arc::from(observations.as_slice()));
        }
        binding.declaration_operand_layout_advice(tokens)
    }

    /// Original source-backed callees under the recorded pre-operand table.
    /// Unlike Registry metadata advice, this retains a declaration allocation
    /// for the independent native formal binder and supplies no handler facts.
    pub(crate) fn declaration_call_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalDeclarationCallLayoutAdvice> {
        let mut binding = tokens.source_binding.as_ref()?.clone();
        if binding.declaration_layout_observations.is_none() {
            let site = binding.invocation_site()?;
            binding.declaration_layout_observations = self
                .declaration_layouts
                .get(site)
                .map(|observations| Arc::from(observations.as_slice()));
        }
        Some(OriginalDeclarationCallLayoutAdvice {
            layout: binding.original_declared_layout_advice(tokens, true)?,
        })
    }
}

impl super::super::SourceInvocationBinding {
    /// Closed source-only Builtin layout from an explicitly Standalone original
    /// declaration. Reached, Native and supplied source owners cannot borrow it.
    pub(crate) fn original_authored_declaration_scalar_layout(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::super::OriginalCompilationLookupAdvice> {
        if self.lookup_state.is_some()
            || self.has_original_native_source_command()
            || self.has_original_logical_source_advice()
            || tokens.synthetic.is_some()
            || !tokens.original_words_match_argv_text()
        {
            return None;
        }
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut selected = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.words.as_ref() != tokens.words()
                || !row.snapshot.state.baseline.metadata_context.is_standalone()
            {
                return None;
            }
            let advice = super::super::original_site_operand_layout_advice(
                site,
                tokens,
                &row.snapshot,
                &row.namespace,
                row.config,
            )?;
            let metadata = advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()?;
            if metadata.is_some_and(|metadata| !metadata.is_standalone())
                || !advice.is_authored_standalone_source()
                || advice.has_unclosed_source_coverage()
            {
                return None;
            }
            let [target] = advice.targets() else {
                return None;
            };
            if !target.registry_backed
                || target.kind != super::super::BindingKind::Builtin
                || !target.prepended.is_empty()
            {
                return None;
            }
            if selected.as_ref().is_some_and(
                |prior: &super::super::OriginalCompilationLookupAdvice| {
                    prior.targets() != advice.targets() || prior.namespace() != advice.namespace()
                },
            ) {
                return None;
            }
            selected = Some(advice);
        }
        selected
    }

    /// Original source-backed callees under this binding's unanimous declared
    /// pre-operand table. No entered target or successful call is supplied.
    pub(crate) fn declaration_call_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalDeclarationCallLayoutAdvice> {
        Some(OriginalDeclarationCallLayoutAdvice {
            layout: self.original_declared_layout_advice(tokens, true)?,
        })
    }

    /// Conditional diagnostic cell keys selected by exact original static
    /// operands at their own original point. Alias bindings and byte names
    /// come from every retained snapshot, never from reporting spellings or
    /// the caller's current table. This grants no Must destruction or Normal.
    pub(crate) fn declaration_variable_argument_keys(
        &self,
        tokens: &crate::ir::CommandTokens,
        arguments: &[usize],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<crate::var_resolve::VariableCellKey>> {
        self.declaration_operand_layout_advice(tokens)?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut unanimous = None;
        for row in rows {
            let state = &row.snapshot.state;
            let context = &state.source_variables;
            let policy = context.execution_name_policy?.native_recipe()?;
            let mut keys = Vec::new();
            for &argument in arguments {
                let word = tokens.words().get(argument.checked_add(1)?)?;
                let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
                    self.original_source_name_key_at_span(
                        word.source().span,
                        row.config,
                        context.invocation_dialect?.word_values,
                        policy,
                    )?,
                );
                let selected = policy.recipe().combined_variable_input(input.bytes());
                if !original_local_primary_is_byte_unique(state, policy, selected.root().selected())
                {
                    return None;
                }
                let place = crate::var_resolve::resolve_original_name_input(
                    &input,
                    context,
                    registry,
                    false,
                    tcl_registry::TraceOperation::Read,
                );
                keys.push(crate::var_resolve::canonical_binding_value_key(&place)?);
            }
            if unanimous.as_ref().is_some_and(|previous| previous != &keys) {
                return None;
            }
            unanimous = Some(keys);
        }
        unanimous
    }

    /// Original direct operand reads whose source and declaration-local scope
    /// agree across every retained observation. Aliases, formal inputs,
    /// qualified names and unavailable geometry provide no local advice.
    pub(crate) fn declaration_read_occurrences(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<DeclarationReadOccurrenceAdvice>> {
        let layout = self.declaration_operand_layout_advice(tokens)?;
        let site = self.invocation_site()?;
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let end = tokens.words().last()?.source().span.end();
        let config = layout.config;
        let mut reads = Vec::new();
        for access in &tokens.variable_accesses {
            if !direct_declaration_operand_owner(&access.owner, site)
                || access.source.span.start() < site.offset
                || access.source.span.end() > end
            {
                continue;
            }
            let bytes = access.original_spelling.as_bytes();
            let start = usize::try_from(access.source.span.start()).ok()?;
            if bytes.first() != Some(&b'$')
                || site
                    .source
                    .source_image()
                    .bytes()
                    .get(start..start.checked_add(bytes.len())?)
                    != Some(bytes)
            {
                continue;
            }
            let Some(reference) = tcl_lexer::word_parts::scan_var_ref(bytes, 0, config)
                .ok()
                .flatten()
                .filter(|reference| reference.next == bytes.len())
            else {
                continue;
            };
            let Ok(name) = std::str::from_utf8(reference.name) else {
                continue;
            };
            let contexts = access
                .context_alternatives()
                .iter()
                .filter(|context| {
                    observations
                        .clone()
                        .all(|observation| observation.entry.owns_original_context(context))
                })
                .collect::<Vec<_>>();
            if tcl_syntax::naming::is_qualified(reference.name)
                || contexts.is_empty()
                || contexts
                    .iter()
                    .any(|context| local_read_scope_is_excluded(context, name))
                || observations.clone().any(|observation| {
                    let context = &observation.snapshot.state.source_variables;
                    !observation
                        .entry
                        .owns_source(&site.source, access.source.span.start())
                        || observation
                            .entry
                            .parameters()
                            .iter()
                            .any(|formal| formal.name == name)
                        || contexts
                            .iter()
                            .any(|read| context.activation != read.activation)
                        || local_read_scope_is_excluded(context, name)
                        || observation
                            .source_body_name_ownership(
                                registry,
                                crate::script_binds::Ownership::ScopeAliases,
                            )
                            .is_none_or(|effects| {
                                effects.opaque
                                    || effects.names.iter().any(|candidate| candidate == name)
                            })
                })
            {
                continue;
            }
            let read = DeclarationReadOccurrenceAdvice {
                invocation: site.clone(),
                name: name.to_owned(),
                source: access.source.clone(),
                spelling: access.original_spelling.clone(),
            };
            if !reads.contains(&read) {
                reads.push(read);
            }
        }
        Some(reads)
    }

    /// Lexical local-frame classification of the accepted declaration. This
    /// is only a body-layout condition; it supplies no installed alias or
    /// runtime frame and cannot replace the normal variable context.
    pub(crate) fn declaration_alias_frame_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<tcl_registry::VariableAliasFrame> {
        self.declaration_operand_layout_advice(tokens)?;
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        if observations.clone().any(|observation| {
            !matches!(
                observation.entry.frame().layout(),
                crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                    | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            )
        }) {
            return None;
        }
        Some(tcl_registry::VariableAliasFrame::Procedure)
    }

    /// A written, statically decoded scalar operand in the accepted local
    /// declaration frame. Alias/formal/trace or conflicting layouts decline.
    /// Callers separately select the operation/operand role; this is no access.
    pub(crate) fn declaration_variable_operand_advice(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        word: &tcl_source::words::WordExpr,
    ) -> Option<DeclarationVariableOperandAdvice> {
        let layout = self.declaration_operand_layout_advice(tokens)?;
        let site = self.invocation_site()?;
        if word.source().provenance != tcl_source::words::Provenance::Source
            || !tokens.words().iter().any(|original| original == word)
        {
            return None;
        }
        // Original overrides belong to this layout, independently of its dialect.
        let config = layout.config;
        let crate::registry_invocation::EffectiveInvocationWord::Literal(name) =
            crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            )
        else {
            return None;
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_array_name_braced_for_style(&name, true, config.braced_var)
                .1
                .is_some()
        {
            return None;
        }
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        if observations.clone().any(|observation| {
            let context = &observation.snapshot.state.source_variables;
            observation
                .entry
                .parameters()
                .iter()
                .any(|formal| formal.name == name)
                || local_read_scope_is_excluded(context, &name)
                || observation
                    .source_body_name_ownership(
                        registry,
                        crate::script_binds::Ownership::ScopeAliasesAndTraceRegistrations,
                    )
                    .is_none_or(|effects| effects.opaque || effects.names.contains(&name))
        }) {
            return None;
        }
        Some(DeclarationVariableOperandAdvice {
            invocation: site.clone(),
            word: word.clone(),
            name,
        })
    }

    /// Reborrow the actual unanimous original body/declaration row for source
    /// parameter reports. Missing allocated-body inventory cannot mint a Body
    /// from an uninstalled declaration or replace a present reached owner.
    pub(crate) fn original_body_parameter_source_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalBodyParameterSourceScope> {
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let first = rows.clone().next()?;
        if !matches!(
            first.entry.as_ref(),
            OriginalDiagnosticFrameEntry::Body(_)
                | OriginalDiagnosticFrameEntry::DeclaredProcedure(_)
        ) || first.entry.original_formal_topology().is_none()
        {
            return None;
        }
        let layout = self.original_unanimous_declared_layout_advice(
            tokens,
            original_body_parameter_source_layout,
        )?;
        Some(OriginalBodyParameterSourceScope {
            entry: Arc::clone(&first.entry),
            layout,
        })
    }

    /// Candidate layout retained independently of actual normal handler facts.
    /// Exact declaration owner, original geometry and every observation agree.
    pub(crate) fn declaration_operand_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.original_declared_layout_advice(tokens, false)
    }

    pub(in super::super) fn original_declared_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        source_calls: bool,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.original_unanimous_declared_layout_advice(
            tokens,
            if source_calls {
                super::super::original_declaration_call_layout_advice
            } else {
                super::super::original_operand_layout_advice
            },
        )
    }

    /// Closed conditional callees in a positively retained Logical source
    /// world. Original vectors, input, namespace and every declaration row
    /// must agree. A source procedure target is not a physical allocation;
    /// callers separately join its original declaration/body inventory.
    pub(crate) fn original_logical_declaration_call_targets(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<super::super::SourceCommandTarget>> {
        let metadata = self.original_invocation_metadata_for_input(tokens, input, registry)?;
        (self.original_invocation_source_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            .then_some(())?;
        let advice = self.original_logical_declaration_layout_advice(tokens)?;
        advice.closed_logical_source_lookup().then_some(())?;
        // A reached invocation retains its post-operand dispatch guards. A
        // declaration-only source uses its independently sealed conditional
        // targets, while every raw binding and original row must stay unchanged.
        if self.lookup_state.is_some() {
            if self.unknown || self.may_be_absent || self.targets.as_slice() != advice.targets() {
                return None;
            }
        } else if !self
            .original_logical_declaration_targets
            .as_ref()
            .is_some_and(|receipt| receipt.matches(self, &advice))
        {
            return None;
        }
        let mut targets = advice.targets().to_vec();
        for target in &mut targets {
            target.implementation_allocation = None;
        }
        Some(targets)
    }

    /// Complete Logical declaration layout in its original source world.
    /// Internal source allocations serve lexical correspondence only; callers
    /// cannot use this purpose for Native formal storage or actual activation.
    pub(in super::super) fn original_logical_declaration_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.original_unanimous_declared_layout_advice(
            tokens,
            super::super::original_logical_declaration_call_layout_advice,
        )
    }

    fn original_unanimous_declared_layout_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        project: fn(
            &super::super::SourceInvocationBinding,
            &crate::ir::CommandTokens,
            &Arc<super::super::SourceLookupSnapshot>,
            &super::super::SourceNamespaceKey,
            tcl_lexer::LexerConfig,
        ) -> Option<OriginalCompilationLookupAdvice>,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let binding = self;
        let site =
            super::super::trace_rbs_operand_option("declaration.site", binding.invocation_site())?;
        let observations = super::super::trace_rbs_operand_option(
            "declaration.original_rows",
            original_declaration_layouts(super::super::trace_rbs_operand_option(
                "declaration.rows",
                self.declaration_layout_observations.as_deref(),
            )?),
        )?;
        let mut unanimous: Option<OriginalCompilationLookupAdvice> = None;
        for observation in observations {
            // Entered activations retain their own immutable points; they do
            // not participate in this original declaration's unanimity.
            if !observation.entry.owns_source(&site.source, site.offset) {
                return super::super::trace_rbs_operand_option("declaration.row_owner", None);
            }
            let advice = super::super::trace_rbs_operand_option(
                "declaration.row_project",
                project(
                    binding,
                    tokens,
                    &observation.snapshot,
                    &observation.namespace,
                    observation.config,
                ),
            )?;
            if unanimous.as_ref().is_some_and(|previous| {
                previous.targets != advice.targets
                    || previous.dialect != advice.dialect
                    || previous.realm() != advice.realm()
                    || previous.alias_frame() != advice.alias_frame()
                    || previous.frame() != advice.frame()
                    || previous.namespace() != advice.namespace()
                    || previous.unknown != advice.unknown
                    || previous.may_be_absent != advice.may_be_absent
            }) {
                return super::super::trace_rbs_operand_option("declaration.unanimity", None);
            }
            unanimous = Some(advice);
        }
        super::super::trace_rbs_operand_option("declaration.result", unanimous)
    }
}

fn original_body_parameter_source_layout(
    binding: &super::super::SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &super::super::SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    let rows = original_declaration_layouts(binding.declaration_layout_observations.as_deref()?)?;
    let entry = &rows.clone().next()?.entry;
    let layout =
        super::super::original_operand_layout_advice(binding, tokens, snapshot, namespace, config)?;
    layout
        .closed_body_parameter_source_entry_lookup(entry)
        .then_some(layout)
}

pub(in super::super) fn local_read_scope_is_excluded(
    context: &crate::var_resolve::ResolveContext,
    name: &str,
) -> bool {
    context.globals.contains(name)
        || context.ns_vars.contains(name)
        || context.upvar_aliases.contains_key(name)
        || context.instance_vars.contains(name)
        || context.traced.contains(name)
        || !context.untracked_traces.is_empty()
}

pub(in super::super) fn direct_declaration_operand_owner(
    owner: &super::super::SourceVariableEvaluationOwner,
    site: &CommandAllocationSite,
) -> bool {
    let mut pending = vec![owner];
    while let Some(owner) = pending.pop() {
        match owner {
            super::super::SourceVariableEvaluationOwner::InvocationArguments {
                invocation, ..
            } if invocation == site => {}
            super::super::SourceVariableEvaluationOwner::Alternatives(owners)
                if !owners.is_empty() =>
            {
                pending.extend(owners);
            }
            _ => return false,
        }
    }
    true
}
