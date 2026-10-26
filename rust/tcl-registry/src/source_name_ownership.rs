// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lexical variable-name ownership under an independently selected source schema.

use crate::{ArgRole, ResolvedInvocation, StateTransition, StateTransitionDomain, Traits};

/// The independently selected source-role grammar. Neither variant supplies
/// an entered frame, native argument acceptance or successful handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRolePurpose {
    /// Original dialect-aware descriptor roles, with ordinary uncertainty.
    Original,
    /// Existing authored frame layouts under a positive Logical source model.
    Logical,
}

/// Which named operands contribute to conditional lexical ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceNameOwnershipPurpose {
    /// Literal binding operands and loop-variable lists.
    Bindings,
    /// Binding operands and inputs passed by variable name.
    BindingsOrNameReads,
    /// Local operands of typed variable-cell alias declarations.
    ScopeAliases,
    /// Alias locals and the original variable subjects of trace registrations.
    /// This is a lexical exclusion veto, not an installed observer or read fact.
    ScopeAliasesAndTraceRegistrations,
}

/// Possible named operands, independent of interpolation, stores and current
/// contents. An opaque residual prevents proving that a name is excluded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceVariableNameOwnership {
    /// Literal source roots selected by this ownership purpose. Alias and
    /// trace subjects remain lexical exclusions, not installed bindings.
    pub bindings: Vec<String>,
    /// Literal roots of inputs supplied by name, separate from interpolation.
    pub by_name_reads: Vec<String>,
    /// Unavailable roles, dynamic names or unresolved alias effects remain.
    pub opaque: bool,
}

impl SourceVariableNameOwnership {
    fn include(names: &mut Vec<String>, name: &str) {
        let root = tcl_syntax::naming::split_element_ref(name).map_or(name, |(root, _)| root);
        Self::include_root(names, root);
    }

    fn include_root(names: &mut Vec<String>, root: &str) {
        if !names.iter().any(|name| name == root) {
            names.push(root.to_owned());
        }
    }
}

fn include_operand(
    invocation: &ResolvedInvocation<'_, '_>,
    names: &mut Vec<String>,
    argument: usize,
    name: &str,
) {
    if invocation.authored_source_option_variable_scope_at(argument)
        == Some(crate::VariableScope::Global)
        && tcl_syntax::naming::unroot_rooted_key(name).is_none()
    {
        SourceVariableNameOwnership::include(names, &tcl_syntax::naming::root_unrooted_key(name));
    } else {
        SourceVariableNameOwnership::include(names, name);
    }
}

/// The actual typed trace subject retains its own receiver ingress. An
/// analytical string containing NUL cannot donate a counted Native byte value;
/// unavailable subject geometry remains an opaque exclusion obligation.
fn trace_registration_root(
    invocation: &ResolvedInvocation<'_, '_>,
    role_purpose: SourceRolePurpose,
    subject: &crate::TransitionSubject,
) -> Option<String> {
    use crate::resolved_invocation::VariableReceiverOperandForm;
    let argument = subject.argument_index()?;
    let form = invocation.authored_source_variable_receiver_operand_form(argument)?;
    if form != VariableReceiverOperandForm::TraceSubject {
        return None;
    }
    let dialect = invocation.words.arguments().dialect()?;
    let protocol = match role_purpose {
        SourceRolePurpose::Original => dialect.native_name_protocol()?,
        SourceRolePurpose::Logical => dialect.authored_name_policy()?.recipe(),
    };
    let original = if let Some(original) = subject.native_bytes() {
        original
    } else {
        let original = subject.literal()?.as_bytes();
        if original.contains(&0) {
            return None;
        }
        original
    };
    let tcl_syntax::naming::NativeVariableInputForm::Combined(selected) =
        form.input_form(protocol, original)?
    else {
        return None;
    };
    let projection = protocol.combined_variable_input(selected);
    std::str::from_utf8(projection.root().selected())
        .ok()
        .map(str::to_owned)
}

