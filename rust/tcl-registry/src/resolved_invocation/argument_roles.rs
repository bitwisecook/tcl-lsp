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

//! The role of each argument word: declared, repeated, resolver-supplied and frame-layout roles, on either completion edge.

use super::subcommand_resolution::SubcommandResolution;
use super::{ResolvedInvocation, SelectedDescriptors};
use crate::arg_role::ArgRole;
use crate::invocation_words::{CommandPrefixArguments, InvocationWord};
use crate::spec::ArgRoleResolverInput;

/// Readonly argument roles under a separately retained Logical source model.
/// Case bodies retain their original effective container/list-element locations
/// and every option-selection or no-match alternative. This projection is
/// independent of executable `InvocationFacts` and native handler acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredLogicalSourceRoleProjection {
    pub(super) argument_roles: Vec<(usize, ArgRole)>,
    case_bodies: Option<crate::case_bodies::CaseBodyOperands>,
}

impl AuthoredLogicalSourceRoleProjection {
    /// Effective post-head roles; list fields remain on `case_body_operands`.
    #[must_use]
    pub fn argument_roles(&self) -> &[(usize, ArgRole)] {
        &self.argument_roles
    }

    /// Possible original case fields, with selection and no-match residuals.
    #[must_use]
    pub fn case_body_operands(&self) -> Option<&crate::case_bodies::CaseBodyOperands> {
        self.case_bodies.as_ref()
    }

    /// Whether a computed subject can change option parsing or raise there.
    #[must_use]
    pub fn selection_unknown(&self) -> bool {
        self.case_bodies
            .as_ref()
            .is_some_and(|bodies| bodies.selection_unknown)
    }
}

