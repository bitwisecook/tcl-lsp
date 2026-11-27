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

//! The variables a call reads and writes, projected from its roles and effects.

use super::CommandRegistry;
use crate::InvocationWords;
use crate::arg_role::ArgRole;
use crate::invocation_words::{VariableReadProjection, VariableWriteProjection};
use crate::resolved_invocation::ResolvedInvocation;
use crate::state_transition::{StateTransition, TransitionSubject};
use crate::traits::Traits;
use tcl_dialect::model::SurfaceQuery;

#[derive(Clone, Copy)]
enum VariableWriteProjectionPurpose {
    Invocation,
    Source,
}

impl CommandRegistry {
    fn effect_query(&self, words: InvocationWords<'_>) -> Option<SurfaceQuery<'_>> {
        words
            .arguments()
            .dialect()
            .and_then(tcl_native_model::InvocationDialect::authoring_query)
            .or_else(|| self.own_surface_query())
    }

    fn effect_invocation_for_query<'r, 'w>(
        &'r self,
        words: InvocationWords<'w>,
        query: Option<SurfaceQuery<'w>>,
    ) -> Option<ResolvedInvocation<'r, 'w>> {
        let spec = self.get_for_surface(words.head_literal()?, query)?;
        if !self.effect_provider_is_admitted(spec, query) {
            return None;
        }
        self.resolve_structured_invocation(words, query).resolved()
    }

    /// Project the variable cells source-aware invocation words read **by
    /// name** — the read-only counterpart of
    /// [`Self::variable_write_projection`], and resolved the same way, so an
    /// alias or renamed spelling answers like the command it reaches.
    ///
    /// A name word that substitutes (`info exists $p`) denotes a cell only
    /// the runtime knows, so it widens [`VariableReadProjection::
    /// opaque_variable_frame`] rather than exposing its source spelling as a
    /// variable name.
    ///
    /// A destroyer's targets are read too (`unset x`, the
    /// [`Traits::DESTROYS_VARIABLE`] trait): an unbind reads its place's
    /// existence before it removes it — an absent place raises unless
    /// `-nocomplain` says otherwise — so the store feeding the place is
    /// observed. [`Self::variable_write_projection`] leaves a destroy out, a
    /// destroy being no value definition, so without this a nested `[unset
    /// x]` observed nothing and the store before it was deleted as dead.
    #[must_use]
    pub fn variable_read_projection(&self, words: InvocationWords<'_>) -> VariableReadProjection {
        let query = self.effect_query(words);
        self.variable_read_projection_for_query(words, query, false)
    }

    /// Source read footprint under independently retained complete availability.
    /// Unavailable metadata stays opaque; this does not prove a read or cell.
    #[must_use]
    pub fn variable_read_projection_in_resolved_context(
        &self,
        context: &crate::model::ResolvedContext,
        words: InvocationWords<'_>,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> VariableReadProjection {
        if words
            .head_literal()
            .is_none_or(|name| context.resolve_spec_in_realm(self, name, realm).is_none())
        {
            return VariableReadProjection {
                opaque_variable_frame: true,
                ..VariableReadProjection::default()
            };
        }
        self.variable_read_projection_for_query(
            words,
            Some(context.authoring_query().with_realm(realm)),
            true,
        )
    }

    fn variable_read_projection_for_query(
        &self,
        words: InvocationWords<'_>,
        query: Option<SurfaceQuery<'_>>,
        unavailable_is_opaque: bool,
    ) -> VariableReadProjection {
        let Some(name) = words.head_literal() else {
            return VariableReadProjection {
                literal_names: Vec::new(),
                opaque_variable_frame: true,
            };
        };
        let Some(invocation) = self.effect_invocation_for_query(words, query) else {
            return VariableReadProjection {
                opaque_variable_frame: unavailable_is_opaque,
                ..VariableReadProjection::default()
            };
        };
        let args = words.arguments();
        let wanted: &[ArgRole] = if invocation
            .semantics
            .traits
            .contains(Traits::DESTROYS_VARIABLE)
        {
            &[ArgRole::VarRead, ArgRole::VarWrite]
        } else {
            &[ArgRole::VarRead]
        };
        let Some(roles) = Self::selected_argument_role_assignments(wanted, &invocation) else {
            return VariableReadProjection {
                literal_names: Vec::new(),
                opaque_variable_frame: self.get_for_surface(name, query).is_some_and(|spec| {
                    wanted
                        .iter()
                        .any(|role| Self::spec_may_have_arg_role(spec, *role))
                }),
            };
        };
        let mut projection = VariableReadProjection::default();
        for (index, _) in roles {
            match args.literal_at(index) {
                Some(variable) if !variable.is_empty() => {
                    if !projection
                        .literal_names
                        .iter()
                        .any(|found| found == variable)
                    {
                        projection.literal_names.push(variable.to_owned());
                    }
                }
                _ => projection.opaque_variable_frame = true,
            }
        }
        projection
    }

    /// Project the variable-cell writes of source-aware invocation words.
    ///
    /// Command, subcommand, option, form, and repeated-tail selection stays
    /// owned by the registry. A dynamic or expanded target is never exposed
    /// as if its source spelling were its runtime variable name. A computed
    /// head is opaque because it may dispatch any variable-writing command.
    #[must_use]
    pub fn variable_write_projection(&self, words: InvocationWords<'_>) -> VariableWriteProjection {
        let query = self.effect_query(words);
        self.variable_write_projection_for_query(words, query, false)
    }

    /// Possible output names from independently supplied source words.
    /// The retained catalogue owns nominal selection and provider admission;
    /// this supplies no live lookup, executed write, cell or frame proof.
    #[must_use]
    pub fn variable_write_projection_for_source_words(
        &self,
        words: InvocationWords<'_>,
    ) -> VariableWriteProjection {
        let Some(invocation) = self.effect_invocation_for_query(words, self.effect_query(words))
        else {
            return VariableWriteProjection {
                opaque_variable_frame: true,
                ..Default::default()
            };
        };
        Self::variable_write_projection_for_selected_source(&invocation)
    }

    /// Source write footprint under independently retained complete availability.
    /// Unknown or unavailable metadata stays opaque, never a successful store.
    #[must_use]
    pub fn variable_write_projection_in_resolved_context(
        &self,
        context: &crate::model::ResolvedContext,
        words: InvocationWords<'_>,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> VariableWriteProjection {
        if words
            .head_literal()
            .is_none_or(|name| context.resolve_spec_in_realm(self, name, realm).is_none())
        {
            return VariableWriteProjection {
                opaque_variable_frame: true,
                ..VariableWriteProjection::default()
            };
        }
        self.variable_write_projection_for_query(
            words,
            Some(context.authoring_query().with_realm(realm)),
            true,
        )
    }

    fn variable_write_projection_for_query(
        &self,
        words: InvocationWords<'_>,
        query: Option<SurfaceQuery<'_>>,
        unavailable_is_opaque: bool,
    ) -> VariableWriteProjection {
        let Some(name) = words.head_literal() else {
            return VariableWriteProjection {
                literal_names: Vec::new(),
                read_before_write_names: Vec::new(),
                opaque_variable_frame: true,
            };
        };
        let Some(invocation) = self.effect_invocation_for_query(words, query) else {
            return VariableWriteProjection {
                opaque_variable_frame: unavailable_is_opaque,
                ..VariableWriteProjection::default()
            };
        };

        Self::selected_variable_write_projection(
            &invocation,
            VariableWriteProjectionPurpose::Invocation,
            || self.arg_role_variable_writes(name, words, query, &invocation),
        )
    }

    fn selected_variable_write_projection(
        invocation: &ResolvedInvocation<'_, '_>,
        purpose: VariableWriteProjectionPurpose,
        roles: impl FnOnce() -> VariableWriteProjection,
    ) -> VariableWriteProjection {
        // `incr` / `append` / `lappend` / `lset` / `lpop` / `ledit` fold the
        // target's current value into the one they store, so the write is
        // also a read of the same cell. Taken from the *resolved* invocation,
        // so an alias or rename spelling answers like the builtin it reaches.
        // Every VarWrite target of such a command is its read-modify-write
        // target — none of them carries a second, write-only variable role.
        let reads_before_write = invocation
            .semantics
            .traits
            .contains(Traits::READS_BEFORE_WRITE);
        let with_reads = |mut projection: VariableWriteProjection| {
            if reads_before_write {
                projection.read_before_write_names = projection.literal_names.clone();
            }
            projection
        };

        // A destroy is not a value definition. The registry's VarWrite role
        // intentionally covers both operations so name-aware consumers can
        // find their targets; this effect projection keeps their transfers
        // distinct.
        if invocation
            .semantics
            .traits
            .contains(Traits::DESTROYS_VARIABLE)
        {
            return VariableWriteProjection::default();
        }

        // A declared state-transition descriptor whose facts concern variable
        // cells is their closed semantic owner. In particular, `global`,
        // `upvar`, trace registration, and value-less `variable` forms may
        // carry VarWrite roles without writing a value. Only the descriptor's
        // explicit `writes_value` facts count here. A descriptor that only
        // binds commands (a Tk constructor publishing its path) leaves its
        // option writes (`-textvariable`) to the argument roles.
        let transitions = invocation.state_transitions();
        if invocation.semantics.state_transitions.is_declared()
            && transitions
                .facts()
                .iter()
                .any(|fact| match &fact.transition {
                    StateTransition::VariableCellAlias(_) | StateTransition::Trace(_) => true,
                    StateTransition::Widen(widening) => widening.domains.iter().any(|domain| {
                        matches!(
                            domain,
                            crate::StateTransitionDomain::VariableCells
                                | crate::StateTransitionDomain::VariableTraces
                        )
                    }),
                    _ => false,
                })
        {
            let mut projection = VariableWriteProjection::default();
            for fact in transitions.facts() {
                let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                    continue;
                };
                if matches!(
                    alias.local,
                    TransitionSubject::LocatedNativeBytes { .. }
                        | TransitionSubject::Unknown { .. }
                ) {
                    // Even declaration-only aliases make later writes land in
                    // an unnameable cell, so effect consumers must widen.
                    projection.opaque_variable_frame = true;
                }
                if !alias.writes_value {
                    continue;
                }
                match &alias.local {
                    TransitionSubject::Literal(name)
                    | TransitionSubject::LocatedLiteral { value: name, .. } => {
                        if (matches!(purpose, VariableWriteProjectionPurpose::Source)
                            || !name.is_empty())
                            && !projection.literal_names.contains(name)
                        {
                            projection.literal_names.push(name.clone());
                        }
                    }
                    TransitionSubject::LocatedNativeBytes { .. }
                    | TransitionSubject::Unknown { .. } => {
                        projection.opaque_variable_frame = true;
                    }
                }
            }
            return with_reads(projection);
        }

        with_reads(roles())
    }

