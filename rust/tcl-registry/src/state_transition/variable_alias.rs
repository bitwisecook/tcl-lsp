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

//! Variable-cell aliasing (`upvar`, `global`, `variable`, `namespace upvar`): the transition, its frames and the resolvers that pair alias words.

use super::descriptor::StateTransitionResolver;
use super::{StateTransition, StateTransitionDomain, StateTransitions, TransitionSubject};
use crate::invocation_words::{InvocationArguments, InvocationWordKind};

/// Which caller-frame cell an `upvar`-style transition addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerFrameSelection {
    /// Tcl's implicit level: the immediate caller frame.
    DefaultCaller,
    /// An explicit level word selects the frame at runtime.
    Explicit(TransitionSubject),
}

/// The target cell of a local variable alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableAliasTarget {
    /// A variable in the global namespace.
    Global {
        /// Global variable name.
        variable: TransitionSubject,
    },
    /// A variable in the current namespace.
    CurrentNamespace {
        /// Namespace-relative or qualified variable name.
        variable: TransitionSubject,
    },
    /// A variable in a frame selected by an `upvar` level word.
    CallerSelectedFrame {
        /// The selected caller frame.
        frame: CallerFrameSelection,
        /// Variable name in that frame.
        variable: TransitionSubject,
    },
    /// A variable in an explicitly named namespace.
    Namespace {
        /// Namespace containing the target cell.
        namespace: TransitionSubject,
        /// Variable name within that namespace.
        variable: TransitionSubject,
    },
}

/// Storage selected for a variable-alias destination name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableAliasDestination {
    /// A procedure local-table link. In C Tcl global and namespace evaluation
    /// frames this declaration has no effect.
    ProcedureLocal,
    /// An unqualified name binds locally in a procedure activation; namespace
    /// activations and qualified names create a namespace slot without the
    /// ordinary read lookup's global fallback.
    CurrentNamespaceOrLocal,
}

/// Actual activation class consumed by variable-alias declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableAliasFrame {
    /// Interpreter root activation with no local variable table.
    Global,
    /// A namespace-evaluation activation.
    Namespace,
    /// A procedure or method activation owning a local table.
    Procedure,
    /// A caller-selected activation whose class is not proved.
    Unknown,
}

impl VariableAliasDestination {
    /// Whether this destination grammar acts in the selected activation.
    /// Unknown frame or dialect policy never silently selects C Tcl rules.
    #[must_use]
    pub fn is_active_in_frame(
        self,
        frame: VariableAliasFrame,
        dialect: crate::InvocationDialect,
    ) -> Option<bool> {
        self.is_active_in_frame_with_policy(frame, Some(dialect))
    }

    /// The same activation query before an execution dialect is selected.
    /// Frame-independent facts remain usable; namespace policy abstains.
    #[must_use]
    pub fn is_active_in_frame_with_policy(
        self,
        frame: VariableAliasFrame,
        dialect: Option<crate::InvocationDialect>,
    ) -> Option<bool> {
        match (self, frame) {
            (_, VariableAliasFrame::Unknown) => None,
            (Self::CurrentNamespaceOrLocal, _)
            | (Self::ProcedureLocal, VariableAliasFrame::Procedure) => Some(true),
            (Self::ProcedureLocal, VariableAliasFrame::Global) => Some(false),
            (Self::ProcedureLocal, VariableAliasFrame::Namespace) => dialect
                .and_then(|dialect| dialect.variable_lookup_policy)
                .map(|policy| policy == tcl_dialect::VariableLookupPolicy::Jim),
        }
    }
}

/// A local variable cell bound to another Tcl cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCellAliasTransition {
    /// The destination storage selected by this declaration grammar.
    pub destination: VariableAliasDestination,
    /// The current-frame local variable name.
    pub local: TransitionSubject,
    /// The cell reached through `local`.
    pub target: VariableAliasTarget,
    /// Whether this invocation also writes a value through the aliased cell.
    ///
    /// Alias establishment alone is declaration state, but some invocation
    /// forms establish the alias and initialise the target in the same
    /// operation.  Consumers must not infer that distinction from a command
    /// spelling or argument layout.
    pub writes_value: bool,
    /// Where the local and the target's variable name are spelled.
    pub words: AliasWords,
}

/// Where an alias fact's two names are spelled in its invocation.
///
/// A [`TransitionSubject::Literal`] keeps a word's Tcl value but not its
/// place, so a consumer that anchors the alias in source — the analyser's
/// definition of the local, and the word a rename of the aliased cell
/// rewrites — reads the place here. Both are post-head argument indices,
/// counted as [`TransitionSubject::Unknown`] counts them (a subcommand word
/// included). `global` and `variable` name the local and the cell with one
/// word, so the two indices are equal there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AliasWords {
    /// The word spelling the current-frame local.
    pub local: usize,
    /// The word spelling the target cell's variable name.
    pub target: usize,
}

impl AliasWords {
    /// One word names both the local and the cell it aliases.
    #[must_use]
    pub const fn same(index: usize) -> Self {
        Self {
            local: index,
            target: index,
        }
    }
}

