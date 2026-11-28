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

//! Readonly object-reference candidates from the shared original source context.
//! A literal operand names its object; the current-source entry points also
//! take the one string constant propagation settled for a `$name` operand. No
//! live handler, evaluated target or deletion-safety proof is issued here.

use std::collections::HashSet;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

fn category_for_kinds(kinds: &[&str]) -> IrulesObjectReferenceCategory {
    let tables = crate::tables();
    if kinds.iter().any(|kind| {
        tables
            .ltm_pool_kinds
            .iter()
            .chain(&tables.gtm_pool_kinds)
            .any(|candidate| candidate == kind)
    }) {
        IrulesObjectReferenceCategory::Pool
    } else if kinds.iter().any(|kind| {
        tables
            .data_group_kinds
            .iter()
            .any(|candidate| candidate == kind)
    }) {
        IrulesObjectReferenceCategory::DataGroup
    } else {
        IrulesObjectReferenceCategory::Other
    }
}

/// Configuration-reference presentation category; no object identity is issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesObjectReferenceCategory {
    Pool,
    DataGroup,
    Other,
}

/// One readonly literal source-reference candidate, with original geometry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesObjectReference {
    /// Actual supported literal source value, not a propagated runtime value.
    pub name: String,
    /// Shared declarative object-kind candidates.
    pub kinds: Vec<&'static str>,
    /// Actual selected source-schema label for presentation.
    pub command: String,
    /// Source-schema label; this field supplies no live command identity.
    pub effective_command: String,
    /// Category derived from the exact shared object-kind template.
    pub category: IrulesObjectReferenceCategory,
    /// Original post-head argument ordinal.
    pub argument_index: usize,
    /// Genuine original operand source extent.
    pub range: Span,
}

/// Literal source-reference candidates from an independently retained context.
#[must_use]
pub fn extract_original_irules_source_object_references(
    source: &str,
    context: &crate::OriginalIrulesSourceContext,
    rule_module: Option<&str>,
) -> Vec<IrulesObjectReference> {
    source_object_references(source, context, rule_module, OperandValues::Literal)
}

/// Which operand values name an object.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OperandValues {
    /// Only a value the operand word spells itself.
    Literal,
    /// Also the one string constant propagation settled for `$name`.
    Propagated,
}

#[allow(clippy::too_many_lines)]
fn source_object_references(
    source: &str,
    context: &crate::OriginalIrulesSourceContext,
    rule_module: Option<&str>,
    values: OperandValues,
) -> Vec<IrulesObjectReference> {
    // Implementation contract: naming.consumer.original-source-attachment-candidates
    // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
    if !context.matches_source(source) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for command in context.commands() {
        let Some(operands) = command
            .words()
            .with_source_schema(context.context_registry(), |schema| {
                crate::original_source_object_operands(schema, rule_module)
            })
        else {
            continue;
        };
        if let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Vendor(
            metadata,
        ) = command.words().source()
            && metadata
                .authored_barriers()
                .has_nominal_command_name_hazard()
        {
            let words = command.words();
            let original_projection = |argument: usize, kinds: &[&'static str], attachment| {
                Some((
                    argument,
                    *words.origins().get(argument.checked_add(1)?)?,
                    words.original_argument_word(argument)?.clone(),
                    kinds.to_vec(),
                    attachment,
                ))
            };
            let expected = operands
                .iter()
                .map(|operand| {
                    original_projection(operand.argument(), operand.kinds(), operand.attachment())
                })
                .collect::<Option<Vec<_>>>();
            let Some(expected) = expected else {
                continue;
            };
            let alternatives = metadata.authored_barriers().nominal_alias_source_projections(
                context.context_registry(),
                words,
                |schema, origins| {
                    crate::original_source_object_operands(schema, rule_module).iter().map(|operand| {
                        let tcl_compiler::registry_invocation::NominalAliasSourceOperandOrigin::InvocationArgument(argument) = origins.get(operand.argument())? else {
                            return None;
                        };
                        original_projection(*argument, operand.kinds(), operand.attachment())
                    }).collect::<Option<Vec<_>>>()
                },
            );
            if !alternatives.is_some_and(|alternatives| {
                alternatives
                    .iter()
                    .all(|alternative| alternative == &expected)
            }) {
                continue;
            }
        }
        for operand in operands {
            let ordinal = operand.argument();
            let words = command.words();
            let origin = ordinal
                .checked_add(1)
                .and_then(|index| words.origins().get(index));
            let value = match origin {
                Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(_)) => words
                    .original_argument_word(ordinal)
                    .and_then(tcl_syntax::word_rules::original_static_word_unicode_value)
                    .or_else(|| {
                        if values != OperandValues::Propagated {
                            return None;
                        }
                        let operand = words.operands().get(ordinal)?.as_ref()?.span();
                        Some(
                            context
                                .propagated_operand_value(operand)?
                                .as_bytes()
                                .to_vec(),
                        )
                    }),
                Some(tcl_compiler::registry_invocation::InvocationWordOrigin::BindingPrefix(_)) => {
                    words
                        .arguments()
                        .get(ordinal)
                        .and_then(|value| value.literal_bytes())
                        .map(<[u8]>::to_vec)
                }
                _ => None,
            };
            let Some(name) = value
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .filter(|name| !name.contains('\0'))
            else {
                continue;
            };
            let Some(range) = command.words().operands().get(ordinal).and_then(Option::as_ref).map(tcl_compiler::registry_invocation::source_structure::OriginalOperandSource::span) else { continue; };
            let label = command.words().command().to_owned();
            out.push(IrulesObjectReference {
                name,
                kinds: operand.kinds().to_vec(),
                command: label.clone(),
                effective_command: label,
                category: category_for_kinds(operand.kinds()),
                argument_index: ordinal,
                range,
            });
        }
    }
    out.sort_by(|a, b| {
        (a.range.start(), a.range.end(), &a.name).cmp(&(b.range.start(), b.range.end(), &b.name))
    });
    out.dedup();
    out
}