impl ResolvedInvocation<'_, '_> {
    /// Conditional named operands from this already selected descriptor and
    /// exact effective argv. Selection/availability and Logical applicability
    /// remain caller-owned; no runtime alias, body entry or write is issued.
    #[must_use]
    pub fn authored_source_name_ownership(
        &self,
        role_purpose: SourceRolePurpose,
        purpose: SourceNameOwnershipPurpose,
    ) -> SourceVariableNameOwnership {
        let mut out = SourceVariableNameOwnership::default();
        if let Some(arity) = self.authored_source_arity() {
            if arity.count.indeterminate || !arity.arity.accepts(arity.count.minimum) {
                out.opaque = true;
                return out;
            }
        } else if !self
            .semantics
            .traits
            .contains(Traits::STRUCTURALLY_CHECKED_ARITY)
        {
            out.opaque = true;
            return out;
        }
        let (roles, complete) = match role_purpose {
            SourceRolePurpose::Original => self.authored_source_argument_roles(),
            SourceRolePurpose::Logical => self.authored_logical_source_argument_roles(),
        };
        out.opaque |= !complete;
        let arguments = self.words.arguments();
        if !matches!(
            purpose,
            SourceNameOwnershipPurpose::ScopeAliases
                | SourceNameOwnershipPurpose::ScopeAliasesAndTraceRegistrations
        ) {
            for (argument, role) in roles {
                let argument = self.semantics.argument_offset + usize::from(argument);
                if role == ArgRole::VarRead
                    && purpose == SourceNameOwnershipPurpose::BindingsOrNameReads
                {
                    match arguments.literal_at(argument) {
                        Some(name) => include_operand(self, &mut out.by_name_reads, argument, name),
                        None => out.opaque = true,
                    }
                } else if role == ArgRole::VarWrite
                    && !self.semantics.traits.contains(Traits::DESTROYS_VARIABLE)
                    && !self
                        .semantics
                        .traits
                        .intersects(Traits::CREATES_SCOPE_ALIAS | Traits::ALIASES_GLOBAL)
                {
                    match arguments.literal_at(argument) {
                        Some(name) => include_operand(self, &mut out.bindings, argument, name),
                        None => out.opaque = true,
                    }
                } else if role == ArgRole::LoopVarList {
                    let values = arguments
                        .literal_at(argument)
                        .zip(arguments.dialect())
                        .and_then(|(list, dialect)| dialect.word_values.split_list(list).ok());
                    if let Some(values) = values {
                        for name in values {
                            SourceVariableNameOwnership::include(&mut out.bindings, &name);
                        }
                    } else {
                        out.opaque = true;
                    }
                }
            }
        }
        let transitions = match role_purpose {
            SourceRolePurpose::Original => self.state_transitions(),
            SourceRolePurpose::Logical => self
                .authored_logical_source_state_transitions()
                .unwrap_or_else(|| {
                    out.opaque = true;
                    self.state_transitions()
                }),
        };
        for fact in transitions.facts() {
            if purpose == SourceNameOwnershipPurpose::ScopeAliasesAndTraceRegistrations
                && let StateTransition::Trace(crate::TraceTransition::Add {
                    target: crate::TraceTarget::Variable(subject),
                    ..
                }) = &fact.transition
            {
                match trace_registration_root(self, role_purpose, subject) {
                    Some(root) => {
                        SourceVariableNameOwnership::include_root(&mut out.bindings, &root);
                    }
                    None => out.opaque = true,
                }
            }
            if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                if let Some(name) = alias.local.literal() {
                    SourceVariableNameOwnership::include(&mut out.bindings, name);
                } else {
                    out.opaque = true;
                }
            }
        }
        out.opaque |= (purpose == SourceNameOwnershipPurpose::ScopeAliasesAndTraceRegistrations
            && transitions.widens(StateTransitionDomain::VariableTraces))
            || transitions.widens(StateTransitionDomain::VariableCells)
            || (self
                .semantics
                .traits
                .intersects(Traits::CREATES_SCOPE_ALIAS | Traits::ALIASES_GLOBAL)
                && !self.semantics.state_transitions.is_declared());
        out
    }
}

/// One literal name operand in a nominal catalogue mutation layout. This
/// withholds source advice only; it selects no available handler or binding.
#[derive(Debug, Clone)]
pub struct NominalSourceCommandNameHazard {
    descriptor: &'static crate::CommandSpec,
    argument: usize,
    name: Vec<u8>,
    alias_replacement: Option<NominalSourceAliasReplacement>,
}

