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

//! Command references for the procedure names held in dispatch-table literals.

use crate::analyser::state::Analyser;
use std::collections::{HashMap, HashSet};

impl Analyser {
    /// Emit a command reference for each proc-name **literal**
    /// held as a dispatch-table value, provided the table is actually
    /// *consumed* by a `$table(...)` / `[dict get $table …]` dispatch site
    /// this pass walks (the W307 shapes).  The reference anchors at the
    /// literal value token itself — the span **is** the written command name,
    /// so rename may rewrite the table entry, keeping the dispatch alive.
    /// Values with no recoverable span (pure SCCP folds, e.g. through
    /// `string map`) abstain, as does an unconsumed table (a config array
    /// whose values merely look like command names must not gain phantom
    /// references).
    pub(super) fn emit_dispatch_table_command_references(
        &mut self,
        cu: &crate::compilation_unit::CompilationUnit,
    ) {
        let consumed = self
            .var_command_sites
            .iter()
            .filter(|site| site.receiver != crate::analyser::state::DispatchReceiver::SelfDispatch)
            .map(|site| site.cmd_span.start())
            .chain(
                self.cmd_command_sites
                    .iter()
                    .map(|site| site.cmd_span.start()),
            )
            .collect::<HashSet<_>>();
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_TABLE_PROOF_DEBUG").is_some() {
            eprintln!("table consumption sites={consumed:?}");
        }
        let Some(input) = self.result.resolved_input.as_ref() else {
            return;
        };
        let (harvested, divergent) = harvest_table_command_value_spans(
            cu,
            &self.source,
            &input.context_registry(),
            input.lexer_config(),
            &consumed,
        );
        // A literal whose consuming sites reach different implementations
        // keeps no rename authority, including a row another producer adds.
        self.divergent_table_literals.extend(divergent);
        for (value, span, reference) in harvested {
            if !reference.is_user_command()
                || self.result.command_invocations.iter().any(|existing| {
                    existing.range == span
                        && existing.resolved_command_reference.as_ref() == Some(&reference)
                        && existing.resolved_user_definition == reference.is_direct_definition()
                })
            {
                continue;
            }
            let mut invocation = crate::signature_scan::types::SignatureCommandInvocation::written(
                value, span, None,
            );
            invocation.retain_reference(&reference);
            self.result.command_invocations.push(invocation);
        }
    }
}

/// One dispatch-table value literal and the implementation every consuming
/// site reaches through it.
type TableCommandValue = (
    String,
    tcl_lexer::Span,
    crate::command_binding::SourceCommandReference,
);

/// Values are harvested only through original reached consumption carriers and
/// exact physical contents ancestry. A literal's command lookup occurs in the
/// consuming invocation's post-argv world, rather than its producer's namespace.
fn harvest_table_command_value_spans(
    cu: &crate::compilation_unit::CompilationUnit,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    config: tcl_lexer::LexerConfig,
    consumed: &HashSet<u32>,
) -> (Vec<TableCommandValue>, HashSet<(u32, u32)>) {
    use crate::ir::WordExpr;
    if consumed.is_empty() {
        return (Vec::new(), HashSet::new());
    }
    let mut selected = HashMap::new();
    for unit in cu.all_body_function_units() {
        for (&block, body) in &unit.ssa.blocks {
            for index in 0..body.statements.len() {
                let view = crate::ssa::SsaSourceView::at_statement(&unit.ssa, block, index);
                let Some(tokens) = view
                    .source_tokens()
                    .filter(|tokens| tokens.synthetic.is_none())
                else {
                    continue;
                };
                let Some(head) = tokens.words().first() else {
                    continue;
                };
                let expanded = matches!(head, WordExpr::Expand { .. });
                let written_start = unit.abs_span(head.source().span).start();
                let head = if let WordExpr::Expand { word, .. } = head {
                    word.as_ref()
                } else {
                    head
                };
                if !consumed.contains(&written_start)
                    && !consumed.contains(&unit.abs_span(head.source().span).start())
                {
                    continue;
                }
                let Some(binding) = &tokens.source_binding else {
                    continue;
                };
                let read = dispatch_table_values(cu, unit, tokens, head, source, context, config);
                for value in read.into_iter().flatten() {
                    let Some(value) = dispatch_table_head(value, expanded, binding) else {
                        continue;
                    };
                    let Some(span) = value.literal_span else {
                        continue;
                    };
                    let reference = binding.command_reference(&value.value);
                    let key = (span.start(), span.end());
                    let entry = selected
                        .entry(key)
                        .or_insert_with(|| (value.value.clone(), span, reference.clone()));
                    if entry.0 != value.value || entry.2 != reference {
                        entry.2 = None;
                    }
                }
            }
        }
    }
    let mut rows = Vec::new();
    let mut divergent = HashSet::new();
    for (value, span, reference) in selected.into_values() {
        match reference {
            Some(reference) => rows.push((value, span, reference)),
            None => {
                divergent.insert((span.start(), span.end()));
            }
        }
    }
    (rows, divergent)
}