impl<'w> ResolvedInvocation<'_, 'w> {
    /// The call's argument roles — every `(position, role)` the registry
    /// assigns, in post-head coordinates, sorted by position (the roles one
    /// position carries in [`ArgRole::ALL`] order): the clause grammar, the
    /// resolver or the static table, repeated tails, option values and
    /// command prefixes, as
    /// [`crate::CommandRegistry::arg_indices_for_role_words`] answers them
    /// role by role, for the command or subcommand (or instance method) this
    /// resolution selected under its [`Self::dialect`].
    ///
    /// `None` when the words cannot prove the layout — a `{*}` expansion, a
    /// computed subcommand word, or a computed word where an option a
    /// resolver reads could stand. A computed ordinary operand is fine: it
    /// occupies one position. An empty call is read, not abstained on.
    #[must_use]
    pub fn arg_roles(&self) -> Option<Vec<(usize, ArgRole)>> {
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        if matches!(
            self.subcommand,
            SubcommandResolution::Indeterminate { .. }
                | SubcommandResolution::Unknown { .. }
                | SubcommandResolution::Ambiguous { .. }
        ) {
            return None;
        }
        let (mut selected, complete) = self.authored_source_argument_roles();
        if !complete {
            // A literal-value hook may inspect any payload. Option topology
            // alone cannot establish value independence for that callback.
            if !self.layout_is_proven()
                || self.semantics.clause_grammar.is_some()
                || self.semantics.frame_effect.is_some()
                || self.semantics.arg_role_count_resolver.is_some()
                || self.semantics.arg_role_layout_resolver.is_some()
            {
                return None;
            }
            let values = arguments.literal_values()?;
            selected =
                self.semantics.arg_role_resolver?(values.get(self.semantics.argument_offset..)?);
            let mut complete = true;
            self.extend_repeated_roles(&mut selected, &mut complete, false, None);
            selected.extend(
                self.semantics
                    .options
                    .value_roles(arguments.slice_from(self.semantics.argument_offset))?
                    .into_iter()
                    .map(|(at, role)| u8::try_from(at).ok().map(|at| (at, role)))
                    .collect::<Option<Vec<_>>>()?,
            );
            if !complete {
                return None;
            }
        }
        let SelectedDescriptors {
            spec,
            sub,
            instance,
            ..
        } = self.selected;
        let pattern_selected = !instance
            && sub.is_none()
            && (spec.pattern_arg_resolver.is_some() || spec.option_selects_pattern_language());
        let case_body_allowed = instance
            || sub.is_some()
            || spec.case_list.is_none()
            || self.case_invocation().is_some();
        let mut roles = selected
            .into_iter()
            .filter(|(_, role)| {
                *role != ArgRole::CommandPrefix
                    && (*role != ArgRole::Body || case_body_allowed)
                    && (*role != ArgRole::Pattern || !pattern_selected)
            })
            .map(|(at, role)| (usize::from(at) + self.semantics.argument_offset, role))
            .collect::<Vec<_>>();
        if pattern_selected {
            roles.extend(
                self.pattern_args()
                    .into_iter()
                    .map(|pattern| (usize::from(pattern.index), ArgRole::Pattern)),
            );
        }
        let spellings = self.placeholder_spellings()?;
        roles.extend(
            self.command_prefixes_over(&spellings)
                .into_iter()
                .map(|(at, _)| (at, ArgRole::CommandPrefix)),
        );
        roles.retain(|(at, _)| *at < count);
        crate::registry::sort_role_table(&mut roles);
        Some(roles)
    }

    /// The command-prefix positions and appended arities over the placeholder
    /// spellings, with this resolution's word facts behind them so a
    /// literal-sensitive resolver abstains on a computed word.
    fn command_prefixes_over(
        &self,
        spellings: &[&'w str],
    ) -> Vec<(usize, crate::arg_role::AppendedArity)> {
        let arguments = self.words.arguments();
        let words: Vec<InvocationWord<'w>> = (0..spellings.len())
            .map(|index| arguments.get(index).unwrap_or(InvocationWord::Opaque))
            .collect();
        let Some(arguments) = CommandPrefixArguments::structured(spellings, &words) else {
            return Vec::new();
        };
        let SelectedDescriptors { spec, sub, .. } = self.selected;
        crate::registry::command_prefixes_in(spec, sub, arguments)
    }

    /// The call's pattern-bearing arguments and the language each is written
    /// in — [`crate::CommandRegistry::pattern_args_words_for_dialect`]
    /// re-keyed on the resolution. Empty when the command declares none, for
    /// an instance method (the class command's pattern tables are not the
    /// method's), and when the words cannot prove the layout that decides
    /// them.
    #[must_use]
    pub fn pattern_args(&self) -> Vec<crate::patterns::PatternArg> {
        let SelectedDescriptors {
            spec,
            sub,
            instance,
            ..
        } = self.selected;
        if instance || !self.layout_is_proven() {
            return Vec::new();
        }
        let Some(spellings) = self.placeholder_spellings() else {
            return Vec::new();
        };
        crate::registry::pattern_args_in(
            spec,
            sub,
            &spellings,
            || self.spec_options(),
            self.dialect,
            || {
                self.authored_source_argument_roles()
                    .0
                    .into_iter()
                    .filter(|(_, role)| *role == ArgRole::Pattern)
                    .map(|(at, _)| usize::from(at) + self.semantics.argument_offset)
                    .collect()
            },
        )
    }

    fn extend_repeated_roles(
        &self,
        roles: &mut Vec<(u8, ArgRole)>,
        complete: &mut bool,
        successful: bool,
        frame_layout: Option<crate::frame_effect::FrameArgumentResolution>,
    ) {
        if self.semantics.repeated_args.is_empty() {
            return;
        }
        let arguments = self
            .words
            .arguments()
            .slice_from(self.semantics.argument_offset);
        let Some(count) = arguments.exact_argv_len() else {
            *complete = false;
            return;
        };
        for layout in self.semantics.repeated_args {
            // Without a frame grammar to resolve the leading word's width,
            // the layout anchors itself on the last covered word.
            let indices = if layout.optional_leading_word
                && (frame_layout.is_some() || self.semantics.frame_effect.is_some())
            {
                match frame_layout.or_else(|| {
                    self.semantics.frame_effect.map(|effect| {
                        if successful {
                            effect.successful_layout(arguments).layout
                        } else {
                            effect.resolve_arguments(arguments)
                        }
                    })
                }) {
                    Some(crate::frame_effect::FrameArgumentResolution::Valid {
                        level_word_len,
                        ..
                    }) => layout.indices_with_leading_word(count, level_word_len),
                    Some(crate::frame_effect::FrameArgumentResolution::Invalid) => continue,
                    _ => {
                        *complete = false;
                        continue;
                    }
                }
            } else {
                layout.indices(count)
            };
            for index in indices {
                let Ok(index) = u8::try_from(index) else {
                    *complete = false;
                    continue;
                };
                if !roles.contains(&(index, layout.role)) {
                    roles.push((index, layout.role));
                }
            }
        }
        roles.sort_by_key(|(index, _)| *index);
    }

    /// Materialise the selected positional, frame and repeated argument roles.
    ///
    /// Indices are relative to [`InvocationSemantics::argument_offset`]. The
    /// boolean records whether literal-dependent layout selection is complete.
    /// This is the shared projection used by owned facts and compatibility
    /// role queries; selecting a command form must not change their answers.
    #[must_use]
    pub fn argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_on_edge(false)
    }

    /// Argument-role projection on successful completion. A shape eliminated
    /// by grammar error cannot contribute roles to this normal edge.
    #[must_use]
    pub fn argument_roles_after_success(&self) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_on_edge(true)
    }

    fn non_frame_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        if self.semantics.arg_role_resolver_input() == ArgRoleResolverInput::ConflictingResolvers {
            return (Vec::new(), false);
        }
        let (mut roles, mut complete) = self.resolver_argument_roles();
        if self.semantics.clause_grammar.is_some() {
            match self.clause_walk() {
                Some(Ok(plan)) => {
                    for (at, role) in plan.roles {
                        if let Some(at) = at
                            .checked_sub(self.semantics.argument_offset)
                            .and_then(|at| u8::try_from(at).ok())
                        {
                            if !roles.contains(&(at, role)) {
                                roles.push((at, role));
                            }
                        } else {
                            complete = false;
                        }
                    }
                }
                _ => complete = false,
            }
        }
        (roles, complete)
    }

    fn resolver_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        if let Some(resolver) = self.semantics.arg_role_layout_resolver {
            resolver(
                self.words
                    .arguments()
                    .slice_from(self.semantics.argument_offset),
                self.semantics.options,
            )
            .map_or_else(
                || (self.semantics.arg_roles.to_vec(), false),
                |roles| (roles, true),
            )
        } else if let Some(resolver) = self.semantics.arg_role_count_resolver {
            self.words
                .arguments()
                .exact_argv_len()
                .and_then(|count| count.checked_sub(self.semantics.argument_offset))
                .map_or_else(
                    || (self.semantics.arg_roles.to_vec(), false),
                    |count| (resolver(count), count <= usize::from(u8::MAX) + 1),
                )
        } else {
            match self.semantics.arg_role_resolver {
                Some(resolver) => self
                    .words
                    .arguments()
                    .literal_values()
                    .and_then(|arguments| {
                        arguments
                            .get(self.semantics.argument_offset..)
                            .map(resolver)
                    })
                    .map_or_else(
                        || (self.semantics.arg_roles.to_vec(), false),
                        |roles| (roles, true),
                    ),
                None => (self.semantics.arg_roles.to_vec(), true),
            }
        }
    }

    /// Authored source grammar, independent of native procedure installation.
    /// Options, frame selectors and unknown cardinality retain their ordinary
    /// role uncertainty. Procedure roles come from the selected authored
    /// descriptor; these possible positions grant neither executable facts,
    /// native parameter acceptance nor a successful definition.
    #[must_use]
    pub fn authored_source_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        let (roles, complete) = self.argument_roles_with_native_definition(false, false);
        if self.uses_authored_jim_core_layout() {
            self.authored_jim_core_roles(roles, complete)
        } else {
            (roles, complete)
        }
    }

    fn authored_jim_core_roles(
        &self,
        mut roles: Vec<(u8, ArgRole)>,
        complete: bool,
    ) -> (Vec<(u8, ArgRole)>, bool) {
        let Some(positions) = self
            .semantics
            .procedure_definition
            .and_then(|descriptor| descriptor.authored_source_positions(self.words.arguments()))
        else {
            return (Vec::new(), false);
        };
        let role_at = |role| {
            roles
                .iter()
                .filter_map(|&(at, actual)| (actual == role).then_some(usize::from(at)))
                .collect::<Vec<_>>()
        };
        let names = role_at(ArgRole::Name);
        let parameters = role_at(ArgRole::ParamList);
        let bodies = role_at(ArgRole::Body);
        let statics = role_at(ArgRole::StaticVarList);
        if !complete
            || names != [positions.name]
            || parameters != [positions.parameters]
            || bodies.len() != 1
            || (bodies[0] != positions.body && !(positions.statics == Some(2) && bodies[0] == 2))
            || (!statics.is_empty() && statics.as_slice() != positions.statics.as_slice())
        {
            return (Vec::new(), false);
        }
        roles.retain(|&(_, role)| !matches!(role, ArgRole::Body | ArgRole::StaticVarList));
        if let Some(at) = positions.statics {
            roles.push((
                u8::try_from(at).expect("core static position"),
                ArgRole::StaticVarList,
            ));
        }
        roles.push((
            u8::try_from(positions.body).expect("core body position"),
            ArgRole::Body,
        ));
        (roles, true)
    }

    /// Readonly roles in an independently retained Logical source model.
    /// Frame positions use only unanimous existing authored Tcl layouts over
    /// the same structured argv. Dynamic or divergent selectors stay unknown.
    /// Callers must retain the complete positive Logical input; this purpose
    /// grants no Native grammar, entered frame, effects or successful handler.
    #[must_use]
    pub fn authored_logical_source_argument_roles(&self) -> (Vec<(u8, ArgRole)>, bool) {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let layout = self
            .semantics
            .frame_effect
            .map(|effect| effect.logical_source_layout(self.words.arguments()));
        self.argument_roles_with_frame_layout(false, false, layout)
    }

    /// Readonly source roles from this already selected descriptor. The caller
    /// must independently retain a positive complete Logical source input.
    /// Case option ambiguity remains explicit rather than making Native roles
    /// complete; byte values require a lossless String-compatible facet here.
    #[must_use]
    pub fn authored_logical_source_role_projection(
        &self,
    ) -> Option<AuthoredLogicalSourceRoleProjection> {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let arguments = self.words.arguments();
        let count = arguments.exact_argv_len()?;
        if let Some(case) = self.semantics.options.case_list {
            if self.semantics.argument_offset != 0 {
                return None;
            }
            let dialect = arguments.dialect()?;
            let logical = (0..count)
                .map(|argument| match arguments.get(argument)? {
                    InvocationWord::KnownBytes(bytes) => {
                        (!bytes.contains(&0)).then_some(())?;
                        Some(InvocationWord::Literal(std::str::from_utf8(bytes).ok()?))
                    }
                    word => Some(word),
                })
                .collect::<Option<Vec<_>>>()?;
            let logical = crate::InvocationArguments::structured(&logical).with_dialect(dialect);
            let options = self.semantics.options.available().collect::<Vec<_>>();
            let bodies = case.possible_body_operands(logical, &options)?;
            let mut argument_roles = Vec::new();
            for body in &bodies.bodies {
                if !argument_roles.contains(&(body.argument, ArgRole::Body)) {
                    argument_roles.push((body.argument, ArgRole::Body));
                }
            }
            // Variable-valued option roles require their own closed prefix.
            // Possible bodies alone cannot infer the width of an unknown option.
            if !bodies.selection_unknown {
                argument_roles.extend(self.semantics.options.value_roles(logical)?);
            }
            return Some(AuthoredLogicalSourceRoleProjection {
                argument_roles,
                case_bodies: Some(bodies),
            });
        }
        let (roles, complete) = self.authored_logical_source_argument_roles();
        if !complete {
            return None;
        }
        let argument_roles = roles
            .into_iter()
            .map(|(argument, role)| {
                let argument = self
                    .semantics
                    .argument_offset
                    .checked_add(usize::from(argument))?;
                (argument < count).then_some((argument, role))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(AuthoredLogicalSourceRoleProjection {
            argument_roles,
            case_bodies: None,
        })
    }

    pub(super) fn argument_roles_on_edge(&self, successful: bool) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_with_native_definition(successful, true)
    }

    fn argument_roles_with_native_definition(
        &self,
        successful: bool,
        native_definition: bool,
    ) -> (Vec<(u8, ArgRole)>, bool) {
        self.argument_roles_with_frame_layout(successful, native_definition, None)
    }

    fn argument_roles_with_frame_layout(
        &self,
        successful: bool,
        native_definition: bool,
        frame_layout: Option<crate::frame_effect::FrameArgumentResolution>,
    ) -> (Vec<(u8, ArgRole)>, bool) {
        let (mut arg_roles, mut arg_roles_complete) = if let Some(effect) =
            self.semantics.frame_effect
            && matches!(
                effect.layout,
                crate::FrameArgLayout::ScriptInSelectedFrame | crate::FrameArgLayout::AliasPairs
            ) {
            let layout = frame_layout.unwrap_or_else(|| {
                if successful {
                    effect.successful_layout(self.words.arguments()).layout
                } else {
                    effect.resolve_arguments(self.words.arguments())
                }
            });
            match layout {
                crate::frame_effect::FrameArgumentResolution::Valid { level_word_len, .. } => {
                    if effect.layout == crate::FrameArgLayout::AliasPairs {
                        let roles = (level_word_len + 1..self.words.arguments().len())
                            .step_by(2)
                            .filter_map(|index| {
                                u8::try_from(index)
                                    .ok()
                                    .map(|index| (index, ArgRole::VarWrite))
                            })
                            .collect();
                        (
                            roles,
                            self.words.arguments().len() <= usize::from(u8::MAX) + 1,
                        )
                    } else {
                        (
                            vec![(
                                u8::try_from(level_word_len)
                                    .expect("optional level has width zero or one"),
                                ArgRole::Body,
                            )],
                            true,
                        )
                    }
                }
                crate::frame_effect::FrameArgumentResolution::Invalid => (Vec::new(), true),
                crate::frame_effect::FrameArgumentResolution::Unknown => (Vec::new(), false),
            }
        } else {
            self.non_frame_argument_roles()
        };
        self.extend_repeated_roles(
            &mut arg_roles,
            &mut arg_roles_complete,
            successful,
            frame_layout,
        );
        match self.semantics.options.value_roles(
            self.words
                .arguments()
                .slice_from(self.semantics.argument_offset),
        ) {
            Some(roles) => {
                for (index, role) in roles {
                    if let Ok(index) = u8::try_from(index) {
                        if !arg_roles.contains(&(index, role)) {
                            arg_roles.push((index, role));
                        }
                    } else {
                        arg_roles_complete = false;
                    }
                }
            }
            None => arg_roles_complete = false,
        }
        if native_definition {
            self.extend_native_definition_roles(&mut arg_roles, &mut arg_roles_complete);
        }
        // Authored optional positions do not name cells when argv proves those
        // arguments absent. Expansion retains the unresolved obligations.
        if let Some(count) = self.words.arguments().exact_argv_len() {
            arg_roles.retain(|(index, _)| {
                self.semantics
                    .argument_offset
                    .checked_add(usize::from(*index))
                    .is_some_and(|index| index < count)
            });
        }
        (arg_roles, arg_roles_complete)
    }

    fn extend_native_definition_roles(
        &self,
        arg_roles: &mut Vec<(u8, ArgRole)>,
        arg_roles_complete: &mut bool,
    ) {
        if let Some(descriptor) = self.semantics.procedure_definition {
            match descriptor.select(self.words.arguments()) {
                crate::native_procedure::NativeProcedureDefinitionSelection::Valid(definition) => {
                    arg_roles.retain(|(_, role)| *role != ArgRole::Body);
                    arg_roles.push((
                        u8::try_from(definition.body_at).expect("native body position"),
                        ArgRole::Body,
                    ));
                }
                crate::native_procedure::NativeProcedureDefinitionSelection::Invalid => {
                    arg_roles.clear();
                }
                crate::native_procedure::NativeProcedureDefinitionSelection::Unknown => {
                    arg_roles.clear();
                    *arg_roles_complete = false;
                }
            }
        }
    }
}