/// The domains a variable-cell alias can invalidate: which cell a name
/// reaches, and the traces that fire on it — what `upvar`, `global` and
/// `variable` widen for a dynamic operand, and what an abstained alias fact
/// widens.
pub const VARIABLE_ALIAS_DOMAINS: &[StateTransitionDomain] = &[
    StateTransitionDomain::VariableCells,
    StateTransitionDomain::VariableTraces,
];

/// The resolver a `state_transitions … resolver from-frame-effect`
/// declaration derives from a command's `frame_effect` of
/// [`crate::frame_effect::FrameArgLayout::AliasPairs`] layout, one per
/// level-word policy (`docs/design/spec-dsl-examples/README.md`,
/// "Derivations, exactly"): the level word is located as the policy says,
/// and each `otherVar myVar` pair after it becomes a
/// [`VariableCellAliasTransition`] against the frame the level selects.
///
/// Two abstentions are the derivation's own, and each widens
/// [`VARIABLE_ALIAS_DOMAINS`] for the word it could not read rather than
/// stating a narrower fact: a dynamic level word aborts the whole
/// derivation (no alias for the call, not "assume the default frame"), and
/// a dynamic member of a pair skips that pair while the pairs after it still
/// resolve. A call whose word count an expansion hides states nothing.
#[must_use]
pub fn alias_pairs_resolver(
    level_word: crate::frame_effect::FrameLevelWord,
) -> StateTransitionResolver {
    use crate::frame_effect::FrameLevelWord;
    match level_word {
        FrameLevelWord::None => alias_pairs_without_level,
        FrameLevelWord::ArityParity => alias_pairs_by_arity_parity,
        FrameLevelWord::LeadingProbe => alias_pairs_by_leading_probe,
        FrameLevelWord::Upvar => alias_pairs_by_upvar,
    }
}

/// Source resolver for the same authored derived alias layout. Native call
/// context is deliberately absent only inside this positively selected purpose.
#[must_use]
pub fn logical_source_alias_pairs_resolver(
    level_word: crate::frame_effect::FrameLevelWord,
) -> StateTransitionResolver {
    use crate::frame_effect::FrameLevelWord;
    match level_word {
        FrameLevelWord::None => logical_alias_pairs_without_level,
        FrameLevelWord::ArityParity => logical_alias_pairs_by_arity_parity,
        FrameLevelWord::LeadingProbe => logical_alias_pairs_by_leading_probe,
        FrameLevelWord::Upvar => logical_alias_pairs_by_upvar,
    }
}

fn source_arguments(arguments: InvocationArguments<'_>) -> InvocationArguments<'_> {
    match arguments {
        InvocationArguments::Literals(words)
        | InvocationArguments::ContextualLiterals(words, _) => InvocationArguments::Literals(words),
        InvocationArguments::Structured(words)
        | InvocationArguments::ContextualStructured(words, _) => {
            InvocationArguments::Structured(words)
        }
    }
}

fn logical_alias_pairs_without_level(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs_with_source_target(
        source_arguments(arguments),
        crate::frame_effect::FrameLevelWord::None,
        true,
    )
}

fn logical_alias_pairs_by_arity_parity(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs_with_source_target(
        source_arguments(arguments),
        crate::frame_effect::FrameLevelWord::ArityParity,
        true,
    )
}

fn logical_alias_pairs_by_leading_probe(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs_with_source_target(
        source_arguments(arguments),
        crate::frame_effect::FrameLevelWord::LeadingProbe,
        true,
    )
}

fn logical_alias_pairs_by_upvar(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs_with_source_target(
        source_arguments(arguments),
        crate::frame_effect::FrameLevelWord::Upvar,
        true,
    )
}

fn alias_pairs_without_level(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs(arguments, crate::frame_effect::FrameLevelWord::None)
}

fn alias_pairs_by_arity_parity(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs(arguments, crate::frame_effect::FrameLevelWord::ArityParity)
}

fn alias_pairs_by_leading_probe(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs(arguments, crate::frame_effect::FrameLevelWord::LeadingProbe)
}

fn alias_pairs_by_upvar(arguments: InvocationArguments<'_>) -> StateTransitions {
    alias_pairs(arguments, crate::frame_effect::FrameLevelWord::Upvar)
}

fn alias_pairs(
    arguments: InvocationArguments<'_>,
    level_word: crate::frame_effect::FrameLevelWord,
) -> StateTransitions {
    alias_pairs_with_source_target(arguments, level_word, false)
}

// Only the positively selected Logical source resolver can retain an unknown
// target as conditional source geometry. Its original widening remains intact;
// an unknown local or frame never supplies a declaration or selected link.
fn alias_pairs_with_source_target(
    arguments: InvocationArguments<'_>,
    level_word: crate::frame_effect::FrameLevelWord,
    retain_source_target: bool,
) -> StateTransitions {
    use crate::frame_effect::{FrameArgLayout, FrameEffectSpec};
    let mut transitions = StateTransitions::default();
    let Some(len) = arguments.exact_argv_len() else {
        return transitions;
    };
    let frame = FrameEffectSpec {
        level_word,
        layout: FrameArgLayout::AliasPairs,
    };
    let taken = if let Some(taken) = frame.level_word_len_for_arguments(arguments) {
        taken
    } else {
        if let Some(subject) = TransitionSubject::from_argument(arguments, 0) {
            transitions.widen(subject, VARIABLE_ALIAS_DOMAINS);
        }
        // A conditional source role is not a pre-dispatch layout or frame.
        if !retain_source_target
            || frame
                .conditional_no_level_source_layout(arguments)
                .is_none()
        {
            return transitions;
        }
        0
    };
    let level = match taken {
        0 => CallerFrameSelection::DefaultCaller,
        _ => match TransitionSubject::from_argument(arguments, 0) {
            Some(level) if level.literal().is_some() || level.native_bytes().is_some() => {
                CallerFrameSelection::Explicit(level)
            }
            Some(dynamic) => {
                transitions.widen(dynamic, VARIABLE_ALIAS_DOMAINS);
                return transitions;
            }
            None => return transitions,
        },
    };
    for other in (taken..len).step_by(2) {
        let (Some(variable), Some(local)) = (
            TransitionSubject::from_argument(arguments, other),
            TransitionSubject::from_argument(arguments, other + 1),
        ) else {
            continue;
        };
        if let Some(dynamic) = [&variable, &local]
            .into_iter()
            .find(|subject| subject.literal().is_none() && subject.native_bytes().is_none())
        {
            transitions.widen(dynamic.clone(), VARIABLE_ALIAS_DOMAINS);
            if !retain_source_target
                || (local.literal().is_none() && local.native_bytes().is_none())
            {
                continue;
            }
        }
        transitions.push(StateTransition::VariableCellAlias(
            VariableCellAliasTransition {
                destination: VariableAliasDestination::CurrentNamespaceOrLocal,
                local,
                target: VariableAliasTarget::CallerSelectedFrame {
                    frame: level.clone(),
                    variable,
                },
                writes_value: false,
                words: AliasWords {
                    local: other + 1,
                    target: other,
                },
            },
        ));
    }
    transitions
}

/// The operation selecting a local variable alias name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableAliasNamePurpose {
    /// A global-variable link; Jim leaves rooted names without a local link.
    Global,
    /// A namespace-variable link, with the engine's namespace-tail rules.
    NamespaceVariable,
}