    /// Possible output names of an already selected source descriptor.
    /// Selection/context remain owned by the caller. This never re-resolves a
    /// command spelling or proves a current variable write or entered frame.
    #[must_use]
    pub fn variable_write_projection_for_selected_source(
        invocation: &ResolvedInvocation<'_, '_>,
    ) -> VariableWriteProjection {
        Self::selected_variable_write_projection(
            invocation,
            VariableWriteProjectionPurpose::Source,
            || {
                let arguments = invocation.words.arguments();
                let Some(selected) = invocation.authored_source_arity() else {
                    return VariableWriteProjection {
                        opaque_variable_frame: true,
                        ..Default::default()
                    };
                };
                if arguments.exact_argv_len().is_some_and(|count| {
                    selected
                        .command
                        .variable_write_min_args
                        .is_some_and(|minimum| count < usize::from(minimum))
                }) {
                    return VariableWriteProjection::default();
                }
                let (roles, complete) = invocation.authored_source_argument_roles();
                if !complete {
                    return VariableWriteProjection {
                        opaque_variable_frame: Self::spec_may_have_arg_role(
                            selected.command,
                            ArgRole::VarWrite,
                        ),
                        ..Default::default()
                    };
                }
                let mut projection = VariableWriteProjection::default();
                for (argument, role) in roles {
                    if role != ArgRole::VarWrite {
                        continue;
                    }
                    let argument = usize::from(argument) + invocation.semantics.argument_offset;
                    let Some(name) = arguments.literal_at(argument) else {
                        projection.opaque_variable_frame = true;
                        continue;
                    };
                    let name = if invocation.authored_source_option_variable_scope_at(argument)
                        == Some(crate::hover::VariableScope::Global)
                        && tcl_syntax::naming::unroot_rooted_key(name).is_none()
                    {
                        tcl_syntax::naming::root_unrooted_key(name)
                    } else {
                        name.to_owned()
                    };
                    if !projection.literal_names.contains(&name) {
                        projection.literal_names.push(name);
                    }
                }
                projection
            },
        )
    }

