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

//! Variable completion from source names and genuine lexical frames. Candidates
//! do not establish live cells, successful reads or executed alias links.

use super::{CompletionEdit, CompletionItem, CompletionKind};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::command_binding::SourceOriginalVariableFrame;
use tcl_compiler::signature_scan::scope::SignatureNamespaceScope;
use tcl_compiler::signature_scan::variable_name::SignatureSourceVariableRoot;
use tcl_compiler::signature_scan::variable_symbol::{
    OriginalVariableSymbolReceiver, SignatureSourceVariableOccurrence, SignatureSourceVariableSlot,
};
use tcl_compiler::var_resolve::VariableExecutionFrame;
use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::naming::{NamePolicyProtocol, NativeVariableRootGeometry};

pub(super) fn items(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    prefix: &tcl_lexer::word_parts::VariableCursorPrefix,
) -> Option<Vec<CompletionItem>> {
    if analysis.allows_retained_logical_declaration_advice() {
        return None;
    }
    let Some(config) = analysis.body_lexer_config else {
        return Some(Vec::new());
    };
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return Some(Vec::new());
    }
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    if !prefix.matches_source(&image, config) || prefix.reference_span().end() != cursor {
        return Some(Vec::new());
    }
    let Some(partial) = source.get(prefix.partial_span().as_range()) else {
        return Some(Vec::new());
    };
    let expected = if prefix.is_braced() { "${" } else { "$" };
    let begin = index.position_at_utf16(prefix.reference_span().start(), source);
    let finish = index.position_at_utf16(prefix.completion_replacement_span().end(), source);
    if begin.line != line || finish.line != line {
        return Some(Vec::new());
    }
    let mut candidates = reference_candidates(
        source,
        analysis,
        cursor,
        prefix.is_braced(),
        prefix.index_span().is_some(),
        Some(prefix.reference_span()),
    );
    // Braces retain a combined name: the actual candidate owner decides whether
    // its parentheses denote an element rather than treating typed '(' as one.
    if prefix.is_braced() {
        candidates.extend(reference_candidates(
            source,
            analysis,
            cursor,
            true,
            true,
            Some(prefix.reference_span()),
        ));
        candidates.sort_by(|left, right| left.insert_text.cmp(&right.insert_text));
        candidates.dedup_by(|left, right| left.insert_text == right.insert_text);
    }
    if prefix.index_span().is_none() {
        let frame = analysis.original_variable_frame_in_source(&image, config, cursor);
        extend_loop_list_candidates(
            source,
            analysis,
            &image,
            config,
            frame.as_ref(),
            prefix.is_braced(),
            &mut candidates,
        );
    }
    for item in &mut candidates {
        item.text_edit = Some(CompletionEdit {
            start_char: begin.character.get(),
            end_char: finish.character.get(),
            new_text: item.insert_text.clone(),
        });
    }
    candidates.sort_by(|left, right| left.label.cmp(&right.label));
    let super::FilteredCandidates {
        mut candidates,
        fuzzy,
    } = super::filter_candidates(partial, candidates, |item| {
        item.label
            .strip_prefix("${")
            .or_else(|| item.label.strip_prefix('$'))
            .unwrap_or(&item.label)
    });
    if fuzzy {
        super::decorate_fuzzy_items(&mut candidates, &format!("{expected}{partial}"));
    }
    normalise_reference_labels(&mut candidates);
    Some(candidates)
}

/// Labels describe a source name; only the separately validated edit is Tcl.
fn normalise_reference_labels(items: &mut [CompletionItem]) {
    let labels: Vec<String> = items
        .iter()
        .map(|item| {
            item.label
                .strip_prefix("${")
                .and_then(|name| name.strip_suffix('}'))
                .map_or_else(|| item.label.clone(), |name| format!("${name}"))
        })
        .collect();
    let mut edits = std::collections::HashMap::<String, Option<String>>::new();
    for (item, label) in items.iter().zip(&labels) {
        edits
            .entry(label.clone())
            .and_modify(|old| {
                if old.as_deref() != Some(item.insert_text.as_str()) {
                    *old = None;
                }
            })
            .or_insert_with(|| Some(item.insert_text.clone()));
    }
    for (item, label) in items.iter_mut().zip(&labels) {
        if edits.get(label.as_str()).is_some_and(Option::is_some) && item.label != *label {
            if item.filter_text.is_none() {
                item.filter_text = Some(item.label.clone());
            }
            item.label.clone_from(label);
        }
    }
}

