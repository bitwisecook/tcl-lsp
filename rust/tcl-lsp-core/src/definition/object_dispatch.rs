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

//! Method declarations a receiver dispatches to: class members, per-object state, visibility masks and self-dispatch.

use super::method_receivers::{enclosing_class_at, original_instance_method_at_cursor};
use super::variables::variable_scope_extent;
use super::{LspRange, span_to_range};
use crate::original_context::CurrentSourcePosition;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;
use tcl_registry::definer::DeclaredMemberVisibility;

/// Go-to-definition for an instance-method call site (`$obj method` /
/// `[$obj method]` / `my method`).  Extracted from [`definition`] to keep
/// that function within the line budget.
///
/// `None` means the cursor isn't on an instance-method call at all — the
/// caller falls through to the remaining checks (proc / class / alias).
/// `Some` is definitive: once the receiver's per-object override or class
/// is resolved, the call reaches exactly what this returns (possibly an
/// empty vector for an unexported/undefined method), never a same-named proc.
pub(super) fn instance_method_definition(
    analysis: &AnalysisResult,
    source: &str,
    line_index: &LineIndex,
    line: u32,
    character: u32,
) -> Option<Vec<LspRange>> {
    let cursor = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    if let Some(selected) = crate::receiver_identity::method_at_cursor(analysis, source, cursor) {
        return Some(vec![span_to_range(
            source,
            line_index,
            selected.method.name_span,
        )]);
    }
    if !analysis.allows_lexical_declaration_advice()
        && let Some((_, _, method)) = declared_self_dispatch_at(source, analysis, cursor)
    {
        return Some(vec![span_to_range(source, line_index, method.name_span)]);
    }
    None
}

/// Original linked command heads select their retained method declaration.
/// Constructor and destructor keywords retain their own declaration path.
pub(super) fn lookup_class_member(
    source: &str,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<tcl_lexer::Span> {
    if let Some(linked) = tcl_compiler::analyser::types::original_source_linked_member_at(
        analysis,
        source,
        cursor_offset,
    ) {
        return Some(linked.target().metadata().name_span);
    }
    let class_def = analysis
        .all_classes
        .get(enclosing_class_at(analysis, cursor_offset)?)?;
    // A constructor or destructor keyword: the member it declared says so.
    if let Some(c) = class_def
        .constructors
        .first()
        .filter(|c| c.is_declared_by_keyword(word))
    {
        if !c.name_span.is_empty() {
            return Some(c.name_span);
        }
        // Analyser doesn't store a name span for the
        // constructor keyword (it has no name token).
        // Fall back to the body span's start so the
        // editor at least lands on the constructor's
        // body opener.
        return Some(c.body_span);
    }
    if let Some(d) = class_def
        .destructor
        .as_ref()
        .filter(|d| d.is_declared_by_keyword(word))
    {
        if !d.name_span.is_empty() {
            return Some(d.name_span);
        }
        return Some(d.body_span);
    }
    None
}

/// The folded per-object member state for the receiver binding at the call
/// site: the [`tcl_compiler::analyser::ObjectMemberState`]
/// whose anchor resolves to the same variable binding as the call — the
/// same lexical variable-scope extent used by the object-state summary. `None`
/// when no `oo::objdefine` touched the receiver, or when the binding is
/// ambiguous (several same-scope states — abstain rather than guess).
pub(crate) fn object_member_state_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    receiver: &str,
    line: u32,
    character: u32,
) -> Option<&'a tcl_compiler::analyser::ObjectMemberState> {
    let states = analysis.object_member_state.get(receiver)?;
    let call_offset = CurrentSourcePosition::capture(source, analysis, line, character)?.offset();
    let call_scope = variable_scope_extent(analysis, receiver, call_offset);
    let matched: Vec<&tcl_compiler::analyser::ObjectMemberState> = states
        .iter()
        .filter(|st| variable_scope_extent(analysis, receiver, st.anchor_offset) == call_scope)
        .collect();
    match matched.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

/// Withhold workspace method fallback for a retained external visibility mask
/// or an unavailable current source query. Per-object source summaries remain
/// conditional source advice; they do not establish Native method visibility.
/// Missing source currency withholds fallback without asserting a hidden method.
#[must_use]
pub fn object_dispatch_mask(
    analysis: &AnalysisResult,
    source: &str,
    line: u32,
    character: u32,
) -> ExternalObjectDispatchMask {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let Some(position) = CurrentSourcePosition::capture(source, analysis, line, character) else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let cursor = position.offset();
    let mut mask = ExternalObjectDispatchMask::Clear;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if !command
                .argv
                .get(1)
                .is_some_and(|word| word.span.start() <= cursor && cursor < word.span.end())
            {
                return false;
            }
            mask = object_dispatch_mask_at_command(analysis, source, command, &current, cursor);
            true
        },
    );
    mask
}

/// Original visibility and conditional source advice stay separate. None of
/// these outcomes proves method execution or permission to resolve a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalObjectDispatchMask {
    /// This local source query supplies no visibility mask.
    Clear,
    /// A retained original Native receiver entry is not exported.
    Native,
    /// Selected per-object source configuration describes a possible mask.
    ConditionalSource,
    /// Current source or the mask's required original input is unavailable.
    Unavailable,
}