/// Source-name ranges for presentation; no runtime object lookup is performed.
#[must_use]
pub fn object_ref_spans(source: &str, registry: &CommandRegistry) -> Vec<Span> {
    let mut spans = extract_irules_object_references(source, None, registry)
        .into_iter()
        .map(|reference| reference.range)
        .collect::<Vec<_>>();
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

/// Current event-associated literal source-reference candidates.
#[must_use]
pub fn extract_irules_object_references(
    source: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
) -> Vec<IrulesObjectReference> {
    crate::OriginalIrulesSourceContext::capture(source, registry).map_or_else(Vec::new, |context| {
        source_object_references(source, &context, rule_module, OperandValues::Propagated)
    })
}

/// Literal candidates associated with an actual current event source descriptor.
#[must_use]
pub fn extract_irules_event_object_references(
    source: &str,
    event: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
) -> Vec<IrulesObjectReference> {
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    let spans = context
        .commands()
        .iter()
        .filter(|command| command.event_source().event().eq_ignore_ascii_case(event))
        .map(crate::source_context::OriginalIrulesSourceCommand::span)
        .collect::<Vec<_>>();
    source_object_references(source, &context, rule_module, OperandValues::Propagated)
        .into_iter()
        .filter(|reference| {
            spans.iter().any(|span| {
                span.start() <= reference.range.start() && reference.range.end() <= span.end()
            })
        })
        .collect()
}

/// Compatibility source-range filter. DTO labels never select source schema;
/// every accepted extent is revalidated against the genuine current inventory.
#[must_use]
pub fn extract_irules_object_references_in_closure(
    source: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
    executable: &[crate::IrulesExecutableCommand],
) -> Vec<IrulesObjectReference> {
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    let selected = executable
        .iter()
        .map(|command| command.span)
        .collect::<HashSet<_>>();
    let spans = context
        .commands()
        .iter()
        .filter(|command| selected.contains(&command.span()))
        .map(crate::source_context::OriginalIrulesSourceCommand::span)
        .collect::<Vec<_>>();
    extract_original_irules_source_object_references(source, &context, rule_module)
        .into_iter()
        .filter(|reference| {
            spans.iter().any(|span| {
                span.start() <= reference.range.start() && reference.range.end() <= span.end()
            })
        })
        .collect()
}