/// Complete source spellings for snippet choices through the same actual
/// visibility and decoder owner as variable completion. No live read is claimed.
pub(super) fn source_references(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Vec<String>> {
    if analysis.allows_retained_logical_declaration_advice() {
        return None;
    }
    let mut names = reference_candidates(source, analysis, cursor, false, false, None)
        .into_iter()
        .map(|item| item.insert_text)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Some(names)
}

fn reference_candidates(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
    braced: bool,
    arrays: bool,
    excluded: Option<Span>,
) -> Vec<CompletionItem> {
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return Vec::new();
    }
    let frame = analysis.original_variable_frame_in_source(&image, config, cursor);
    let namespace = analysis.original_namespace_scope_at(cursor);
    if analysis.has_original_vendor_source_names() {
        return vendor_reference_candidates(analysis, (&image, config), cursor, excluded);
    }
    let mut candidates = Vec::new();
    extend_symbol_reference_candidates(
        analysis,
        (source, config),
        (namespace, frame.as_ref()),
        braced,
        arrays,
        excluded,
        &mut candidates,
    );
    if !arrays {
        extend_alias_reference_candidates(
            analysis,
            (source, &image, config),
            frame.as_ref(),
            braced,
            excluded,
            &mut candidates,
        );
        extend_write_reference_candidates(
            analysis,
            (source, config),
            (namespace, frame.as_ref()),
            braced,
            excluded,
            &mut candidates,
        );
    }
    candidates
}

// This receipt holds the applicability-bearing registry owner alongside the
// original body occurrence and field; it is only projected as conditional UI.
struct ConditionalLoopListField {
    command: tcl_compiler::command_binding::OriginalNativeProcedureBodyCommand,
    words: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    argument: usize,
    ordinal: usize,
    field: tcl_syntax::word_rules::OriginalSourceListElement,
    frame: SourceOriginalVariableFrame,
}

// A list field is authored source evidence only: it supplies neither a
// variable cell nor proof that the loop executes or writes that field.
fn extend_loop_list_candidates(
    source: &str,
    analysis: &AnalysisResult,
    image: &SourceImage,
    config: LexerConfig,
    frame: Option<&SourceOriginalVariableFrame>,
    braced: bool,
    candidates: &mut Vec<CompletionItem>,
) {
    let Some(frame) = frame else { return };
    let receipts = collect_loop_list_fields(source, analysis, image, config, frame);
    for receipt in receipts {
        let Some(word) = receipt.words.original_argument_word(receipt.argument) else {
            continue;
        };
        let Some(fields) = tcl_syntax::word_rules::original_static_word_list_elements(word) else {
            continue;
        };
        if receipt.command.caller_declaration(analysis).is_none()
            || fields.get(receipt.ordinal) != Some(&receipt.field)
            || &receipt.frame != frame
            || !receipt.words.roles().is_some_and(|roles| {
                roles.contains(&(receipt.argument, tcl_registry::ArgRole::LoopVarList))
            })
        {
            continue;
        }
        let Some(policy) = receipt
            .words
            .dialect()
            .and_then(tcl_registry::InvocationDialect::authored_name_policy)
        else {
            continue;
        };
        let Some(spelling) = reference_spelling(
            receipt.field.value().as_bytes(),
            None,
            braced,
            config,
            policy,
        ) else {
            continue;
        };
        if candidates.iter().any(|item| item.insert_text == spelling) {
            continue;
        }
        candidates.push(CompletionItem {
            label: spelling.clone(),
            insert_text: spelling,
            kind: CompletionKind::Variable,
            detail: Some("variable — conditional loop-list source candidate".to_owned()),
            ..CompletionItem::default()
        });
    }
}

fn collect_loop_list_fields(
    source: &str,
    analysis: &AnalysisResult,
    image: &SourceImage,
    config: LexerConfig,
    frame: &SourceOriginalVariableFrame,
) -> Vec<ConditionalLoopListField> {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    let Some(commands) = source_structure::source_native_procedure_body_commands(source, analysis)
    else {
        return Vec::new();
    };
    let mut receipts = Vec::new();
    for command in commands {
        if command.caller_declaration(analysis).is_none() {
            continue;
        }
        let original = command.original_words();
        let Some(head) = original.first() else {
            continue;
        };
        let Some(words) =
            source_structure::source_registry_words_at(source, analysis, head.span().start())
        else {
            continue;
        };
        if !matches!(
            words.source(),
            source_structure::OriginalRegistrySource::Selected
                | source_structure::OriginalRegistrySource::SourceTransitions(_)
        ) || words.head_source().and_then(|head| head.word()) != Some(head)
            || words.origins().len() != original.len()
            || words.arguments().len() + 1 != original.len()
            || !words
                .origins()
                .iter()
                .enumerate()
                .skip(1)
                .all(|(index, origin)| {
                    *origin == InvocationWordOrigin::Written(index)
                        && words.original_argument_word(index - 1) == original.get(index)
                })
        {
            continue;
        }
        for &(index, role) in words.roles().unwrap_or_default() {
            if role != tcl_registry::ArgRole::LoopVarList {
                continue;
            }
            let Some(word) = words.original_argument_word(index) else {
                continue;
            };
            let Some(fields) = tcl_syntax::word_rules::original_static_word_list_elements(word)
            else {
                continue;
            };
            for (ordinal, field) in fields.into_iter().enumerate() {
                let Some(span) = field.source_span() else {
                    continue;
                };
                let name = field.value();
                let mut bytes = name.bytes();
                if !bytes
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
                    || !bytes.all(|c| c.is_ascii_alphanumeric() || c == b'_')
                    || source.get(span.as_range()) != Some(name)
                    || analysis
                        .original_variable_frame_in_source(image, config, span.start())
                        .as_ref()
                        != Some(frame)
                {
                    continue;
                }
                receipts.push(ConditionalLoopListField {
                    command: command.clone(),
                    words: words.clone(),
                    argument: index,
                    ordinal,
                    field,
                    frame: frame.clone(),
                });
            }
        }
    }
    receipts
}

