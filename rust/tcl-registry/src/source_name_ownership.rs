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
}

/// Possible named operands, independent of interpolation, stores and current
/// contents. An opaque residual prevents proving that a name is excluded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceVariableNameOwnership {
    /// Literal binding roots, including local names of alias declarations.
    pub bindings: Vec<String>,
    /// Literal roots of inputs supplied by name, separate from interpolation.
    pub by_name_reads: Vec<String>,
    /// Unavailable roles, dynamic names or unresolved alias effects remain.
    pub opaque: bool,
}

impl SourceVariableNameOwnership {
    fn include(names: &mut Vec<String>, name: &str) {
        let root = tcl_syntax::naming::split_element_ref(name).map_or(name, |(root, _)| root);
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
        && !name.starts_with("::")
    {
        SourceVariableNameOwnership::include(names, &format!("::{name}"));
    } else {
        SourceVariableNameOwnership::include(names, name);
    }
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
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
        if purpose != SourceNameOwnershipPurpose::ScopeAliases {
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
            if let StateTransition::VariableCellAlias(alias) = &fact.transition {
                if let Some(name) = alias.local.literal() {
                    SourceVariableNameOwnership::include(&mut out.bindings, name);
                } else {
                    out.opaque = true;
                }
            }
        }
        out.opaque |= transitions.widens(StateTransitionDomain::VariableCells)
            || (self
                .semantics
                .traits
                .intersects(Traits::CREATES_SCOPE_ALIAS | Traits::ALIASES_GLOBAL)
                && !self.semantics.state_transitions.is_declared());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceNameOwnershipPurpose, SourceRolePurpose};

    #[test]
    fn original_source_roles_keep_empty_and_literal_dollar_read_write_names() {
        // experiments/name-handling/results/ssa-empty-read-write-names/results.json
        // experiments/name-handling/results/variable-protection-owner-20261014/oracles/observations.json
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
