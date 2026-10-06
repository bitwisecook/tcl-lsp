// SPDX-License-Identifier: AGPL-3.0-or-later
//! Script-position timing from the retained selected invocation, without text
//! stand-ins for unknown operand objects or execution/admission permission.

use crate::hover::ScriptTiming;
use crate::spec::{CommandSpec, SubCommand};
use crate::{ArgRole, CommandPrefixArguments, InvocationArguments, ResolvedInvocation};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectedScriptMetadata<'r> {
    pub(crate) command: &'r CommandSpec,
    pub(crate) subcommand: Option<&'r SubCommand>,
}

impl SelectedScriptMetadata<'_> {
    pub(crate) fn deferred_arguments(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        roles: &[(u8, ArgRole)],
        roles_complete: bool,
    ) -> Option<Vec<usize>> {
        if !roles_complete
            || !invocation
                .argument_count_for_arity()
                .is_some_and(|count| invocation.semantics.arity.accepts(count))
            || !matches!(
                invocation.subcommand,
                crate::resolved_invocation::SubcommandResolution::NotApplicable
                    | crate::resolved_invocation::SubcommandResolution::Exact(_)
                    | crate::resolved_invocation::SubcommandResolution::UniquePrefix(_)
            )
        {
            return None;
        }
        let arguments = invocation.words.arguments();
        let count = arguments.exact_argv_len()?;
        let offset = invocation.semantics.argument_offset;
        if roles.is_empty()
            && invocation
                .semantics
                .arg_role_resolver_roles
                .iter()
                .any(|role| role.has_script_timing())
        {
            // A concatenated or otherwise unclassified script retains its
            // deferred payload obligation even without one operand position.
            return None;
        }
        let selected = arguments.slice_from(offset);
        let mut positions: Vec<usize> = roles
            .iter()
            .filter(|(_, role)| role.has_script_timing())
            .filter_map(|(index, _)| offset.checked_add(usize::from(*index)))
            .collect();
        let (prefixes, prefix_resolver, timing_resolver) = self.descriptors();
        if let Some(resolver) = prefix_resolver {
            // Legacy value-sensitive descriptors get only complete actual values.
            let values = selected.literal_values()?;
            positions.extend(
                resolver(CommandPrefixArguments::literals(&values))
                    .into_iter()
                    .filter_map(|(index, _)| offset.checked_add(usize::from(index))),
            );
        } else {
            positions.extend(
                prefixes
                    .iter()
                    .filter_map(|(index, _)| offset.checked_add(usize::from(*index))),
            );
        }
        let default = if invocation
            .semantics
            .traits
            .contains(crate::traits::Traits::DEFERS_BODY)
        {
            ScriptTiming::Deferred
        } else {
            ScriptTiming::SameInvocation
        };
        let timings = if let Some(resolver) = timing_resolver {
            let values = selected.literal_values()?;
            Some(resolver(&values))
        } else {
            None
        };
        positions.retain(|&index| {
            index < count
                && timings.as_ref().map_or(default, |timings| {
                    timings
                        .iter()
                        .find(|(at, _)| offset + usize::from(*at) == index)
                        .map_or(default, |(_, timing)| *timing)
                }) == ScriptTiming::Deferred
        });
        extend_option_timings(
            &mut positions,
            invocation,
            selected,
            offset,
            timings.as_deref(),
        )?;
        positions.sort_unstable();
        positions.dedup();
        Some(positions)
    }
    fn descriptors(
        self,
    ) -> (
        &'static [(u8, crate::arg_role::AppendedArity)],
        Option<crate::spec::CommandPrefixResolver>,
        Option<crate::spec::ScriptTimingResolver>,
    ) {
        self.subcommand.map_or(
            (
                self.command.command_prefixes,
                self.command.command_prefix_resolver,
                self.command.script_timing_resolver,
            ),
            |sub| {
                (
                    sub.command_prefixes,
                    sub.command_prefix_resolver,
                    sub.script_timing_resolver,
                )
            },
        )
    }
}

fn extend_option_timings(
    positions: &mut Vec<usize>,
    invocation: &ResolvedInvocation<'_, '_>,
    arguments: InvocationArguments<'_>,
    offset: usize,
    timings: Option<&[(u8, ScriptTiming)]>,
) -> Option<()> {
    let options = invocation.semantics.options;
    let end = options.leading_word_count(arguments)?;
    let available: Vec<_> = options.available().collect();
    let mut index = options.positional_prefix_words;
    while index < end {
        let word = arguments.literal_at(index)?;
        let option = crate::spec::resolve_available_option_prefix_with(
            &available,
            word,
            options.prefix_matching,
        )?;
        let width = option.value_word_count_for_arguments(arguments, index)?;
        if let Some(timing) = option.value_script_timing() {
            for at in index + 1..index + 1 + width {
                let absolute = offset + at;
                positions.retain(|&position| position != absolute);
                let selected = timings
                    .and_then(|timings| {
                        timings
                            .iter()
                            .find(|(position, _)| usize::from(*position) == at)
                    })
                    .map_or(timing, |(_, timing)| *timing);
                if selected == ScriptTiming::Deferred {
                    positions.push(absolute);
                }
            }
        }
        index += 1 + width;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use crate::{
        InvocationWord::{Dynamic, Expanded, Literal},
        InvocationWords,
    };

    #[test]
    fn selected_deferred_positions_accept_unknown_values_without_invented_text() {
        let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
        for (head, arguments, expected) in [
            ("after", vec![Literal("idle"), Dynamic], Some(vec![1])),
            ("after", vec![Literal("1000"), Dynamic], Some(vec![1])),
            ("after", vec![Literal("idle"), Dynamic, Dynamic], None),
            ("after", vec![Dynamic, Dynamic], None),
            ("after", vec![Literal("idle"), Expanded], None),
            (
                "fileevent",
                vec![Dynamic, Literal("readable"), Dynamic],
                Some(vec![2]),
            ),
            ("subst", vec![Dynamic], Some(vec![])),
        ] {
            let invocation = registry.resolve_structured_invocation(
                InvocationWords::structured(Literal(head), &arguments),
                registry.own_surface_query(),
            );
            assert_eq!(
                invocation
                    .resolved()
                    .unwrap()
                    .facts()
                    .deferred_script_argument_indices(),
                expected.as_deref(),
                "{head} {arguments:?}"
            );
        }
    }
}