fn vendor_reference_candidates(
    analysis: &AnalysisResult,
    position: (&SourceImage, LexerConfig),
    cursor: u32,
    excluded: Option<Span>,
) -> Vec<CompletionItem> {
    let (image, config) = position;
    let home = analysis.original_vendor_variable_body_in_source(image, config, cursor);
    let mut candidates = Vec::new();
    for advice in analysis.original_vendor_variable_advice() {
        let visible = match (home, advice.home()) {
            (Some(current), Some(owner)) => current == owner,
            (None, None) => advice.is_source_root(),
            _ => false,
        };
        if !visible
            || excluded.is_some_and(|span| {
                advice.span().start() <= span.start() && span.end() <= advice.span().end()
            })
            || !advice
                .input()
                .original_occurrence()
                .name_input()
                .matches_source(image, config)
        {
            continue;
        }
        let Some(spelling) =
            tcl_compiler::signature_scan::vendor_variable::vendor_variable_source_reference(advice)
        else {
            continue;
        };
        if candidates
            .iter()
            .any(|item: &CompletionItem| item.insert_text == spelling)
        {
            continue;
        }
        candidates.push(CompletionItem {
            label: spelling.clone(),
            insert_text: spelling.clone(),
            kind: CompletionKind::Variable,
            detail: Some("variable — hosted source declaration candidate".to_owned()),
            ..CompletionItem::default()
        });
    }
    candidates
}

fn extend_symbol_reference_candidates(
    analysis: &AnalysisResult,
    position: (&str, LexerConfig),
    scope: (
        Option<&SignatureNamespaceScope>,
        Option<&SourceOriginalVariableFrame>,
    ),
    braced: bool,
    arrays: bool,
    excluded: Option<Span>,
    candidates: &mut Vec<CompletionItem>,
) {
    let (source, config) = position;
    let (namespace, frame) = scope;
    for occurrence in &analysis.original_variable_symbols {
        // Cursor geometry cannot supply a new completion candidate from the
        // partial lexical reference currently being completed.
        if excluded.is_some_and(|span| {
            occurrence.span().start() <= span.start() && span.end() <= occurrence.span().end()
        }) {
            continue;
        }
        if !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            occurrence.original_name_input(),
            occurrence.span(),
        ) {
            continue;
        }
        let Some(root) = accessible_name(
            occurrence,
            namespace,
            frame,
            &analysis.original_variable_symbols,
        ) else {
            continue;
        };
        let element = if arrays {
            let Some(element) = element_name(occurrence) else {
                continue;
            };
            Some(element)
        } else {
            None
        };
        let policy = occurrence.symbol().policy();
        let Some(spelling) =
            reference_spelling(root.as_bytes(), element.as_deref(), braced, config, policy)
        else {
            continue;
        };
        if candidates
            .iter()
            .any(|item: &CompletionItem| item.insert_text == spelling)
        {
            continue;
        }
        let label = spelling.clone();
        candidates.push(CompletionItem {
            label,
            insert_text: spelling.clone(),
            kind: CompletionKind::Variable,
            detail: Some("variable — source declaration candidate".to_owned()),
            ..CompletionItem::default()
        });
    }
}

fn extend_alias_reference_candidates(
    analysis: &AnalysisResult,
    position: (&str, &SourceImage, LexerConfig),
    frame: Option<&SourceOriginalVariableFrame>,
    braced: bool,
    excluded: Option<Span>,
    candidates: &mut Vec<CompletionItem>,
) {
    let (source, image, config) = position;
    for advice in analysis
        .original_variable_alias_advice_in_source(image, config)
        .into_iter()
        .flatten()
    {
        let input = advice.original_name_input();
        if Some(advice.original_frame()) != frame
            || excluded.is_some_and(|span| {
                advice.span().start() <= span.start() && span.end() <= advice.span().end()
            })
            || !crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                input,
                advice.span(),
            )
        {
            continue;
        }
        let Some(spelling) = reference_spelling(
            advice.local_name().as_bytes(),
            None,
            braced,
            config,
            input.policy(),
        ) else {
            continue;
        };
        if candidates
            .iter()
            .any(|item: &CompletionItem| item.insert_text == spelling)
        {
            continue;
        }
        candidates.push(CompletionItem {
            label: spelling.clone(),
            insert_text: spelling,
            kind: CompletionKind::Variable,
            detail: Some("variable — source alias declaration candidate".to_owned()),
            ..CompletionItem::default()
        });
    }
}

