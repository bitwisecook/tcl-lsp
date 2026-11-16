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

//! Script operands in the authored source: frame scripts, script timing, executable values, command prefixes, retained callbacks and control arms.

use super::subcommand_resolution::SubcommandResolution;
use super::{AuthoredSourceDescriptors, ResolvedInvocation};
use crate::arg_role::ArgRole;
use crate::frame_effect::FrameLevel;
use crate::hooks::LoweringHookId;
use crate::hover::OptionSpec;
use crate::traits::Traits;
use crate::value_transfer::inputs::OperandId;

/// Selector layout of an immediate self-dispatch on the current object.
/// It describes source layout only; no method lookup or dispatch is proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredSourceSelfDispatch {
    /// The method selector is this one literal effective argument ordinal.
    Selector(usize),
    /// The selector is computed, expanded or missing.
    Unproven,
}

/// Source-value executable capability of one retained command schema.
/// This is neither handler identity nor execution, callback entry or Normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceExecutableValueCapability {
    /// Every authored role, form, option and prefix contract excludes code values.
    Absent,
    /// At least one authored layout can carry a script, lambda or command prefix.
    MayContain,
    /// Selection or the closed resolver vocabulary is unavailable.
    Unproven,
}

fn source_executable_traits(traits: Traits) -> bool {
    traits.intersects(Traits::EVALUATES_CODE | Traits::DEFERS_BODY | Traits::WRAPS_COMMAND_PREFIX)
}

fn source_option_has_executable_value(option: &OptionSpec) -> bool {
    option.value_role().is_some_and(ArgRole::has_script_timing)
        || option
            .value_also_role()
            .is_some_and(ArgRole::has_script_timing)
        || option.value_script_timing().is_some()
}

fn source_form_has_executable_value(form: &crate::forms::CommandForm) -> bool {
    form.arg_roles
        .iter()
        .any(|(_, role)| role.has_script_timing())
        || form.options.iter().any(source_option_has_executable_value)
        || form.traits.is_some_and(source_executable_traits)
}

fn source_sub_has_executable_value(sub: &crate::SubCommand) -> bool {
    sub.arg_roles
        .iter()
        .any(|(_, role)| role.has_script_timing())
        || sub
            .arg_role_resolver_roles
            .iter()
            .any(|role| role.has_script_timing())
        || sub
            .repeated_args
            .iter()
            .any(|layout| layout.role.has_script_timing())
        || sub.clause_grammar.is_some()
        || source_executable_traits(sub.traits)
        || sub.body_execution.is_some()
        || !sub.command_prefixes.is_empty()
        || sub.command_prefix_resolver.is_some()
        || sub.script_timing_resolver.is_some()
        || !sub.callback_taint_inputs.is_empty()
        || sub.options.iter().any(source_option_has_executable_value)
        || sub
            .subcommand_forms
            .iter()
            .any(source_form_has_executable_value)
        || sub.sub_subcommands.iter().any(|nested| {
            nested
                .options
                .is_some_and(|options| options.iter().any(source_option_has_executable_value))
        })
}

impl ResolvedInvocation<'_, '_> {
    /// The frame the call crosses into, and the operands that act there — the
    /// command's `FrameEffectSpec` read over the call's words under its
    /// [`Self::dialect`]'s release: `upvar`'s level decided by argument-count
    /// parity, `uplevel`'s by the leading word. The operands are the words
    /// after the level word.
    ///
    /// `None` when the command crosses no frame, or when a `{*}` expansion
    /// makes the level word's presence unknown. A computed level word is
    /// [`FrameLevel::Dynamic`], not an abstention, and so is a level whose
    /// spelling the releases read differently when the dialect names none.
    #[must_use]
    pub fn frame_effect(&self) -> Option<(FrameLevel, Vec<OperandId>)> {
        let frame = self.semantics.frame_effect?;
        let offset = self.semantics.argument_offset;
        let arguments = self.words.arguments().slice_from(offset);
        let len = arguments.exact_argv_len()?;
        let taken = frame.level_word_len_for_arguments(arguments)?;
        let level = if taken == 0 {
            FrameLevel::DEFAULT
        } else {
            arguments
                .literal_at(0)
                .and_then(|word| {
                    if let Some(dialect) = arguments.dialect() {
                        FrameLevel::parse_for_dialect(word, dialect)
                    } else {
                        FrameLevel::parse_for(word, self.tcl_version())
                    }
                })
                .unwrap_or(FrameLevel::Dynamic)
        };
        Some((
            level,
            (taken..len).map(|at| OperandId(offset + at)).collect(),
        ))
    }