/// Original literal operands of one possible same-interpreter source alias.
/// This is a conditional schema alternative, never an installed alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NominalSourceAliasReplacement {
    target: usize,
    prefix: Vec<usize>,
}

impl NominalSourceAliasReplacement {
    /// Original post-head target operand.
    #[must_use]
    pub const fn target_argument(&self) -> usize {
        self.target
    }

    /// Original post-head captured operands, in their declared order.
    #[must_use]
    pub fn prefix_arguments(&self) -> &[usize] {
        &self.prefix
    }
}

impl PartialEq for NominalSourceCommandNameHazard {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.descriptor, other.descriptor)
            && self.argument == other.argument
            && self.name == other.name
            && self.alias_replacement == other.alias_replacement
    }
}
impl Eq for NominalSourceCommandNameHazard {}

impl NominalSourceCommandNameHazard {
    /// Actual descriptor row in the supplied catalogue, without availability.
    #[must_use]
    pub const fn descriptor(&self) -> &'static crate::CommandSpec {
        self.descriptor
    }

    /// Original post-head operand ordinal from the shared transition schema.
    #[must_use]
    pub const fn argument(&self) -> usize {
        self.argument
    }

    /// Exact retained literal value; no command slot or identity is issued.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Possible source replacement operands from the same actual transition.
    #[must_use]
    pub const fn alias_replacement(&self) -> Option<&NominalSourceAliasReplacement> {
        self.alias_replacement.as_ref()
    }
}

impl crate::CommandRegistry {
    /// Nominal source mutation operands from this exact catalogue. Descriptor
    /// availability and actual dispatch are deliberately unproved: this query
    /// can only withdraw advice about a matching source name. Missing or dynamic
    /// operands issue no named hazard and never prove absence or a quiet world.
    #[must_use]
    pub fn nominal_source_command_name_hazards(
        &self,
        words: crate::InvocationWords<'_>,
    ) -> Vec<NominalSourceCommandNameHazard> {
        use crate::CommandBindingTransition;
        let Some(descriptor) = words.head_literal().and_then(|name| self.get(name)) else {
            return Vec::new();
        };
        let selected = crate::registry::resolve_source_descriptor_invocation(
            descriptor,
            words,
            self.source_descriptor_availability(descriptor, None),
            words.arguments().dialect().map(|dialect| dialect.numbers),
        );
        if !selected
            .argument_count_for_arity()
            .is_some_and(|count| selected.semantics.arity.accepts(count))
        {
            return Vec::new();
        }
        let transitions = selected.state_transitions();
        let mut subjects = Vec::new();
        for transition in transitions.command_bindings() {
            match transition {
                CommandBindingTransition::Define { name, .. } => subjects.push((name, None)),
                CommandBindingTransition::Move { from, to } => {
                    subjects.extend([(from, None), (to, None)]);
                }
                CommandBindingTransition::Delete { interpreter, name } => {
                    if interpreter
                        .as_ref()
                        .is_none_or(|path| path.literal() == Some(""))
                    {
                        subjects.push((name, None));
                    }
                }
                CommandBindingTransition::Alias {
                    source_interpreter,
                    alias,
                    target_interpreter,
                    target,
                    arguments,
                    target_lookup,
                } => {
                    if source_interpreter.literal() == Some("") {
                        let replacement = nominal_alias_replacement(
                            words.arguments(),
                            target_interpreter,
                            target,
                            arguments,
                            *target_lookup,
                        );
                        subjects.push((alias, replacement));
                    }
                }
                CommandBindingTransition::Unknown { .. } => {}
            }
        }
        let mut hazards = Vec::new();
        for (subject, alias_replacement) in subjects {
            let Some(argument) = subject.argument_index() else {
                continue;
            };
            let Some(name) = subject
                .literal()
                .map(str::as_bytes)
                .or_else(|| subject.native_bytes())
            else {
                continue;
            };
            let actual = words.arguments().get(argument).and_then(|word| {
                word.literal()
                    .map(str::as_bytes)
                    .or_else(|| word.native_bytes())
            });
            if actual != Some(name) {
                continue;
            }
            let hazard = NominalSourceCommandNameHazard {
                descriptor,
                argument,
                name: name.to_vec(),
                alias_replacement,
            };
            if !hazards.contains(&hazard) {
                hazards.push(hazard);
            }
        }
        hazards
    }
}