impl ExternalObjectDispatchMask {
    /// Withhold workspace selection for a mask or an unavailable source query.
    #[must_use]
    pub const fn withholds_workspace_fallback(self) -> bool {
        match self {
            Self::Clear => false,
            Self::Native | Self::ConditionalSource | Self::Unavailable => true,
        }
    }
}

fn object_dispatch_mask_at_command(
    analysis: &AnalysisResult,
    source: &str,
    command: &tcl_source::segment::SegmentedCommand,
    current: &crate::original_context::CurrentSourceContext<'_>,
    cursor: u32,
) -> ExternalObjectDispatchMask {
    use tcl_compiler::command_binding::SourceOriginKind;
    let Some(head) = command.argv.first() else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let binding = realm.invocation_at_source(command.name(), head.span.start());
    if !matches!(binding.source_origin().map(|origin| origin.kind()),
        Some(SourceOriginKind::Authored(bytes)) if bytes.as_ref() == source.as_bytes())
    {
        return ExternalObjectDispatchMask::Unavailable;
    }
    let image = tcl_lexer::SourceImage::document(source);
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        current.config(),
        command,
    );
    let Some(original) = tcl_compiler::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        head.span.start(),
        tokens.native_lexer_config(current.config()),
    ) else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    let Some(head) = tokens.word_exprs.first() else {
        return ExternalObjectDispatchMask::Unavailable;
    };
    if binding
        .receiver_self_method_entry(current.registry())
        .is_some()
        || original_internal_dispatch(source, analysis, command, current)
    {
        return ExternalObjectDispatchMask::Clear;
    }
    let variable_receiver = head.sole_variable_substitution().is_some();
    let accesses = variable_receiver
        .then(|| realm.variable_accesses_for_invocation_args(head.source().span.start()));
    let selected = if let Some(accesses) = &accesses {
        accesses
            .iter()
            .find_map(|access| binding.object_receiver_method_entry(access, head))
    } else {
        binding
            .frozen_object_receiver_method_entry(head)
            .or_else(|| binding.named_object_receiver_method_entry())
    };
    if let Some((class, entry, _)) = selected {
        if crate::receiver_identity::original_class(analysis, source, class).is_none()
            || !matches!(entry.declaration().source.kind(), SourceOriginKind::Authored(bytes)
                if bytes.as_ref() == source.as_bytes())
        {
            return ExternalObjectDispatchMask::Unavailable;
        }
        return if entry.is_exported() {
            ExternalObjectDispatchMask::Clear
        } else {
            ExternalObjectDispatchMask::Native
        };
    }
    if original
        .first()
        .is_some_and(|head| original_self_receiver_head(source, analysis, head, current))
    {
        // An original self result identifies the receiver only. External
        // visibility requires its independent actual method entry above.
        return ExternalObjectDispatchMask::Unavailable;
    }
    object_dispatch_configuration_mask(analysis, source, current, cursor)
}

fn object_dispatch_configuration_mask(
    analysis: &AnalysisResult,
    source: &str,
    current: &crate::original_context::CurrentSourceContext<'_>,
    cursor: u32,
) -> ExternalObjectDispatchMask {
    let position = LineIndex::new(source).position_at_utf16(cursor, source);
    let Some((receiver, method, _)) = original_instance_method_at_cursor(
        source,
        analysis,
        position.line,
        position.character.get(),
    ) else {
        return ExternalObjectDispatchMask::Clear;
    };
    let Some(state) = object_member_state_at(
        analysis,
        source,
        &receiver,
        position.line,
        position.character.get(),
    ) else {
        return if analysis.object_member_state.contains_key(&receiver) {
            ExternalObjectDispatchMask::Unavailable
        } else {
            ExternalObjectDispatchMask::Clear
        };
    };
    let same_namespace = analysis
        .original_namespace_scope_at(cursor)
        .zip(analysis.original_namespace_scope_at(state.anchor_offset))
        .is_some_and(|(call, anchor)| call == anchor);
    if !same_namespace
        || state.anchor_offset > cursor
        || state.conditional
        || !original_object_configuration_anchor(source, analysis, state.anchor_offset, current)
    {
        return ExternalObjectDispatchMask::Unavailable;
    }
    if state
        .methods
        .get(&method)
        .is_some_and(|method| method.visibility != DeclaredMemberVisibility::Public.as_str())
        || state.unexports.contains(&method)
    {
        ExternalObjectDispatchMask::ConditionalSource
    } else {
        ExternalObjectDispatchMask::Clear
    }
}