    /// Possible source script tails under the selected frame descriptor.
    /// A computed leading probe retains both selector/no-selector alternatives;
    /// no frame, accepted level or successful body entry is established.
    #[must_use]
    pub fn authored_source_frame_script_arguments(&self) -> Option<Vec<std::ops::Range<usize>>> {
        self.source_frame_script_arguments(false)
    }

    /// Possible tails for a separately admitted Logical source model, using
    /// the existing unanimous authored frame grammar. This does not select
    /// Native level grammar, a physical frame or a successful body entry.
    #[must_use]
    pub fn authored_logical_source_frame_script_arguments(
        &self,
    ) -> Option<Vec<std::ops::Range<usize>>> {
        self.source_frame_script_arguments(true)
    }

    /// Conditional layout of the selected Logical frame descriptor under the
    /// unanimous authored grammar. This supplies no Native level, entered
    /// frame, successful argument acceptance or completed variable link.
    #[must_use]
    pub fn authored_logical_source_frame_argument_layout(
        &self,
    ) -> Option<crate::frame_effect::FrameArgumentResolution> {
        let frame = self.semantics.frame_effect?;
        let arguments = self
            .words
            .arguments()
            .slice_from(self.semantics.argument_offset);
        Some(frame.logical_source_layout(arguments))
    }

    fn source_frame_script_arguments(&self, logical: bool) -> Option<Vec<std::ops::Range<usize>>> {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        use crate::frame_effect::FrameArgumentResolution as Layout;
        let frame = self.semantics.frame_effect?;
        if frame.layout != crate::FrameArgLayout::ScriptInSelectedFrame {
            return None;
        }
        let offset = self.semantics.argument_offset;
        let arguments = self.words.arguments().slice_from(offset);
        let count = arguments.exact_argv_len()?;
        let layout = if logical {
            frame.logical_source_layout(arguments)
        } else {
            frame.resolve_arguments(arguments)
        };
        let starts = match layout {
            Layout::Valid { level_word_len, .. } => vec![level_word_len],
            Layout::Invalid => Vec::new(),
            Layout::Unknown if frame.level_word == crate::FrameLevelWord::LeadingProbe => {
                vec![0, 1]
            }
            Layout::Unknown => return None,
        };
        Some(
            starts
                .into_iter()
                .filter(|start| *start < count)
                .map(|start| offset + start..offset + count)
                .collect(),
        )
    }

    /// Selected dictionary mapping phases for conditional source analysis.
    /// Body shape/timing, source purpose and observer closure are independent;
    /// generic invocation effects and Native/Normal admission remain unchanged.
    #[must_use]
    pub fn authored_dictionary_scope_source_phase(
        &self,
    ) -> Option<crate::dictionary_scope::AuthoredDictionaryScopeSourcePhase> {
        crate::dictionary_scope::authored_dictionary_scope_source_phase(self)
    }

    /// Timing of an independently selected original script/container ordinal.
    /// This does not identify a body role, entered callback or executing frame.
    #[must_use]
    pub fn authored_source_script_timing_at(&self, argument: usize) -> Option<crate::ScriptTiming> {
        if self.semantics.procedure_definition.is_none()
            && let Some(positions) = self.authored_source_procedure_arguments()
            && argument == positions.body
        {
            return self.semantics.script_metadata.timing_at_with_default(
                self,
                argument,
                crate::ScriptTiming::Deferred,
            );
        }
        self.semantics.script_metadata.timing_at(self, argument)
    }