/// The exact written-word replacement of a same-interpreter global alias.
fn nominal_alias_replacement(
    words: crate::InvocationArguments<'_>,
    target_interpreter: &crate::TransitionSubject,
    target: &crate::TransitionSubject,
    prefix: &[crate::TransitionSubject],
    target_lookup: crate::AliasTargetLookup,
) -> Option<NominalSourceAliasReplacement> {
    if target_interpreter.literal() != Some("") || target_lookup != crate::AliasTargetLookup::Global
    {
        return None;
    }
    let exact_argument = |subject: &crate::TransitionSubject| {
        let ordinal = subject.argument_index()?;
        let expected = subject
            .literal()
            .map(str::as_bytes)
            .or_else(|| subject.native_bytes())?;
        let actual = words.get(ordinal)?;
        (actual
            .literal()
            .map(str::as_bytes)
            .or_else(|| actual.native_bytes())
            == Some(expected))
        .then_some(ordinal)
    };
    Some(NominalSourceAliasReplacement {
        target: exact_argument(target)?,
        prefix: prefix
            .iter()
            .map(exact_argument)
            .collect::<Option<Vec<_>>>()?,
    })
}

#[cfg(test)]
mod tests {
    use super::{SourceNameOwnershipPurpose, SourceRolePurpose};

    #[test]
    fn trace_registration_source_exclusion_keeps_subject_ingress_and_existing_purposes() {
        use crate::InvocationWord::{Dynamic, KnownBytes, Literal};
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = crate::model::ingress::context_for_profile(profile);
        let dialect = crate::InvocationDialect::of_profile(profile);
        for (target, expected, opaque) in [
            (Literal("local"), vec!["local"], false),
            (Literal("other"), vec!["other"], false),
            (Literal("local(key)"), vec!["local"], false),
            (KnownBytes(b"local\0tail"), vec!["local"], false),
            (Literal("local\0tail"), Vec::new(), true),
            (Dynamic, Vec::new(), true),
        ] {
            let arguments = [
                Literal("add"),
                Literal("variable"),
                target,
                Literal("write"),
                Literal("callback"),
            ];
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("trace"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let schema = selected.resolved().unwrap();
            let old = schema.authored_source_name_ownership(
                SourceRolePurpose::Original,
                SourceNameOwnershipPurpose::ScopeAliases,
            );
            assert!(
                old.bindings.is_empty(),
                "existing alias-only purpose: {target:?}"
            );
            let selected = schema.authored_source_name_ownership(
                SourceRolePurpose::Original,
                SourceNameOwnershipPurpose::ScopeAliasesAndTraceRegistrations,
            );
            assert_eq!(selected.bindings, expected, "{target:?}");
            assert_eq!(selected.opaque, opaque, "{target:?}");
        }
        for arguments in [
            vec![
                Literal("add"),
                Literal("command"),
                Literal("local"),
                Literal("rename"),
                Literal("callback"),
            ],
            vec![
                Literal("remove"),
                Literal("variable"),
                Literal("local"),
                Literal("write"),
                Literal("callback"),
            ],
        ] {
            let selected =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("trace"), &arguments)
                        .with_dialect(dialect),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let selected = selected.resolved().unwrap().authored_source_name_ownership(
                SourceRolePurpose::Original,
                SourceNameOwnershipPurpose::ScopeAliasesAndTraceRegistrations,
            );
            assert!(
                selected.bindings.is_empty(),
                "a non-registration has no variable registration subject"
            );
            assert!(!selected.opaque);
        }
    }