    /// The [`ArgRole::VarWrite`] half of [`Self::variable_write_projection`]:
    /// the targets named by this invocation's argument words, once the
    /// declared-state-transition path has declined. Split out to keep each
    /// half readable on its own.
    fn arg_role_variable_writes(
        &self,
        name: &str,
        words: InvocationWords<'_>,
        query: Option<SurfaceQuery<'_>>,
        invocation: &ResolvedInvocation<'_, '_>,
    ) -> VariableWriteProjection {
        let args = words.arguments();
        if args.exact_argv_len().is_some_and(|count| {
            self.get_for_surface(name, query)
                .and_then(|spec| spec.variable_write_min_args)
                .is_some_and(|minimum| count < usize::from(minimum))
        }) {
            // Option-sensitive role resolution may have to abstain when a
            // dynamic leading word could be a switch. The command descriptor
            // can nevertheless prove that this exact argv is too short to
            // contain any variable target (for example `regexp $re $s`).
            return VariableWriteProjection::default();
        }
        let global_arguments: std::collections::BTreeSet<_> = (0..args.len())
            .filter(|index| {
                invocation.authored_source_option_variable_scope_at(*index)
                    == Some(crate::hover::VariableScope::Global)
            })
            .collect();
        let Some(roles) =
            Self::selected_argument_role_assignments(&[ArgRole::VarWrite], invocation)
        else {
            return VariableWriteProjection {
                literal_names: Vec::new(),
                read_before_write_names: Vec::new(),
                opaque_variable_frame: self
                    .get_for_surface(name, query)
                    .is_some_and(|spec| Self::spec_may_have_arg_role(spec, ArgRole::VarWrite)),
            };
        };
        let mut projection = VariableWriteProjection::default();
        for (index, _) in roles {
            match args.literal_at(index) {
                Some(variable) if !variable.is_empty() => {
                    let effective_name = if global_arguments.contains(&index)
                        && tcl_syntax::naming::unroot_rooted_key(variable).is_none()
                    {
                        tcl_syntax::naming::root_unrooted_key(variable)
                    } else {
                        variable.to_owned()
                    };
                    if !projection
                        .literal_names
                        .iter()
                        .any(|found| found == &effective_name)
                    {
                        projection.literal_names.push(effective_name);
                    }
                }
                // An element whose key is computed still names its array.
                _ => match args.array_element_root_at(index) {
                    Some(root) if !root.is_empty() => {
                        if !projection.literal_names.iter().any(|found| found == root) {
                            projection.literal_names.push(root.to_owned());
                        }
                    }
                    _ => projection.opaque_variable_frame = true,
                },
            }
        }
        projection
    }
}