fn dispatch_table_values(
    compilation: &crate::compilation_unit::CompilationUnit,
    unit: &crate::compilation_unit::FunctionUnit,
    tokens: &crate::ir::CommandTokens,
    head: &crate::ir::WordExpr,
    source: &str,
    context: &tcl_registry::model::ContextRegistry,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<crate::table_value_provenance::TableValueContributor>> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Availability and lexer overrides remain separate from reached cell contents.
    let registry = context.commands();
    if config.nested().normalized() != unit.source_lexer_config().nested().normalized() {
        return None;
    }
    if head.sole_variable_substitution().is_some() {
        return crate::table_value_provenance::table_values(
            compilation,
            unit,
            tokens,
            head,
            None,
            source,
            registry,
        );
    }
    let module = &compilation.ir_module;
    let owner = module.retained_source_bindings.as_deref()?;
    if !owner.matches_module(module, registry) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let metadata = if module.source_metadata_input.is_some() {
        unit.invocation_metadata_context_for_module(registry, module)?;
        Some(binding.original_invocation_metadata_for_module(tokens, module, registry)?)
    } else {
        if !module.source_entry.metadata_context.is_standalone()
            || !owner.owns_original_tokens(tokens)
        {
            return None;
        }
        binding
            .original_invocation_metadata_for_function(tokens, unit, registry)?
            .context()
    };
    let config = unit.source_lexer_config();
    let calls =
        if let Some(actual) = metadata.filter(|actual| actual.source_analysis_input().is_some()) {
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens, config, registry, actual,
            )?
        } else {
            crate::word_subst::checked_lifted_calls(tokens, config)?
        };
    let mut nested = crate::word_subst::whole_word_command_tokens(head, config)?;
    nested.inherit_nested_bindings(tokens);
    if !calls
        .iter()
        .any(|call| call.tokens.as_ref() == Some(&nested))
    {
        return None;
    }
    let nested_binding = nested.source_binding.as_ref()?;
    let metadata = if module.source_metadata_input.is_some() {
        Some(nested_binding.original_invocation_metadata_for_module(&nested, module, registry)?)
    } else {
        if !owner.owns_original_tokens(&nested) {
            return None;
        }
        nested_binding
            .original_invocation_metadata_for_function(&nested, unit, registry)?
            .context()
    };
    let (input, keys) =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, metadata, &nested,
        )?
        .dictionary_lookup()?;
    if keys.is_empty() {
        return None;
    }
    crate::table_value_provenance::table_values(
        compilation,
        unit,
        &nested,
        &input,
        Some(&keys),
        source,
        registry,
    )
}

fn dispatch_table_head(
    value: crate::table_value_provenance::TableValueContributor,
    expanded: bool,
    binding: &crate::command_binding::SourceInvocationBinding,
) -> Option<crate::value_provenance::ValueContributor> {
    let mut value = match value {
        crate::table_value_provenance::TableValueContributor::ListHead(value) => {
            return expanded.then_some(value);
        }
        crate::table_value_provenance::TableValueContributor::SingletonList(value) => {
            return (expanded
                || tcl_syntax::list::join_list([value.value.as_str()]) == value.value)
                .then_some(value);
        }
        crate::table_value_provenance::TableValueContributor::Prefix(value) => value,
    };
    if !expanded {
        return Some(value);
    }
    let rules = binding.variable_context.invocation_dialect?.word_values;
    if rules.split_list(&value.value).ok()?.is_empty() {
        return None;
    }
    let element =
        tcl_syntax::list::find_element_with_syntax(&value.value, 0, rules.list).ok()??;
    if !element.literal {
        return None;
    }
    let base = value.literal_span?.start();
    value.literal_span = Some(tcl_lexer::Span::new(
        base.checked_add(u32::try_from(element.value.start).ok()?)?,
        base.checked_add(u32::try_from(element.value.end).ok()?)?,
    ));
    value.value.get(element.value.clone())?;
    value.value.truncate(element.value.end);
    value.value.replace_range(..element.value.start, "");
    Some(value)
}
