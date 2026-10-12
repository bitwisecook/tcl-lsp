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
        if !root.is_empty() && !names.iter().any(|name| name == root) {
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
        let transitions = self.state_transitions();
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
