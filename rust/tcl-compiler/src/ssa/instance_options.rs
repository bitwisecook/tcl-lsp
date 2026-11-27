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

//! Definitions and writes of instance configuration options linked to variables, mirrored onto the global root frame.

use crate::cfg;
use crate::ir::{CommandTokens, Statement};
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

/// Retain linked-input definitions from a registry-selected instance class.
pub(super) fn enrich_instance_option_defs(
    func: &cfg::Function,
    registry: &CommandRegistry,
) -> Option<cfg::Function> {
    let mut enriched = func.clone();
    let defs = enrich_instance_option_defs_with_initial(
        &mut enriched,
        registry,
        &crate::taint::InstanceClassState::new(),
    );
    (!defs.is_empty()).then_some(enriched)
}

pub(crate) fn linked_option_definition_name(
    tokens: &CommandTokens,
    index: usize,
    global: bool,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<String> {
    let binding = tokens.source_binding.as_ref()?;
    let purpose = binding.original_source_advice_purpose(tokens, metadata, registry)?;
    let input = binding.original_written_name_input(tokens, index.checked_add(1)?)?;
    let key = input.original_word_key()?;
    // Empty linked-input options disconnect; dollar signs in static values are data.
    let value = key.bytes();
    if value.is_empty() {
        return None;
    }
    let variable = (purpose != crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        .then(|| key.policy().recipe().combined_variable_input(value));
    // Logical words retain Unicode values; they are not a Native string pool.
    let root = variable.as_ref().map_or_else(
        || tcl_syntax::naming::split_element_ref_bytes(value).map_or(value, |(root, _)| root),
        |variable| variable.root().selected(),
    );
    let name = std::str::from_utf8(root).ok()?;
    Some(
        if global && tcl_syntax::naming::unroot_rooted_key(name).is_none() {
            tcl_syntax::naming::root_unrooted_key(name)
        } else {
            name.to_owned()
        },
    )
}

/// Selected source option metadata keeps its effective ordinal and genuine
/// written operand separately. This grants no physical definition or store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OriginalLinkedOptionWrite {
    pub(crate) effective_argument: usize,
    pub(crate) written_argument: usize,
    pub(crate) scope: tcl_registry::VariableScope,
}

fn selected_source_option_writes(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<Vec<(usize, tcl_registry::VariableScope)>> {
    schema.authored_source_argument_roles().1.then_some(())?;
    schema.semantics.options.prefix_occurrences(
        schema
            .words
            .arguments()
            .slice_from(schema.semantics.argument_offset),
    )?;
    Some(
        schema
            .authored_source_variable_name_arguments()
            .into_iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
            .filter_map(|(argument, _)| {
                Some((
                    argument,
                    schema.authored_source_option_variable_scope_at(argument)?,
                ))
            })
            .collect(),
    )
}

pub(crate) fn original_command_option_writes(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalLinkedOptionWrite>> {
    tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, metadata, registry)?;
    let advice =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    advice.unanimous_command_words()?;
    let mut selected = None;
    for shape in &advice.candidates {
        let writes =
            shape.with_source_schema(registry, metadata, tokens, selected_source_option_writes)?;
        let writes = writes
            .into_iter()
            .filter_map(|(effective_argument, scope)| {
                let crate::registry_invocation::InvocationWordOrigin::Written(written) = shape
                    .effective
                    .origins
                    .get(effective_argument.checked_add(1)?)?
                else {
                    return None;
                };
                Some(OriginalLinkedOptionWrite {
                    effective_argument,
                    written_argument: written.checked_sub(1)?,
                    scope,
                })
            })
            .collect::<Vec<_>>();
        if selected
            .as_ref()
            .is_some_and(|previous| previous != &writes)
        {
            return None;
        }
        selected = Some(writes);
    }
    selected
}

/// Selected command options whose current Registry schema specifically
/// declares both variable writes and tainted external input. Generic SSA
/// variable-write consumers keep using `original_command_option_writes`.
pub(crate) fn original_command_taint_option_writes(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalLinkedOptionWrite>> {
    fn selected(
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Vec<(usize, tcl_registry::VariableScope)>> {
        if !schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::TAINTS_VAR_WRITES)
        {
            return Some(Vec::new());
        }
        let offset = schema.semantics.argument_offset;
        let options = schema
            .semantics
            .options
            .prefix_occurrences(schema.words.arguments().slice_from(offset))?;
        let variable_writes = selected_source_option_writes(schema)?;
        Some(
            variable_writes
                .into_iter()
                .filter(|(argument, _)| {
                    argument.checked_sub(offset).is_some_and(|relative| {
                        options.iter().any(|occurrence| {
                            occurrence.values.contains(&relative)
                                && occurrence
                                    .option
                                    .is_some_and(tcl_registry::hover::OptionSpec::taints_var_write)
                        })
                    })
                })
                .collect(),
        )
    }

    tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, metadata, registry)?;
    let advice =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    advice.unanimous_command_words()?;
    let mut selected_writes = None;
    for shape in &advice.candidates {
        let writes = shape.with_source_schema(registry, metadata, tokens, selected)?;
        let writes = writes
            .into_iter()
            .filter_map(|(effective_argument, scope)| {
                let crate::registry_invocation::InvocationWordOrigin::Written(written) = shape
                    .effective
                    .origins
                    .get(effective_argument.checked_add(1)?)?
                else {
                    return None;
                };
                Some(OriginalLinkedOptionWrite {
                    effective_argument,
                    written_argument: written.checked_sub(1)?,
                    scope,
                })
            })
            .collect::<Vec<_>>();
        if selected_writes
            .as_ref()
            .is_some_and(|previous| previous != &writes)
        {
            return None;
        }
        selected_writes = Some(writes);
    }
    selected_writes
}

pub(crate) fn original_instance_option_writes(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<Vec<OriginalLinkedOptionWrite>> {
    let source = tokens
        .source_binding
        .as_ref()?
        .original_registered_instance_source_for_tokens(tokens, metadata, registry)?;
    let context = metadata
        .source_analysis_input()?
        .borrowed_context_registry();
    source
        .instance()
        .descriptor(context)?
        .traits
        .contains(tcl_registry::Traits::TAINTS_VAR_WRITES)
        .then_some(())?;
    let writes = source.with_source_schema(context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::CONFIGURES_INSTANCE_OPTIONS)
            .then_some(())?;
        let offset = schema.semantics.argument_offset;
        let options = schema
            .semantics
            .options
            .prefix_occurrences(schema.words.arguments().slice_from(offset))?;
        Some(
            selected_source_option_writes(schema)?
                .into_iter()
                .filter(|(argument, _)| {
                    argument.checked_sub(offset).is_some_and(|relative| {
                        options.iter().any(|occurrence| {
                            occurrence.values.contains(&relative)
                                && occurrence
                                    .option
                                    .is_some_and(tcl_registry::hover::OptionSpec::taints_var_write)
                        })
                    })
                })
                .collect::<Vec<_>>(),
        )
    })??;
    let mut selected = Vec::new();
    for (effective_argument, scope) in writes {
        let word = source.argument_word(effective_argument)?;
        if word.group().expand {
            return None;
        }
        // A captured prefix keeps its earlier source word. Only the actual
        // current complete vector can supply this call's written operand.
        let mut written = source
            .original_words()
            .iter()
            .enumerate()
            .filter_map(|(written, original)| (original == word).then_some(written));
        let Some(index) = written.next() else {
            continue;
        };
        if written.next().is_some() {
            return None;
        }
        selected.push(OriginalLinkedOptionWrite {
            effective_argument,
            written_argument: index.checked_sub(1)?,
            scope,
        });
    }
    Some(selected)
}

pub(super) fn original_global_root_frame(
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
) -> Option<()> {
    let binding = tokens.source_binding.as_ref()?;
    binding.original_source_advice_purpose(tokens, metadata, registry)?;
    if binding.variable_frame.layout() != &tcl_names::variable::VariableExecutionFrame::Global {
        return None;
    }
    let namespace = binding.variable_context.namespace_identity.as_ref()?;
    namespace.exact_native_path()?.is_root().then_some(())
}

// Bare aliases require the original global frame and root namespace, not a body label.
pub(super) fn mirror_top_level_global_option_defs(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
) -> bool {
    let Some(crate::registry_invocation::InvocationMetadataSelection::Retained(metadata)) =
        func.metadata_context.metadata_context(registry)
    else {
        return false;
    };
    let mut changed = false;
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                command,
                args,
                defs,
                tokens,
                ..
            } = statement
            else {
                continue;
            };
            let Some(tokens) = tokens.as_ref() else {
                continue;
            };
            if tokens.argv_texts.first() != Some(command)
                || tokens.argv_texts.get(1..) != Some(args.as_slice())
            {
                continue;
            }
            if original_global_root_frame(tokens, metadata, registry).is_none() {
                continue;
            }
            let Some(writes) = original_command_option_writes(tokens, metadata, registry) else {
                continue;
            };
            let aliases: Vec<String> = writes
                .into_iter()
                .filter(|write| write.scope == tcl_registry::VariableScope::Global)
                .filter_map(|write| {
                    let global = linked_option_definition_name(
                        tokens,
                        write.written_argument,
                        true,
                        metadata,
                        registry,
                    )?;
                    let bare = tcl_syntax::naming::unroot_rooted_key(&global)?;
                    (!tcl_syntax::naming::is_qualified(bare.as_bytes()) && defs.contains(&global))
                        .then_some(bare.to_owned())
                })
                .filter(|alias| !defs.contains(alias))
                .collect();
            changed |= !aliases.is_empty();
            defs.extend(aliases);
        }
    }
    changed
}