/// Project an original alias operand through the selected native naming owner.
/// Missing policies and non-Unicode projections remain opaque; no native
/// lookup, successful link, physical cell or compiler permission follows.
/// `None` means the selected operation creates no local alias.
#[must_use]
pub fn local_alias_name(
    subject: &TransitionSubject,
    argument_index: usize,
    purpose: VariableAliasNamePurpose,
    dialect: Option<crate::InvocationDialect>,
) -> Option<TransitionSubject> {
    if let TransitionSubject::LocatedNativeBytes {
        value: name,
        argument_index,
    } = subject
    {
        let argument_index = *argument_index;
        let selected = match dialect.and_then(crate::InvocationDialect::native_name_protocol) {
            Some(protocol) => match purpose {
                VariableAliasNamePurpose::Global => {
                    tcl_syntax::naming::global_local_name_bytes(protocol, name)?
                }
                VariableAliasNamePurpose::NamespaceVariable => {
                    tcl_syntax::naming::variable_local_name_bytes(protocol, name)
                }
            },
            None if dialect.is_some_and(|dialect| {
                dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            }) =>
            {
                tcl_syntax::naming::c_family_local_alias_name_bytes(name)?
            }
            None => {
                return Some(TransitionSubject::Unknown {
                    argument_index,
                    word_kind: InvocationWordKind::KnownBytes,
                });
            }
        };
        return Some(TransitionSubject::LocatedNativeBytes {
            value: selected,
            argument_index,
        });
    }
    let Some(name) = subject.literal() else {
        return Some(subject.clone());
    };
    let opaque = || TransitionSubject::Unknown {
        argument_index,
        word_kind: InvocationWordKind::Opaque,
    };
    let Some(protocol) = dialect.and_then(crate::InvocationDialect::native_name_protocol) else {
        if dialect.is_some_and(|dialect| dialect.family() == Some(tcl_dialect::model::Family::Tcl))
        {
            return Some(
                tcl_syntax::naming::c_family_local_alias_name_bytes(name.as_bytes())
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .map_or_else(opaque, |value| {
                        subject
                            .with_literal_value(value)
                            .expect("known alias value")
                    }),
            );
        }
        return Some(opaque());
    };
    let selected = match purpose {
        VariableAliasNamePurpose::Global => {
            tcl_syntax::naming::global_local_name_bytes(protocol, name.as_bytes())?
        }
        VariableAliasNamePurpose::NamespaceVariable => {
            tcl_syntax::naming::variable_local_name_bytes(protocol, name.as_bytes())
        }
    };
    Some(String::from_utf8(selected).map_or_else(
        |_| opaque(),
        |value| {
            subject
                .with_literal_value(value)
                .expect("known alias value")
        },
    ))
}
