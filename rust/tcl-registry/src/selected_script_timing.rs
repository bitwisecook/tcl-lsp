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
    /// Timing is independent of whether an ordinal is an owned script role.
    /// Case descriptors supply their own script-container positions.
    pub(crate) fn timing_at(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        index: usize,
    ) -> Option<ScriptTiming> {
        let arguments = invocation.words.arguments();
        let count = arguments.exact_argv_len()?;
        let offset = invocation.semantics.argument_offset;
        if index < offset
            || index >= count
            || !invocation
                .semantics
                .arity
                .accepts(invocation.argument_count_for_arity()?)
        {
            return None;
        }
        let default = if invocation
            .semantics
            .traits
            .contains(crate::Traits::DEFERS_BODY)
        {
            ScriptTiming::Deferred
        } else {
            ScriptTiming::SameInvocation
        };
        let (_, _, resolver) = self.descriptors();
        let timings = resolver.map(|resolver| resolver(arguments.slice_from(offset)));
        let timing = timings
            .as_ref()
            .and_then(|timings| {
                timings
                    .iter()
                    .find(|(at, _)| offset + usize::from(*at) == index)
            })
            .map_or(default, |(_, timing)| *timing);
        let mut positions = vec![(index, timing)];
        extend_option_timings(
            &mut positions,
            invocation,
            arguments.slice_from(offset),
            offset,
            timings.as_deref(),
        )?;
        positions
            .iter()
            .find(|(at, _)| *at == index)
            .map(|(_, timing)| *timing)
    }

    pub(crate) fn deferred_arguments(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        roles: &[(u8, ArgRole)],
        roles_complete: bool,
    ) -> Option<Vec<usize>> {
        Some(
            self.script_arguments(invocation, roles, roles_complete)?
                .into_iter()
                .filter_map(|(index, timing)| (timing == ScriptTiming::Deferred).then_some(index))
                .collect(),
        )
    }

    pub(crate) fn lookup_arguments(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        roles: &[(u8, ArgRole)],
        roles_complete: bool,
    ) -> Option<Vec<(usize, crate::ScriptLookupScope)>> {
        let scope = self
            .subcommand
            .and_then(|sub| sub.script_lookup_scope)
            .or(self.command.script_lookup_scope)?;
        Some(
            self.script_arguments(invocation, roles, roles_complete)?
                .into_iter()
                .filter_map(|(index, timing)| {
                    (timing != ScriptTiming::ReferenceOnly).then_some((index, scope))
                })
                .collect(),
        )
    }

    pub(crate) fn prefix_arguments(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        roles: &[(u8, ArgRole)],
        roles_complete: bool,
    ) -> Option<Vec<(usize, crate::AppendedArity)>> {
        let executable = self.script_arguments(invocation, roles, roles_complete)?;
        let arguments = invocation.words.arguments();
        let offset = invocation.semantics.argument_offset;
        let selected = arguments.slice_from(offset);
        let (prefixes, resolver, _) = self.descriptors();
        let mut positions = if let Some(resolver) = resolver {
            resolver(CommandPrefixArguments::from_invocation_arguments(selected))
                .into_iter()
                .filter_map(|(index, arity)| {
                    offset
                        .checked_add(usize::from(index))
                        .map(|index| (index, arity))
                })
                .collect::<Vec<_>>()
        } else {
            prefixes
                .iter()
                .filter_map(|&(index, arity)| {
                    offset
                        .checked_add(usize::from(index))
                        .map(|index| (index, arity))
                })
                .collect()
        };
        let options = invocation.semantics.options;
        let end = options.leading_word_count(selected)?;
        let available: Vec<_> = options.available().collect();
        let mut index = options.positional_prefix_words;
        while index < end {
            let option = crate::spec::resolve_available_option_prefix_with(
                &available,
                selected.literal_at(index)?,
                options.prefix_matching,
            )?;
            let width = option.value_word_count_for_arguments(selected, index)?;
            if option.value_role() == Some(ArgRole::CommandPrefix) {
                positions.extend(
                    (index + 1..index + 1 + width)
                        .map(|at| (offset + at, option.value_appended_arity())),
                );
            }
            index += 1 + width;
        }
        positions.retain(|(index, _)| {
            executable
                .iter()
                .any(|&(at, timing)| at == *index && timing != ScriptTiming::ReferenceOnly)
        });
        positions.sort_unstable_by_key(|&(index, _)| index);
        positions.dedup();
        Some(positions)
    }

    pub(crate) fn plain_script_arguments(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        roles: &[(u8, ArgRole)],
        roles_complete: bool,
    ) -> Option<Vec<usize>> {
        let executable = self.script_arguments(invocation, roles, roles_complete)?;
        let prefixes = self.prefix_arguments(invocation, roles, roles_complete)?;
        let offset = invocation.semantics.argument_offset;
        let option_roles = invocation
            .semantics
            .options
            .value_roles(invocation.words.arguments().slice_from(offset))?;
        Some(
            executable
                .into_iter()
                .filter_map(|(index, timing)| {
                    let plain_role = |role| matches!(role, ArgRole::Body | ArgRole::CommandPrefix);
                    let declared = roles.iter().any(|&(at, role)| {
                        offset.checked_add(usize::from(at)) == Some(index) && plain_role(role)
                    }) || option_roles.iter().any(|&(at, role)| {
                        offset.checked_add(at) == Some(index) && plain_role(role)
                    }) || prefixes.iter().any(|&(at, _)| at == index);
                    (declared && timing != ScriptTiming::ReferenceOnly).then_some(index)
                })
                .collect(),
        )
    }

    pub(crate) fn script_arguments(
        self,
        invocation: &ResolvedInvocation<'_, '_>,
        roles: &[(u8, ArgRole)],
        roles_complete: bool,
    ) -> Option<Vec<(usize, ScriptTiming)>> {
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
            positions.extend(
                resolver(CommandPrefixArguments::from_invocation_arguments(selected))
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
        let timings = timing_resolver.map(|resolver| resolver(selected));
        let mut positions: Vec<_> = positions
            .into_iter()
            .filter(|&index| index < count)
            .map(|index| {
                let timing = timings.as_ref().map_or(default, |timings| {
                    timings
                        .iter()
                        .find(|(at, _)| offset + usize::from(*at) == index)
                        .map_or(default, |(_, timing)| *timing)
                });
                (index, timing)
            })
            .collect();
        extend_option_timings(
            &mut positions,
            invocation,
            selected,
            offset,
            timings.as_deref(),
        )?;
        positions.sort_unstable_by_key(|&(index, _)| index);
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
    positions: &mut Vec<(usize, ScriptTiming)>,
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
                positions.retain(|&(position, _)| position != absolute);
                let selected = timings
                    .and_then(|timings| {
                        timings
                            .iter()
                            .find(|(position, _)| usize::from(*position) == at)
                    })
                    .map_or(timing, |(_, timing)| *timing);
                positions.push((absolute, selected));
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
    fn authored_timing_keeps_unknown_payloads_and_refuses_unknown_layouts() {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let context = crate::model::ingress::static_context_for(name);
            let registry = context.commands();
            for (arguments, expected) in [
                (
                    vec![
                        Literal("add"),
                        Literal("variable"),
                        Dynamic,
                        Literal("write"),
                        Dynamic,
                    ],
                    Some(vec![(4, crate::AppendedArity::Exactly(3))]),
                ),
                (
                    vec![
                        Literal("add"),
                        Literal("execution"),
                        Dynamic,
                        Literal("enter leave"),
                        Dynamic,
                    ],
                    Some(vec![(
                        4,
                        crate::AppendedArity::OneOf(crate::AppendedAritySet::from_sorted_unique(
                            &[2, 4],
                        )),
                    )]),
                ),
                (
                    vec![
                        Literal("remove"),
                        Literal("variable"),
                        Dynamic,
                        Literal("write"),
                        Dynamic,
                    ],
                    Some(vec![]),
                ),
                (
                    vec![
                        Literal("add"),
                        Dynamic,
                        Literal("v"),
                        Literal("write"),
                        Dynamic,
                    ],
                    None,
                ),
                (
                    vec![
                        Literal("add"),
                        Literal("variable"),
                        Literal("v"),
                        Literal("write"),
                        Expanded,
                    ],
                    None,
                ),
            ] {
                let resolution = registry.resolve_structured_invocation(
                    InvocationWords::structured(Literal("trace"), &arguments)
                        .with_profile(Some(context.commands().profile().unwrap())),
                    registry.own_surface_query(),
                );
                let selected = resolution.resolved().unwrap();
                assert_eq!(
                    selected.authored_source_command_prefix_arguments(),
                    expected,
                    "{name}: {arguments:?}"
                );
                assert_eq!(
                    selected.authored_source_script_arguments(),
                    expected.as_ref().map(|positions| positions
                        .iter()
                        .map(|&(ordinal, _)| ordinal)
                        .collect::<Vec<_>>()),
                    "selected script timing: {name}: {arguments:?}"
                );
                assert_eq!(
                    selected.authored_source_plain_script_arguments(),
                    expected.as_ref().map(|positions| positions
                        .iter()
                        .map(|&(ordinal, _)| ordinal)
                        .collect::<Vec<_>>()),
                    "selected plain script timing: {name}: {arguments:?}"
                );
            }
        }
    }

    #[test]
    fn authored_plain_scripts_keep_lambda_lists_separate_from_bodies_and_prefixes() {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let context = crate::model::ingress::static_context_for(name);
            let registry = context.commands();
            for (head, arguments, generic, plain) in [
                ("apply", vec![Dynamic], vec![0], Vec::new()),
                ("uplevel", vec![Literal("#0"), Dynamic], vec![1], vec![1]),
                ("after", vec![Literal("idle"), Dynamic], vec![1], vec![1]),
            ] {
                let resolution = registry.resolve_structured_invocation(
                    InvocationWords::structured(Literal(head), &arguments)
                        .with_profile(Some(registry.profile().unwrap())),
                    registry.own_surface_query(),
                );
                let schema = resolution.resolved().unwrap();
                assert_eq!(
                    schema.authored_source_script_arguments(),
                    Some(generic),
                    "{name}: {head}"
                );
                assert_eq!(
                    schema.authored_source_plain_script_arguments(),
                    Some(plain),
                    "{name}: {head}"
                );
            }
        }
    }

    #[test]
    fn authored_ascii_list_result_retains_element_boundaries_and_unknown_values() {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let context = crate::model::ingress::static_context_for(name);
            let registry = context.commands();
            for (arguments, expected) in [
                (
                    vec![Literal("source"), Literal("a b.tcl")],
                    Some(b"source {a b.tcl}".to_vec()),
                ),
                (
                    vec![Literal("#source"), Literal("b.tcl")],
                    Some(b"{#source} b.tcl".to_vec()),
                ),
                (vec![Literal("source"), Dynamic], None),
                (vec![Literal("source"), Expanded], None),
                (
                    vec![
                        Literal("source"),
                        crate::InvocationWord::KnownBytes(b"a\0b"),
                    ],
                    None,
                ),
                (
                    vec![
                        Literal("source"),
                        crate::InvocationWord::KnownBytes(b"a\xffb"),
                    ],
                    None,
                ),
            ] {
                let resolution = registry.resolve_structured_invocation(
                    InvocationWords::structured(Literal("list"), &arguments)
                        .with_profile(Some(context.commands().profile().unwrap())),
                    registry.own_surface_query(),
                );
                assert_eq!(
                    resolution
                        .resolved()
                        .unwrap()
                        .authored_source_ascii_list_result(),
                    expected,
                    "{name}: {arguments:?}"
                );
            }
        }
        let context = crate::model::ingress::static_context_for("jim");
        let resolution = context.commands().resolve_structured_invocation(
            InvocationWords::literals("list", &["source", "b.tcl"])
                .with_profile(Some(context.commands().profile().unwrap())),
            context.commands().own_surface_query(),
        );
        assert!(
            resolution
                .resolved()
                .unwrap()
                .authored_source_ascii_list_result()
                .is_none()
        );
    }

    #[test]
    fn selected_lookup_scopes_require_executable_positions_and_reference_timing() {
        // Implementation contract: naming.callback.lookup-scope-owner (docs/design/analysis/name-resolution-proofs/callback-lookup-scope-owner.md).
        use crate::ScriptLookupScope::{GlobalFrame, InvokingFrame, TriggerFrame};
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = crate::model::ingress::static_context_for(name).commands();
            for (head, arguments, index, expected) in [
                (
                    "lsort",
                    vec![Literal("-command"), Literal("cmp"), Literal("b a")],
                    1,
                    Some(InvokingFrame),
                ),
                (
                    "lsort",
                    vec![Literal("-command"), Dynamic, Literal("b a")],
                    1,
                    Some(InvokingFrame),
                ),
                (
                    "lsort",
                    vec![Literal("-command"), Literal("cmp"), Literal("b a")],
                    2,
                    None,
                ),
                ("lsort", vec![Literal("-command"), Literal("cmp")], 1, None),
                (
                    "lsort",
                    vec![Literal("-command"), Expanded, Literal("b a")],
                    1,
                    None,
                ),
                (
                    "after",
                    vec![Literal("idle"), Literal("delayed")],
                    1,
                    Some(GlobalFrame),
                ),
                (
                    "after",
                    vec![Literal("1000"), Literal("delayed")],
                    1,
                    Some(GlobalFrame),
                ),
                (
                    "after",
                    vec![Literal("cancel"), Literal("delayed")],
                    1,
                    None,
                ),
                ("after", vec![Literal("idle"), Literal("delayed")], 0, None),
            ] {
                let invocation = registry.resolve_structured_invocation(
                    InvocationWords::structured(Literal(head), &arguments),
                    registry.own_surface_query(),
                );
                let facts = invocation.resolved().unwrap().facts();
                assert_eq!(
                    facts.script_lookup_scope(index),
                    expected,
                    "{name} {head} {arguments:?} {index}"
                );
                if head == "lsort" {
                    assert_eq!(
                        facts.command_prefix_arity(index),
                        expected.map(|_| crate::AppendedArity::Exactly(2))
                    );
                }
            }
            if name == "jim" {
                continue;
            }
            for (arguments, index, expected) in [
                (
                    vec![
                        Literal("add"),
                        Literal("variable"),
                        Literal("v"),
                        Literal("write"),
                        Literal("watch"),
                    ],
                    4,
                    Some(TriggerFrame),
                ),
                (
                    vec![
                        Literal("add"),
                        Literal("variable"),
                        Literal("v"),
                        Literal("write"),
                        Literal("watch"),
                    ],
                    2,
                    None,
                ),
                (
                    vec![
                        Literal("remove"),
                        Literal("variable"),
                        Literal("v"),
                        Literal("write"),
                        Literal("watch"),
                    ],
                    4,
                    None,
                ),
            ] {
                let invocation = registry.resolve_structured_invocation(
                    InvocationWords::structured(Literal("trace"), &arguments),
                    registry.own_surface_query(),
                );
                assert_eq!(
                    invocation
                        .resolved()
                        .unwrap()
                        .facts()
                        .script_lookup_scope(index),
                    expected,
                    "{name} trace {arguments:?}"
                );
            }
        }
    }

    #[test]
    fn original_unknown_handler_scopes_require_selected_supported_callback_operands() {
        // naming.callback.original-unknown-handler-lookup-scope
        // docs/design/analysis/name-resolution-proofs/callback-original-unknown-handler-lookup-scope.md
        // This asserts Registry projection. The linked public native controls
        // independently record the reached handler; no frame or lookup is issued.
        use crate::ScriptLookupScope::{GlobalFrame, TriggerFrame};
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = crate::model::ingress::static_context_for(name);
            for (head, scope) in [
                (
                    "namespace",
                    (!matches!(name, "tcl8.4" | "jim")).then_some(TriggerFrame),
                ),
                ("package", (name != "jim").then_some(GlobalFrame)),
            ] {
                for callback in [Literal("handler"), Dynamic] {
                    let arguments = [Literal("unknown"), callback];
                    let resolution =
                        crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                            context.commands(),
                            Some(context.context()),
                            InvocationWords::structured(Literal(head), &arguments),
                            tcl_dialect::model::InvocationRealm::RuleLoader,
                        );
                    let selected = resolution.resolved();
                    assert_eq!(
                        selected.and_then(|schema| schema.facts().script_lookup_scope(1)),
                        scope,
                        "{name} {head}: {callback:?}"
                    );
                    assert!(
                        selected.is_none_or(|schema| {
                            schema.facts().script_lookup_scope(0).is_none()
                        })
                    );
                }
                for arguments in [vec![Literal("unknown")], vec![Literal("unknown"), Expanded]] {
                    let resolution =
                        crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                            context.commands(),
                            Some(context.context()),
                            InvocationWords::structured(Literal(head), &arguments),
                            tcl_dialect::model::InvocationRealm::RuleLoader,
                        );
                    assert!(
                        resolution.resolved().is_none_or(|schema| {
                            schema.facts().script_lookup_scope(1).is_none()
                        }),
                        "{name} {head}: {arguments:?}"
                    );
                }
            }
        }
    }

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