fn extend_write_reference_candidates(
    analysis: &AnalysisResult,
    position: (&str, LexerConfig),
    scope: (
        Option<&SignatureNamespaceScope>,
        Option<&SourceOriginalVariableFrame>,
    ),
    braced: bool,
    excluded: Option<Span>,
    candidates: &mut Vec<CompletionItem>,
) {
    let (source, config) = position;
    let (namespace, frame) = scope;
    for advice in &analysis.original_variable_write_advice {
        let visible = if let Some(owner) = advice.original_frame() {
            Some(owner) == frame
        } else {
            frame.is_none() && namespace.is_some() && advice.original_namespace() == namespace
        };
        let input = advice.original_name_input();
        if !visible
            || excluded.is_some_and(|span| {
                advice.span().start() <= span.start() && span.end() <= advice.span().end()
            })
            || !crate::original_name_edit::original_input_matches_source(
                source,
                analysis,
                input,
                advice.span(),
            )
            || analysis.original_variable_symbols.iter().any(|occurrence| {
                occurrence.span() == advice.span() && occurrence.original_name_input() == input
            })
        {
            continue;
        }
        let protocol = input.policy().recipe();
        let Some(form) = advice.receiver_form().input_form(protocol, input.bytes()) else {
            continue;
        };
        let root = match form {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let Some(spelling) =
            reference_spelling(root.root().selected(), None, braced, config, input.policy())
        else {
            continue;
        };
        if candidates
            .iter()
            .any(|item: &CompletionItem| item.insert_text == spelling)
        {
            continue;
        }
        candidates.push(CompletionItem {
            label: spelling.clone(),
            insert_text: spelling.clone(),
            kind: CompletionKind::Variable,
            detail: Some("variable — conditional write-name candidate".to_owned()),
            ..CompletionItem::default()
        });
    }
}

fn accessible_name(
    occurrence: &SignatureSourceVariableOccurrence,
    namespace: Option<&SignatureNamespaceScope>,
    frame: Option<&SourceOriginalVariableFrame>,
    symbols: &[SignatureSourceVariableOccurrence],
) -> Option<NameBytes> {
    if let Some((owner, local)) = occurrence.original_local_alias() {
        return (Some(owner) == frame).then(|| local.clone());
    }
    let symbol = occurrence.symbol();
    let protocol = symbol.policy().recipe();
    match symbol.slot() {
        SignatureSourceVariableSlot::Local {
            frame: owner,
            simple,
        } => (Some(owner) == frame).then(|| simple.clone()),
        SignatureSourceVariableSlot::C {
            namespace: home,
            simple,
        } => {
            let current = namespace?.context()?;
            // A `namespace eval` activation (or the global frame) has no
            // local table: an unqualified name there is a namespace variable.
            let namespace_level = frame.is_none_or(|frame| {
                matches!(
                    frame.frame(),
                    VariableExecutionFrame::Global
                        | VariableExecutionFrame::Namespace(_)
                        | VariableExecutionFrame::NamespaceActivation { .. }
                )
            });
            let here = namespace == Some(&SignatureNamespaceScope::C(home.clone()));
            // Tcl 8.x (before TIP 278) resolves an unqualified namespace-scope
            // variable the current namespace does not hold in the global
            // namespace; tclsh 8.6: `set flag 1; namespace eval foo {set flag}`
            // answers 1, tclsh 9.0 raises `can't read "flag"`.
            let global_fallback = namespace_level
                && !here
                && home.is_root()
                && matches!(protocol, tcl_syntax::naming::NativeNameProtocol::C(version)
                    if version.namespace_var_global_fallback())
                && matches!(namespace, Some(SignatureNamespaceScope::C(current)) if !current.is_root()
                    && !symbols.iter().any(|other| matches!(other.symbol().slot(),
                        SignatureSourceVariableSlot::C { namespace: owner, simple: name }
                            if owner == current && name == simple)));
            let bare = namespace_level && (here || global_fallback);
            let input = if bare {
                simple.as_bytes().to_vec()
            } else {
                tcl_syntax::naming::qualify_bytes(
                    &tcl_syntax::naming::native_namespace_full_name_bytes(home),
                    simple.as_bytes(),
                )
            };
            let selected = protocol.variable_root_geometry(current, &input);
            let same = match selected {
                NativeVariableRootGeometry::CNamespace {
                    namespace,
                    simple: root,
                } => namespace == *home && root == *simple,
                NativeVariableRootGeometry::Local(root) => bare && root == *simple,
                NativeVariableRootGeometry::JimAbsolute(_) => false,
            };
            same.then(|| input.into())
        }
        SignatureSourceVariableSlot::Jim(key) => {
            let current = namespace?.context()?;
            let input = [b"::".as_slice(), key.as_bytes()].concat();
            matches!(protocol.variable_root_geometry(current, &input),
                NativeVariableRootGeometry::JimAbsolute(ref selected) if selected == key)
            .then(|| input.into())
        }
    }
}

fn element_name(occurrence: &SignatureSourceVariableOccurrence) -> Option<Vec<u8>> {
    match occurrence.receiver() {
        OriginalVariableSymbolReceiver::Operand(
            tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
        ) => occurrence
            .original_name_input()
            .policy()
            .recipe()
            .combined_variable_input(occurrence.original_name_input().bytes())
            .element()
            .map(|element| element.selected().to_vec()),
        OriginalVariableSymbolReceiver::LexicalRoot => {
            let tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(
                root,
            ) = occurrence.original_name_input()
            else {
                return None;
            };
            if root.is_separate_array_root() {
                root.static_index_input()
                    .map(|index| index.bytes().to_vec())
            } else {
                root.policy()
                    .recipe()
                    .combined_variable_input(root.bytes())
                    .element()
                    .map(|element| element.selected().to_vec())
            }
        }
        _ => None,
    }
}

fn reference_spelling(
    root: &[u8],
    element: Option<&[u8]>,
    braced: bool,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Option<String> {
    let name = tcl_syntax::backslash::native_literal_source_text(
        root,
        tcl_lexer::SourceChannel::Document,
        policy.string_protocol(),
    )?;
    let mut proposals = Vec::new();
    if let Some(element) = element {
        if let Some(text) = tcl_syntax::backslash::native_literal_source_text(
            element,
            tcl_lexer::SourceChannel::Document,
            policy.string_protocol(),
        ) {
            proposals.push(format!("${{{name}({text})}}"));
        }
        if !braced {
            let word = tcl_syntax::backslash::native_literal_source_word(
                element,
                tcl_lexer::SourceChannel::Document,
                config,
                policy.string_protocol(),
            )?;
            proposals.insert(
                0,
                format!("${name}({})", word.get(1..word.len().checked_sub(1)?)?),
            );
        }
    } else {
        if !braced {
            proposals.push(format!("${name}"));
        }
        proposals.push(format!("${{{name}}}"));
    }
    proposals
        .into_iter()
        .find(|proposal| reference_matches(proposal, root, element, config, policy))
}

fn reference_matches(
    proposal: &str,
    wanted: &[u8],
    element: Option<&[u8]>,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> bool {
    let image = SourceImage::document(proposal);
    let Some(end) = u32::try_from(image.len()).ok() else {
        return false;
    };
    let span = Span::new(0, end);
    let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
        image.clone(),
        span,
        tcl_lexer::SubstFlags::default(),
        config,
    ) else {
        return false;
    };
    let [part] = arena.list(arena.root()) else {
        return false;
    };
    if part.span != span {
        return false;
    }
    let Some(root) = SignatureSourceVariableRoot::from_original_executable(
        &arena,
        &image,
        config,
        part.span,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
        policy,
    ) else {
        return false;
    };
    if !root.is_separate_array_root() {
        let selected = policy.recipe().combined_variable_input(root.bytes());
        return selected.root().selected() == wanted
            && selected
                .element()
                .map(tcl_syntax::naming::NativeNameProjection::selected)
                == element;
    }
    if root.bytes() != wanted {
        return false;
    }
    let Some(element) = element else {
        return false;
    };
    // This receiver is an executable substitution. Its retained index list
    // supplies the static value; compiler variable-operand shape is a separate
    // purpose and cannot decide whether this source reference roundtrips.
    root.static_index_input()
        .is_some_and(|input| input.bytes() == element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    pub(super) fn items(
        source: &str,
        line: u32,
        character: u32,
        analysis: &AnalysisResult,
        partial: &str,
        trigger: char,
    ) -> Option<Vec<CompletionItem>> {
        if analysis.allows_retained_logical_declaration_advice() {
            return None;
        }
        let index = tcl_lexer::LineIndex::new(source);
        let cursor = crate::definition::byte_offset_at(&index, source, line, character);
        let position = index.position_at_utf16(cursor, source);
        if position.line != line || position.character.get() != character {
            return Some(Vec::new());
        }
        let Ok(Some(prefix)) =
            crate::source_structure::original_variable_cursor_prefix(source, analysis, cursor)
        else {
            return Some(Vec::new());
        };
        if source.get(prefix.partial_span().as_range()) != Some(partial)
            || trigger != if prefix.is_braced() { '{' } else { '$' }
        {
            return Some(Vec::new());
        }
        super::items(source, line, character, analysis, &prefix)
    }

    #[test]
    fn loop_list_candidates_are_conditional_same_frame_source_fields() {
        for body in [
            "dict for {k v} {a 1} {puts $}",
            "foreach {k v} {a 1} {puts $}",
        ] {
            let source = format!("proc p {{}} {{{body}}}");
            let analysis = Analyser::new().analyse(&source, "tcl8.6");
            let offset = u32::try_from(source.find("puts $").unwrap() + 6).unwrap();
            let index = tcl_lexer::LineIndex::new(&source);
            let position = index.position_at_utf16(offset, &source);
            let got = items(
                &source,
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$',
            )
            .unwrap();
            for name in ["$k", "$v"] {
                assert!(
                    got.iter().any(|item| item.insert_text == name
                        && item.detail.as_deref()
                            == Some("variable — conditional loop-list source candidate")),
                    "{body}: {got:?}"
                );
            }
            let choices = source_references(&source, &analysis, offset).unwrap();
            assert!(
                !choices.iter().any(|name| name == "$k" || name == "$v"),
                "{choices:?}"
            );
        }
        for source in [
            r"proc p {} {foreach {a\x62 arr(x)} {} {puts $}}",
            "proc p {} {foreach {k v} {} {}}; proc q {} {puts $}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("puts $").unwrap() + 6).unwrap();
            let index = tcl_lexer::LineIndex::new(source);
            let position = index.position_at_utf16(offset, source);
            let got = items(
                source,
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$',
            )
            .unwrap();
            assert!(
                !got.iter().any(|item| item.detail.as_deref()
                    == Some("variable — conditional loop-list source candidate")),
                "{got:?}"
            );
        }
    }

    #[test]
    fn loop_list_observer_shadow_and_stale_context_stay_conditional() {
        let shadowed = "proc dict args {}; proc p {} {dict for {k v} {} {puts $}}";
        let traced = "proc observe args {}; trace add execution dict enter observe; proc p {} {dict for {k v} {} {puts $}}";
        for (source, shadow) in [(shadowed, true), (traced, false)] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.find("puts $").unwrap() + 6).unwrap();
            let index = tcl_lexer::LineIndex::new(source);
            let position = index.position_at_utf16(offset, source);
            let got = items(
                source,
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$',
            )
            .unwrap();
            let relevant = got
                .iter()
                .filter(|item| item.insert_text == "$k" || item.insert_text == "$v")
                .collect::<Vec<_>>();
            if shadow {
                assert!(relevant.is_empty(), "{relevant:?}");
            } else {
                assert!(
                    relevant.iter().all(|item| item.detail.as_deref()
                        == Some("variable — conditional loop-list source candidate")),
                    "{relevant:?}"
                );
            }
            let changed = format!("{source} ");
            assert_eq!(
                items(
                    &changed,
                    position.line,
                    position.character.get(),
                    &analysis,
                    "",
                    '$'
                )
                .unwrap(),
                [] as [crate::completion::CompletionItem; 0]
            );
        }
        let source = "proc p {} {foreach {k v} {} {puts $}}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let other = Analyser::new().analyse(source, "tcl8.5");
        assert_ne!(analysis.resolved_input, other.resolved_input);
        analysis.resolved_input = other.resolved_input;
        let offset = u32::try_from(source.find("puts $").unwrap() + 6).unwrap();
        let index = tcl_lexer::LineIndex::new(source);
        let position = index.position_at_utf16(offset, source);
        assert_eq!(
            items(
                source,
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$'
            )
            .unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
    }

    #[test]
    fn display_labels_preserve_the_validated_variable_edit() {
        for (source, cursor, partial, trigger, label, insertion, end) in [
            (
                "set greeting hello\nputs ${gre",
                10,
                "gre",
                '{',
                "$greeting",
                "${greeting}",
                10,
            ),
            (
                "set greeting hello\nputs ${gre}",
                10,
                "gre",
                '{',
                "$greeting",
                "${greeting}",
                11,
            ),
            (
                "set \"foo-bar\" 1\nputs $",
                6,
                "",
                '$',
                "$foo-bar",
                "${foo-bar}",
                6,
            ),
            (
                "set {first last} 1\nputs $",
                6,
                "",
                '$',
                "$first last",
                "${first last}",
                6,
            ),
            ("set myé 1\nputs ${my", 9, "my", '{', "$myé", "${myé}", 9),
            (
                "set arr(name) 1\nputs ${arr(na",
                13,
                "arr(na",
                '{',
                "$arr(name)",
                "${arr(name)}",
                13,
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let result = items(source, 1, cursor, &analysis, partial, trigger).unwrap();
            let item = result
                .iter()
                .find(|item| item.label == label)
                .expect(source);
            assert_eq!(item.insert_text, insertion);
            assert_eq!(item.filter_text.as_deref(), Some(insertion));
            let edit = item.text_edit.as_ref().unwrap();
            assert_eq!((edit.start_char, edit.end_char), (5, end));
            assert_eq!(edit.new_text, insertion);
            let line = source.lines().nth(1).unwrap();
            let applied = format!(
                "{}{}{}",
                &line[..edit.start_char as usize],
                edit.new_text,
                &line[edit.end_char as usize..]
            );
            assert_eq!(applied, format!("puts {insertion}"));
            assert_eq!(
                items(
                    &format!("{source} "),
                    1,
                    cursor,
                    &analysis,
                    partial,
                    trigger
                )
                .unwrap_or_default(),
                [] as [crate::completion::CompletionItem; 0]
            );
        }
    }

    #[test]
    fn colliding_display_labels_keep_distinct_edits() {
        let mut values = vec![
            CompletionItem {
                label: "${x}".into(),
                insert_text: "${x}".into(),
                ..CompletionItem::default()
            },
            CompletionItem {
                label: "$x".into(),
                insert_text: "$x".into(),
                ..CompletionItem::default()
            },
        ];
        super::normalise_reference_labels(&mut values);
        assert_eq!(values[0].label, "${x}");
        assert_eq!(values[1].label, "$x");
        let mut fuzzy = vec![CompletionItem {
            label: "${long-name}".into(),
            insert_text: "${long-name}".into(),
            filter_text: Some("$ln".into()),
            ..CompletionItem::default()
        }];
        super::normalise_reference_labels(&mut fuzzy);
        assert_eq!(fuzzy[0].label, "$long-name");
        assert_eq!(fuzzy[0].filter_text.as_deref(), Some("$ln"));
    }

    #[test]
    fn variable_completion_requires_positive_logical_input_for_scope_advice() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        let source = "set ordinary 1\nputs $ord";
        let logical = Analyser::new().analyse(source, "tcl");
        assert!(logical.allows_retained_logical_declaration_advice());
        assert!(items(source, 1, 9, &logical, "ord", '$').is_none());
        assert!(
            super::super::variable_trigger_completions(source, 1, 9, &logical)
                .unwrap()
                .iter()
                .any(|item| item.insert_text.contains("ordinary"))
        );
        let mut missing = logical.clone();
        missing.resolved_input = None;
        assert!(!missing.allows_retained_logical_declaration_advice());
        assert_eq!(
            items(source, 1, 9, &missing, "ord", '$').unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
        assert_eq!(
            super::super::variable_trigger_completions(source, 1, 9, &missing).unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
        assert_eq!(
            source_references(source, &missing, 0).unwrap(),
            [] as [std::string::String; 0]
        );
        for dialect in ["tcl8.6", "f5-irules"] {
            let mut original = Analyser::new().analyse(source, dialect);
            assert!(!original.allows_retained_logical_declaration_advice());
            assert_eq!(
                items(source, 0, 0, &original, "ord", '$').unwrap(),
                [] as [crate::completion::CompletionItem; 0]
            );
            original.body_lexer_config = None;
            assert_eq!(
                super::super::variable_trigger_completions(source, 1, 9, &original).unwrap(),
                [] as [crate::completion::CompletionItem; 0]
            );
            assert_eq!(
                source_references(source, &original, 0).unwrap(),
                [] as [std::string::String; 0]
            );
        }
    }

    #[test]
    fn original_variable_completion_selects_actual_formal_frame_and_alias_spelling() {
        // Implementation contract: naming.compiler.original-variable-alias-source-schema
        // docs/design/analysis/name-resolution-proofs/original-variable-alias-source-schema.md
        // Implementation contract: naming.variable.original-body-cursor-frame
        // docs/design/analysis/name-resolution-proofs/original-body-cursor-frame.md
        let source = "set shared 1\nproc first {argument} {set local 1; global shared; upvar #0 shared link; puts $}\nproc second {other} {set foreign 1}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.global_scope.variables.clear();
        for scope in &mut analysis.global_scope.children {
            scope.variables.clear();
            scope.name.clear();
        }
        let offset = u32::try_from(source.find("puts $").unwrap() + "puts $".len()).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
        let config = analysis.body_lexer_config.unwrap();
        let frame = analysis
            .original_variable_frame_in_source(&SourceImage::document(source), config, offset)
            .expect("the lexical cursor retains its actual procedure frame");
        assert!(analysis.original_variable_symbols.iter().any(|occurrence|
            matches!(occurrence.symbol().slot(), SignatureSourceVariableSlot::Local { frame: owner, simple }
                if owner == &frame && simple.as_bytes() == b"argument")),
            "the formal declaration belongs to the selected cursor frame");
        let completed = items(
            source,
            position.line,
            position.character.get(),
            &analysis,
            "",
            '$',
        )
        .unwrap();
        let names = completed
            .iter()
            .map(|item| item.insert_text.as_str())
            .collect::<Vec<_>>();
        for expected in ["$argument", "$local", "$shared", "$link", "$::shared"] {
            assert!(names.contains(&expected), "{expected}: {names:?}");
        }
        assert!(
            !names.contains(&"$other") && !names.contains(&"$foreign"),
            "{names:?}"
        );
        assert_eq!(
            items(
                &format!("{source} "),
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$'
            )
            .unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
    }

    #[test]
    fn original_vendor_variable_completion_uses_authentic_body_and_formal_source_advice() {
        // Implementation contract: naming.vendor.original-source-declaration-consumers
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
        let source = "proc first {argument} {set local 1; set v\\uD800 2; puts $}\nproc second {other} {set foreign 1}\n";
        let mut analysis = Analyser::new().analyse(source, "f5-iapps");
        assert!(analysis.has_original_vendor_source_names());
        analysis.all_procs.clear();
        analysis.global_scope.variables.clear();
        for scope in &mut analysis.global_scope.children {
            scope.variables.clear();
            scope.name.clear();
        }
        let offset = u32::try_from(source.find("puts $").unwrap() + "puts $".len()).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
        let home = analysis
            .original_vendor_variable_body_in_source(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
                offset,
            )
            .expect("the source cursor belongs to the genuine selected procedure body");
        assert!(
            analysis
                .original_vendor_variable_advice()
                .any(|advice| advice.home() == Some(home)
                    && advice.literal_units() == Some(b"argument".as_slice()))
        );
        let completed = items(
            source,
            position.line,
            position.character.get(),
            &analysis,
            "",
            '$',
        )
        .unwrap();
        let names = completed
            .iter()
            .map(|item| item.insert_text.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"${argument}"), "{names:?}");
        assert!(names.contains(&"${local}"), "{names:?}");
        assert!(
            !names.contains(&"${other}") && !names.contains(&"${foreign}"),
            "{names:?}"
        );
        assert!(names.iter().all(|name| !name.contains("D800")));
        assert_eq!(analysis.original_variable_symbols, [] as [tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence; 0]);
        assert_eq!(
            items(
                &format!("{source} "),
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$'
            )
            .unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
    }

    #[test]
    fn original_variable_completion_uses_static_array_indices_without_reporting_maps() {
        let source = "set arr(first) 1\nset arr(second) 2\nputs $arr(f)\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.global_scope.variables.clear();
        analysis.all_variables.clear();
        let offset = u32::try_from(source.find("$arr(f").unwrap() + "$arr(f".len()).unwrap();
        let position = tcl_lexer::LineIndex::new(source).position_at_utf16(offset, source);
        let completed = items(
            source,
            position.line,
            position.character.get(),
            &analysis,
            "arr(f",
            '$',
        )
        .unwrap();
        assert_eq!(completed.len(), 1, "{completed:?}");
        assert_eq!(completed[0].insert_text, "$arr(first)");
        assert_eq!(completed[0].text_edit.as_ref().unwrap().end_char, 12);
    }

    #[test]
    fn original_array_reference_spelling_uses_executable_index_lineage() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::for_tcl_version(version),
            );
            let policy = dialect.authored_name_policy().unwrap();
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            assert_eq!(
                reference_spelling(b"arr", Some(b"first"), false, config, policy).as_deref(),
                Some("$arr(first)")
            );
            assert_eq!(
                reference_spelling(b"arr", Some(b"first"), true, config, policy).as_deref(),
                Some("${arr(first)}")
            );
            assert!(reference_matches(
                r"$arr(\$index)",
                b"arr",
                Some(b"$index"),
                config,
                policy
            ));
            assert!(!reference_matches(
                "$arr($index)",
                b"arr",
                Some(b"$index"),
                config,
                policy
            ));
            assert!(!reference_matches(
                "$arr([unknown])",
                b"arr",
                Some(b"[unknown]"),
                config,
                policy
            ));
        }
    }

    #[test]
    fn literal_variable_completion_refuses_numeric_escape_donation_and_roundtrips_unicode() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
            );
            let policy = dialect.authored_name_policy().unwrap();
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            let units = tcl_syntax::backslash::native_source_literal_bytes(
                "café🙂".as_bytes(),
                tcl_lexer::SourceChannel::Document,
                policy.string_protocol(),
            )
            .unwrap();
            let spelling = reference_spelling(&units, None, true, config, policy).unwrap();
            assert!(reference_matches(&spelling, &units, None, config, policy));
            if profile != "jim" {
                assert!(reference_spelling(b"v\xed\xa0\x80", None, true, config, policy).is_none());
            }
        }
    }
}

#[cfg(test)]
mod write_advice_tests {
    use super::tests::items;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_conditional_write_candidates_require_a_genuine_home_and_current_source() {
        let source = "set own 1\nputs $o\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            analysis
                .original_variable_write_advice
                .iter()
                .any(|advice| advice.original_name_input().bytes() == b"own"
                    && advice.original_namespace().is_some())
        );
        analysis.original_variable_symbols.clear();
        analysis.global_scope.variables.clear();
        let candidates = items(source, 1, 7, &analysis, "o", '$').unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(|item| item.insert_text.as_str())
                .collect::<Vec<_>>(),
            ["$own"]
        );
        assert_eq!(
            candidates[0].detail.as_deref(),
            Some("variable — conditional write-name candidate")
        );
        assert_eq!(
            items(&format!("# displaced\n{source}"), 2, 7, &analysis, "o", '$').unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
        analysis.original_variable_write_advice.clear();
        assert_eq!(
            items(source, 1, 7, &analysis, "o", '$').unwrap(),
            [] as [crate::completion::CompletionItem; 0]
        );
    }
}