    /// Variable-name option scope at an original effective argument ordinal.
    /// The selected availability, prefix grammar and value widths are shared
    /// with roles; computed options or incomplete layouts refuse the query.
    /// This grants no variable lookup, store, entered frame or allocation.
    #[must_use]
    pub fn authored_source_option_variable_scope_at(
        &self,
        argument: usize,
    ) -> Option<crate::VariableScope> {
        let offset = self.semantics.argument_offset;
        let relative = argument.checked_sub(offset)?;
        let occurrences = self
            .semantics
            .options
            .prefix_occurrences(self.words.arguments().slice_from(offset))?;
        occurrences
            .iter()
            .find(|option| option.values.contains(&relative))?
            .option?
            .value_variable_scope()
    }

    /// Closed negative capability independent of ordinary data-role layout.
    /// The private selected command owner retains all authored forms and
    /// subcommands, so an empty positive position list never proves absence.
    /// Dynamic resolvers need their declared closed role vocabulary. This
    /// supplies no implementation, source frame, Native or successful handler.
    #[must_use]
    pub fn authored_source_executable_value_capability(&self) -> SourceExecutableValueCapability {
        use SourceExecutableValueCapability::{Absent, MayContain, Unproven};
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return Unproven;
        }
        let spec = self.selected.spec;
        if self.selected.form.is_none()
            && (!spec.command_forms.is_empty()
                || spec
                    .subcommands
                    .iter()
                    .any(|sub| !sub.subcommand_forms.is_empty())
                || self
                    .selected
                    .sub
                    .is_some_and(|sub| !sub.subcommand_forms.is_empty()))
        {
            return Unproven;
        }
        if [
            ArgRole::Body,
            ArgRole::LambdaLiteral,
            ArgRole::CommandPrefix,
        ]
        .into_iter()
        .any(|role| crate::CommandRegistry::spec_may_have_arg_role(spec, role))
            || source_executable_traits(spec.traits)
            || spec.body_execution.is_some()
            || spec.definition_body.is_some()
            || !spec.command_prefixes.is_empty()
            || spec.command_prefix_resolver.is_some()
            || spec.script_timing_resolver.is_some()
            || !spec.callback_taint_inputs.is_empty()
            || spec.options.iter().any(source_option_has_executable_value)
            || spec
                .command_forms
                .iter()
                .any(source_form_has_executable_value)
            || self
                .selected
                .form
                .is_some_and(source_form_has_executable_value)
            || spec.subcommands.iter().any(source_sub_has_executable_value)
            || self
                .selected
                .sub
                .is_some_and(source_sub_has_executable_value)
        {
            return MayContain;
        }
        if (spec.has_dynamic_argument_roles() && spec.arg_role_resolver_roles.is_empty())
            || spec.subcommands.iter().any(|sub| {
                sub.has_dynamic_argument_roles() && sub.arg_role_resolver_roles.is_empty()
            })
            || self.selected.sub.is_some_and(|sub| {
                sub.has_dynamic_argument_roles() && sub.arg_role_resolver_roles.is_empty()
            })
        {
            return Unproven;
        }
        Absent
    }

    /// Executable source-script positions from the selected descriptors and
    /// timing grammar. Reference-only positions, unresolved layouts and option
    /// widths decline. No reached callback, body frame or Normal is issued.
    #[must_use]
    pub fn authored_source_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_source_argument_roles();
        self.executable_source_script_arguments(&roles, complete)
    }

    /// Executable source positions under the separate positively retained
    /// Logical role purpose. Native frame grammar and execution remain unknown.
    #[must_use]
    pub fn authored_logical_source_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_logical_source_argument_roles();
        self.executable_source_script_arguments(&roles, complete)
    }

    /// Plain script or command-prefix positions from the selected source
    /// role, option and timing descriptors. Lambda lists retain their separate
    /// grammar; reference-only operands and unknown layouts remain excluded.
    /// This grants no callback entry, frame, native value or normal completion.
    #[must_use]
    pub fn authored_source_plain_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_source_argument_roles();
        self.semantics
            .script_metadata
            .plain_script_arguments(self, &roles, complete)
    }

    /// Plain script or command-prefix positions under the separate positively
    /// retained Logical role purpose. Lambda grammar and Native frame or effect
    /// facts remain independent of this readonly source projection.
    #[must_use]
    pub fn authored_logical_source_plain_script_arguments(&self) -> Option<Vec<usize>> {
        let (roles, complete) = self.authored_logical_source_argument_roles();
        self.semantics
            .script_metadata
            .plain_script_arguments(self, &roles, complete)
    }

    fn executable_source_script_arguments(
        &self,
        roles: &[(u8, ArgRole)],
        complete: bool,
    ) -> Option<Vec<usize>> {
        Some(
            self.semantics
                .script_metadata
                .script_arguments(self, roles, complete)?
                .into_iter()
                .filter_map(|(ordinal, timing)| {
                    (timing != crate::hover::ScriptTiming::ReferenceOnly).then_some(ordinal)
                })
                .collect(),
        )
    }

    /// Executable command-prefix positions from the selected authored source
    /// grammar. Unknown payloads keep their slots; selectors, option widths,
    /// expanded cardinality and reference-only timing retain their refusals.
    /// Effective ordinals include selected subcommands and captured prefixes.
    /// This grants no dispatch, callback entry, native value or normal completion.
    #[must_use]
    pub fn authored_source_command_prefix_arguments(
        &self,
    ) -> Option<Vec<(usize, crate::AppendedArity)>> {
        // naming.source.original-structured-script-timing
        // docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md
        let (roles, complete) = self.authored_source_argument_roles();
        self.semantics
            .script_metadata
            .prefix_arguments(self, &roles, complete)
    }

    /// Selector layout when this call is an immediate self-dispatch on the
    /// current object ([`Traits::TCLOO_SELF_DISPATCH`]). The trait fixes the
    /// layout: argument 0 selects a method and every later word is a value
    /// passed to it, whether or not the command's open subcommand vocabulary
    /// lists that selector, so an unlisted selector is not an unproven role
    /// layout here. A deferred capture of a selector (`callback`, a list built
    /// around the keyword) does not carry the trait. `None` means the call is
    /// not a self-dispatch. This grants no method lookup or dispatch.
    #[must_use]
    pub fn authored_source_self_dispatch(&self) -> Option<AuthoredSourceSelfDispatch> {
        if !self.semantics.traits.contains(Traits::TCLOO_SELF_DISPATCH) {
            return None;
        }
        Some(
            self.words
                .arguments()
                .literal_at(0)
                .map_or(AuthoredSourceSelfDispatch::Unproven, |_| {
                    AuthoredSourceSelfDispatch::Selector(0)
                }),
        )
    }

    /// Whether this selected call may keep a script or command prefix for
    /// later evaluation: a wrapped prefix, a stored script that does not
    /// become a definition body of its own
    /// ([`Traits::BODY_RUNS_IN_OWN_FRAME`]), or a command-prefix operand.
    /// A descriptor without any prefix-bearing position answers without a
    /// layout proof; otherwise the selected layout must place no prefix
    /// operand. Unresolved selectors and unproven layouts answer `None`.
    /// Deferred script-valued options (a Tk `-command` script) are not read
    /// here: they are Body roles, which checked syntax coverage walks when
    /// literal and refuses when dynamic, so a consumer that relies on this
    /// answer must also require that coverage. This grants no callback
    /// entry, frame or dispatch.
    #[must_use]
    pub fn authored_source_retains_callback(&self) -> Option<bool> {
        let traits = self.semantics.traits;
        if traits.contains(Traits::WRAPS_COMMAND_PREFIX)
            || (traits.contains(Traits::DEFERS_BODY)
                && !traits.contains(Traits::BODY_RUNS_IN_OWN_FRAME))
        {
            return Some(true);
        }
        if matches!(
            self.subcommand,
            SubcommandResolution::Indeterminate { .. }
                | SubcommandResolution::Unknown { .. }
                | SubcommandResolution::Ambiguous { .. }
        ) {
            return None;
        }
        let AuthoredSourceDescriptors {
            command,
            subcommand,
        } = self.authored_source_descriptors();
        let (prefixes, resolver) = subcommand.map_or(
            (command.command_prefixes, command.command_prefix_resolver),
            |sub| (sub.command_prefixes, sub.command_prefix_resolver),
        );
        let (roles, _) = self.authored_source_argument_roles();
        let possible = !prefixes.is_empty()
            || resolver.is_some()
            || roles
                .iter()
                .chain(self.semantics.arg_roles)
                .any(|&(_, role)| role == ArgRole::CommandPrefix)
            || self
                .semantics
                .arg_role_resolver_roles
                .contains(&ArgRole::CommandPrefix)
            || self
                .semantics
                .options
                .available()
                .any(|option| option.value_role() == Some(ArgRole::CommandPrefix));
        if !possible {
            return Some(false);
        }
        self.authored_source_command_prefix_arguments()
            .map(|positions| !positions.is_empty())
    }

    /// Command-prefix positions under the independently retained Logical source
    /// grammar. Complete roles, options and timing remain mandatory; this grants
    /// no callback entry, physical frame, Native value or normal completion.
    #[must_use]
    pub fn authored_logical_source_command_prefix_arguments(
        &self,
    ) -> Option<Vec<(usize, crate::AppendedArity)>> {
        let (roles, complete) = self.authored_logical_source_argument_roles();
        self.semantics
            .script_metadata
            .prefix_arguments(self, &roles, complete)
    }

    /// Structural control semantics for one effective original body ordinal.
    /// Complete source roles and the selected clause grammar are mandatory;
    /// this proves no test value, body entry, dispatch or Normal completion.
    #[must_use]
    pub fn authored_source_control_arm_semantics(
        &self,
        argument: usize,
    ) -> Option<crate::registry::ControlArmSemantics> {
        self.source_control_arm_semantics(argument, false)
    }

    /// Structural body semantics under an independently retained Logical
    /// source model. This supplies no Native frame or execution authority.
    #[must_use]
    pub fn authored_logical_source_control_arm_semantics(
        &self,
        argument: usize,
    ) -> Option<crate::registry::ControlArmSemantics> {
        self.source_control_arm_semantics(argument, true)
    }

    fn source_control_arm_semantics(
        &self,
        argument: usize,
        logical: bool,
    ) -> Option<crate::registry::ControlArmSemantics> {
        use crate::registry::ControlArmSemantics;
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
        if argument >= count
            || !self
                .semantics
                .arity
                .accepts(self.argument_count_for_arity()?)
        {
            return None;
        }
        let roles = if logical {
            let projection = self.authored_logical_source_role_projection()?;
            if projection.selection_unknown() {
                return None;
            }
            projection.argument_roles
        } else {
            let (roles, complete) = self.authored_source_argument_roles();
            if !complete {
                return None;
            }
            roles
                .into_iter()
                .map(|(ordinal, role)| {
                    let ordinal = self
                        .semantics
                        .argument_offset
                        .checked_add(usize::from(ordinal))?;
                    (ordinal < count).then_some((ordinal, role))
                })
                .collect::<Option<Vec<_>>>()?
        };
        let hook = self.semantics.lowering_hook?;
        let role = if hook == LoweringHookId::Apply {
            ArgRole::LambdaLiteral
        } else {
            ArgRole::Body
        };
        roles.contains(&(argument, role)).then_some(())?;
        match hook {
            LoweringHookId::If => self
                .clause_plan()?
                .defect
                .is_none()
                .then_some(ControlArmSemantics::Selected),
            LoweringHookId::Apply => Some(ControlArmSemantics::FrameBoundary),
            LoweringHookId::Switch => {
                self.authored_source_case_invocation()?;
                crate::registry::control_arm_semantics_for_hook(hook)
            }
            LoweringHookId::Try => {
                crate::registry::selected_try_control_arms(&self.clause_plan()?, arguments)?
                    .into_iter()
                    .find_map(|(ordinal, semantics)| (ordinal == argument).then_some(semantics))
            }
            hook => crate::registry::control_arm_semantics_for_hook(hook),
        }
    }
}