/// The summary's source anchor belongs to a selected per-object configuration.
/// This authenticates source advice, without identifying a Native receiver.
fn original_object_configuration_anchor(
    source: &str,
    analysis: &AnalysisResult,
    anchor: u32,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let context = current.context();
    let mut selected = false;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if !command.argv.iter().any(|word| word.span.start() == anchor) {
                return false;
            }
            let Some(words) = source_structure::source_registry_words(source, analysis, command)
            else {
                return false;
            };
            selected = words.with_source_schema(&context, |schema| {
                schema.authored_source_definition_body_grammar().is_some()
                    && schema.state_transitions().facts().iter().any(|fact| {
                        let tcl_registry::StateTransition::ObjectDispatch(
                            tcl_registry::ObjectDispatchTransition::Configure {
                                target,
                                layer: tcl_registry::ObjectDispatchLayer::Object,
                            },
                        ) = &fact.transition
                        else {
                            return false;
                        };
                        target
                            .argument_index()
                            .and_then(|index| words.original_argument_word(index))
                            .is_some_and(|word| word.span().start() == anchor)
                    })
            }) == Some(true);
            selected
        },
    );
    selected
}

/// Conditional internal dispatch through the original selected descriptor.
/// Self identity alone never changes an external call's visibility purpose.
pub(super) fn original_internal_dispatch(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_source::segment::SegmentedCommand,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let context = current.context();
    source_structure::source_registry_words(source, analysis, command).and_then(|words| {
        words.with_source_schema(&context, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        })
    }) == Some(true)
}

/// The analysed class whose body holds `offset`, when its declared member set
/// is complete. A definition vocabulary that may rewrite or add members (Clay)
/// records an incomplete set, which answers no dispatch.
pub(crate) fn declared_class_at(
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<&tcl_compiler::analyser::types::ClassDef> {
    analysis
        .all_classes
        .get(enclosing_class_at(analysis, offset)?)
        .filter(|class| !class.member_set_incomplete)
}

/// The declared member a written self-dispatch reaches in the analysed class
/// whose body holds `offset`: the receiving class, the providing class and
/// the method. A method body the analysis never entered still belongs to its
/// class, so hover and go-to-definition read the class's declared members
/// there. This names no receiver and grants no edit.
pub(crate) fn declared_self_dispatch_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> Option<(
    &'a tcl_compiler::analyser::types::ClassDef,
    &'a tcl_compiler::analyser::types::ClassDef,
    &'a tcl_compiler::analyser::types::MethodDef,
)> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let receiver = declared_class_at(analysis, offset)?;
    let mut selector = None;
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if !command
                .argv
                .get(1)
                .is_some_and(|word| word.span.start() <= offset && offset <= word.span.end())
            {
                return false;
            }
            if original_internal_dispatch(source, analysis, command, &current)
                || command_head_is_self_receiver(source, analysis, command, &current)
            {
                selector = command.texts.get(1).cloned();
            }
            selector.is_some()
        },
    );
    let method = selector?;
    let provider = analysis
        .class_hierarchy()
        .method_target(&receiver.qualified_name, &method)?;
    let provider = analysis.all_classes.get(provider)?;
    Some((receiver, provider, provider.methods.get(&method)?))
}

/// Whether this command's head word is a self-receiver substitution
/// (`[self] m`), read from the head's own descriptor.
fn command_head_is_self_receiver(
    source: &str,
    analysis: &AnalysisResult,
    command: &tcl_source::segment::SegmentedCommand,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    let Some(head) = command.argv.first() else {
        return false;
    };
    let image = tcl_lexer::SourceImage::document(source);
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(&image),
        current.config(),
        command,
    );
    tcl_compiler::registry_invocation::original_native_compiler_words(
        &image,
        tokens.words(),
        head.span.start(),
        tokens.native_lexer_config(current.config()),
    )
    .is_some_and(|words| {
        words
            .first()
            .is_some_and(|head| original_self_receiver_head(source, analysis, head, current))
    })
}

/// The selected source head may describe the current object's identity.
/// This shape grants no internal dispatcher or external visibility receipt.
fn original_self_receiver_head(
    source: &str,
    analysis: &AnalysisResult,
    head: &tcl_lexer::NativeWord,
    current: &crate::original_context::CurrentSourceContext<'_>,
) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    let context = current.context();
    let Some(child) = source_structure::original_single_source_command_substitution(head) else {
        return false;
    };
    let Some(head) = child.command().words.first() else {
        return false;
    };
    let Some(words) =
        source_structure::source_registry_words_at(source, analysis, head.span().start())
    else {
        return false;
    };
    if words.head_source().and_then(|head| head.word()) != Some(head) {
        return false;
    }
    words.with_source_schema(&context, |schema| {
        let Some(shape) = schema.authored_source_arity() else {
            return false;
        };
        if shape.command.self_receiver_words.is_empty() {
            return false;
        }
        match schema.words.arguments().exact_argv_len() {
            Some(0) => shape.arity.accepts(0),
            Some(1) => {
                shape.arity.accepts(1)
                    && shape.command.self_receiver_words.iter().any(|word| {
                        match schema.words.arguments().get(0) {
                            Some(tcl_registry::InvocationWord::Literal(value)) => value == *word,
                            Some(tcl_registry::InvocationWord::KnownBytes(value)) => {
                                value == word.as_bytes()
                            }
                            _ => false,
                        }
                    })
            }
            _ => false,
        }
    }) == Some(true)
}