    #[test]
    fn nominal_command_hazards_keep_literal_source_ordinals_without_availability() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let context = crate::model::ingress::static_context_for("f5-irules");
        for (arguments, expected) in [
            (
                vec![Literal("pool"), Literal("moved")],
                vec![(0, b"pool".as_slice()), (1, b"moved".as_slice())],
            ),
            (
                vec![Dynamic, Literal("moved")],
                vec![(1, b"moved".as_slice())],
            ),
            (vec![Literal("pool"), Dynamic], Vec::new()),
            (vec![Expanded, Literal("moved")], Vec::new()),
        ] {
            let words = crate::InvocationWords::structured(Literal("rename"), &arguments);
            assert!(
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    words,
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                )
                .resolved()
                .is_none()
            );
            let hazards = context
                .commands()
                .nominal_source_command_name_hazards(words);
            assert_eq!(
                hazards
                    .iter()
                    .map(|hazard| (hazard.argument(), hazard.name()))
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(hazards.iter().all(|hazard| std::ptr::eq(
                hazard.descriptor(),
                context.commands().get("rename").unwrap()
            )));
        }
    }

    #[test]
    fn nominal_alias_replacements_keep_original_target_and_prefix_ordinals() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let context = crate::model::ingress::static_context_for("f5-irules");
        for (arguments, replacement) in [
            (
                vec![
                    Literal("alias"),
                    Literal(""),
                    Literal("members"),
                    Literal(""),
                    Literal("active_members"),
                    Literal("-list"),
                ],
                true,
            ),
            (
                vec![
                    Literal("alias"),
                    Literal(""),
                    Literal("members"),
                    Literal(""),
                    Dynamic,
                ],
                false,
            ),
            (
                vec![
                    Literal("alias"),
                    Literal(""),
                    Literal("members"),
                    Literal(""),
                    Literal("active_members"),
                    Dynamic,
                ],
                false,
            ),
            (
                vec![
                    Literal("alias"),
                    Literal(""),
                    Literal("members"),
                    Literal("child"),
                    Literal("active_members"),
                ],
                false,
            ),
            (
                vec![
                    Literal("alias"),
                    Literal(""),
                    Literal("members"),
                    Literal(""),
                    Literal("active_members"),
                    Expanded,
                ],
                false,
            ),
        ] {
            let hazards = context.commands().nominal_source_command_name_hazards(
                crate::InvocationWords::structured(Literal("interp"), &arguments),
            );
            assert!(hazards.iter().all(|hazard| hazard.name() == b"members"));
            let actual = hazards
                .first()
                .and_then(|hazard| hazard.alias_replacement());
            assert_eq!(actual.is_some(), replacement, "{arguments:?}");
            if let Some(actual) = actual {
                assert_eq!(actual.target_argument(), 4);
                assert_eq!(actual.prefix_arguments(), &[5]);
            }
        }
    }

    #[test]
    fn original_source_roles_keep_empty_and_literal_dollar_read_write_names() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let context =
                crate::model::ingress::resolve_environment(environment).default_context_registry();
            for (head, arguments, writes, reads) in [
                ("set", vec!["", "VALUE"], vec![""], vec![]),
                ("set", vec![""], vec![], vec![""]),
                ("foreach", vec!["{}", "VALUE", ""], vec![""], vec![]),
                (
                    "foreach",
                    vec!["{$name}", "VALUE", ""],
                    vec!["$name"],
                    vec![],
                ),
                ("set", vec!["$counter", "VALUE"], vec!["$counter"], vec![]),
                ("set", vec!["$counter"], vec![], vec!["$counter"]),
            ] {
                let arguments = arguments
                    .iter()
                    .map(|value| crate::InvocationWord::Literal(value))
                    .collect::<Vec<_>>();
                let selected =
                    crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                        context.commands(),
                        Some(context.context()),
                        crate::InvocationWords::structured(
                            crate::InvocationWord::Literal(head),
                            &arguments,
                        )
                        .with_profile(context.commands().profile()),
                        tcl_dialect::model::InvocationRealm::RuleLoader,
                    );
                let schema = selected.resolved().expect("available original descriptor");
                let names = schema.authored_source_name_ownership(
                    SourceRolePurpose::Original,
                    SourceNameOwnershipPurpose::BindingsOrNameReads,
                );
                assert_eq!(
                    names.bindings, writes,
                    "{environment}: {head} {arguments:?}"
                );
                assert_eq!(
                    names.by_name_reads, reads,
                    "{environment}: {head} {arguments:?}"
                );
            }
        }
    }
}