/// Retain source option definitions under the genuine selected instance schema.
/// Physical SSA definitions still require the independent Normal projection in
/// `defs_of_in_context`; this source metadata supplies no such permission.
pub(crate) fn enrich_instance_option_defs_with_initial(
    func: &mut cfg::Function,
    registry: &CommandRegistry,
    _initial: &crate::taint::InstanceClassState,
) -> HashSet<String> {
    let Some(crate::registry_invocation::InvocationMetadataSelection::Retained(metadata)) =
        func.metadata_context.metadata_context(registry)
    else {
        return HashSet::new();
    };
    let mut instance_defs = HashSet::new();
    for block in func.blocks.values_mut() {
        for statement in &mut block.statements {
            let Statement::Call {
                command,
                args,
                defs,
                tokens,
                ..
            } = statement
            else {
                continue;
            };
            let Some(tokens) = tokens.as_ref() else {
                continue;
            };
            if tokens.argv_texts.first() != Some(command)
                || tokens.argv_texts.get(1..) != Some(args.as_slice())
            {
                continue;
            }
            let Some(writes) = original_instance_option_writes(tokens, metadata, registry) else {
                continue;
            };
            for write in writes {
                let global = write.scope == tcl_registry::VariableScope::Global;
                let Some(name) = linked_option_definition_name(
                    tokens,
                    write.written_argument,
                    global,
                    metadata,
                    registry,
                ) else {
                    continue;
                };
                instance_defs.insert(name.clone());
                if !defs.contains(&name) {
                    defs.push(name.clone());
                }
                if global
                    && original_global_root_frame(tokens, metadata, registry).is_some()
                    && let Some(bare) = tcl_syntax::naming::unroot_rooted_key(&name)
                    && !tcl_syntax::naming::is_qualified(bare.as_bytes())
                {
                    instance_defs.insert(bare.to_owned());
                    if !defs.iter().any(|defined| defined == bare) {
                        defs.push(bare.to_owned());
                    }
                }
            }
        }
    }
    instance_defs
}
