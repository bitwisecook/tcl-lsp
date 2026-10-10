// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
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
use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::naming::{NamePolicyProtocol, NativeVariableRootGeometry};

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
    let Some(config) = analysis.body_lexer_config else {
        return Some(Vec::new());
    };
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return Some(Vec::new());
    }
    let index = tcl_lexer::LineIndex::new(source);
    let cursor = crate::definition::byte_offset_at(&index, source, line, character);
    let prefix = if trigger == '{' { 2 } else { 1 };
    let Some(start) = u32::try_from(partial.len() + prefix)
        .ok()
        .and_then(|length| cursor.checked_sub(length))
    else {
        return Some(Vec::new());
    };
    let expected = if trigger == '{' { "${" } else { "$" };
    if source.get(start as usize..start as usize + prefix) != Some(expected) {
        return Some(Vec::new());
    }
    let end = reference_end(&image, config, start, cursor).unwrap_or(cursor);
    let begin = index.position_at_utf16(start, source);
    let finish = index.position_at_utf16(end, source);
    if begin.line != line || finish.line != line {
        return Some(Vec::new());
    }
    let mut candidates = reference_candidates(
        source,
        analysis,
        cursor,
        trigger == '{',
        partial.contains('('),
        Some(Span::new(start, cursor)),
    );
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
    Some(candidates)
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
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
        let mut locals = 0;
        let mut matching_locals = 0;
        let mut aliases = 0;
        let mut matching_aliases = 0;
        for occurrence in &analysis.original_variable_symbols {
            if let SignatureSourceVariableSlot::Local { frame: owner, .. } =
                occurrence.symbol().slot()
            {
                locals += 1;
                matching_locals += usize::from(Some(owner) == frame.as_ref());
            }
            if let Some((owner, _)) = occurrence.original_local_alias() {
                aliases += 1;
                matching_aliases += usize::from(Some(owner) == frame.as_ref());
            }
        }
        eprintln!(
            "ORIGINAL_VARIABLE_COMPLETION cursor={cursor} frame={} namespace={} symbols={} locals={locals} matching_locals={matching_locals} aliases={aliases} matching_aliases={matching_aliases} write_advice={}",
            frame.is_some(),
            namespace.is_some(),
            analysis.original_variable_symbols.len(),
            analysis.original_variable_write_advice.len()
        );
    }
    if analysis.has_original_vendor_source_names() {
        let home = analysis.original_vendor_variable_body_in_source(&image, config, cursor);
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
                    .matches_source(&image, config)
            {
                continue;
            }
            let Some(spelling) =
                tcl_compiler::signature_scan::vendor_variable::vendor_variable_source_reference(
                    advice,
                )
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
        return candidates;
    }
    let mut candidates = Vec::new();
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
        let Some(root) = accessible_name(occurrence, namespace, frame.as_ref()) else {
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
    if !arrays {
        for advice in analysis
            .original_variable_alias_advice_in_source(&image, config)
            .into_iter()
            .flatten()
        {
            let input = advice.original_name_input();
            if Some(advice.original_frame()) != frame.as_ref()
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
        for advice in &analysis.original_variable_write_advice {
            let visible = if let Some(owner) = advice.original_frame() {
                Some(owner) == frame.as_ref()
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
    candidates
}

fn accessible_name(
    occurrence: &SignatureSourceVariableOccurrence,
    namespace: Option<&SignatureNamespaceScope>,
    frame: Option<&SourceOriginalVariableFrame>,
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
            let bare =
                frame.is_none() && namespace == Some(&SignatureNamespaceScope::C(home.clone()));
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

fn reference_end(image: &SourceImage, config: LexerConfig, start: u32, cursor: u32) -> Option<u32> {
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        image.clone(),
        Span::new(0, u32::try_from(image.len()).ok()?),
        tcl_lexer::SubstFlags::default(),
        config,
    )
    .ok()?;
    let mut parts = arena.all_parts().filter(|part| {
        part.span.start() == start
            && part.span.end() >= cursor
            && matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
    });
    let end = parts.next()?.span.end();
    parts.next().is_none().then_some(end)
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
            && selected.element().map(|element| element.selected()) == element;
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
        assert!(
            items(source, 1, 9, &missing, "ord", '$')
                .unwrap()
                .is_empty()
        );
        assert!(
            super::super::variable_trigger_completions(source, 1, 9, &missing)
                .unwrap()
                .is_empty()
        );
        assert!(source_references(source, &missing, 0).unwrap().is_empty());
        for dialect in ["tcl8.6", "f5-irules"] {
            let mut original = Analyser::new().analyse(source, dialect);
            assert!(!original.allows_retained_logical_declaration_advice());
            assert!(
                items(source, 0, 0, &original, "ord", '$')
                    .unwrap()
                    .is_empty()
            );
            original.body_lexer_config = None;
            assert!(
                super::super::variable_trigger_completions(source, 1, 9, &original)
                    .unwrap()
                    .is_empty()
            );
            assert!(source_references(source, &original, 0).unwrap().is_empty());
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
        assert!(
            items(
                &format!("{source} "),
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$'
            )
            .unwrap()
            .is_empty()
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
        assert!(analysis.original_variable_symbols.is_empty());
        assert!(
            items(
                &format!("{source} "),
                position.line,
                position.character.get(),
                &analysis,
                "",
                '$'
            )
            .unwrap()
            .is_empty()
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
    use super::*;
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
        assert!(
            items(&format!("# displaced\n{source}"), 2, 7, &analysis, "o", '$')
                .unwrap()
                .is_empty()
        );
        analysis.original_variable_write_advice.clear();
        assert!(items(source, 1, 7, &analysis, "o", '$').unwrap().is_empty());
    }
}
