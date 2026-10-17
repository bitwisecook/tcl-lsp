// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Atomic source edits for independently selected original variable symbols.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
use tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSymbol;
use tcl_core_types::NameBytes;
use tcl_lexer::{LineIndex, SourceImage};

use crate::rename::TextEdit;
use crate::rename_safety::RenameRefusal;

/// Plan every source occurrence of a selected variable. Actual naming inputs,
/// receiver purposes, complete source currency and alias coverage precede any
/// edit. Independent upvar local spellings remain unchanged. This proves source
/// correspondence, without granting current cells, values or frame execution.
///
/// # Errors
/// Refuses incomplete coverage, collisions, readonly computed inputs or edits
/// that change lexical delimiters, selected indices or unrelated list children.
pub fn original_variable_rename_edits(
    source: &str,
    analysis: &AnalysisResult,
    symbol: &SignatureSourceVariableSymbol,
    new_tail: &str,
) -> Result<Vec<TextEdit>, RenameRefusal> {
    let refuse = |reason: &str| RenameRefusal {
        reason: reason.to_owned(),
        range: None,
    };
    let image = SourceImage::document(source);
    let config = analysis
        .body_lexer_config
        .ok_or_else(|| refuse("the variable document has no retained complete grammar"))?;
    if !analysis.matches_original_source_image(&image, config) {
        return Err(refuse(
            "the variable document no longer matches its complete original source owner",
        ));
    }
    let tail = tcl_syntax::backslash::native_source_literal_bytes(
        new_tail.as_bytes(),
        image.channel(),
        symbol.policy().string_protocol(),
    )
    .map_err(|_| refuse("the replacement variable has no selected source channel recipe"))?;
    let proposed = symbol
        .renamed(&tail)
        .ok_or_else(|| refuse("the replacement must remain one unqualified variable root"))?;
    if !analysis.original_variable_rename_is_complete(symbol) {
        return Err(refuse(
            "cannot account for every original variable name, alias target and read in this document",
        ));
    }
    if proposed != *symbol
        && analysis
            .original_variable_symbols
            .iter()
            .any(|row| row.symbol() == &proposed)
    {
        return Err(refuse(
            "the replacement names another variable in this document",
        ));
    }
    let mut requests = Vec::<(SignatureSourceNameInput, NameBytes)>::new();
    let mut roots = Vec::new();
    for occurrence in super::occurrences(analysis, symbol) {
        let input = occurrence.original_name_input();
        if !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            input,
            occurrence.span(),
        ) {
            return Err(refuse(
                "a variable occurrence has no current original source correspondence",
            ));
        }
        let wanted = occurrence.renamed_input(&tail).ok_or_else(|| {
            refuse("the variable receiver cannot preserve its selected root and index")
        })?;
        if wanted == input.bytes() {
            continue;
        }
        if let SignatureSourceNameInput::OriginalVariableRoot(root) = input {
            let name = root
                .name_span()
                .ok_or_else(|| refuse("the lexical variable has no original name extent"))?;
            let raw = image
                .bytes()
                .get(name.as_range())
                .ok_or_else(|| refuse("the lexical variable is outside its original source"))?;
            let form = if root.is_separate_array_root() {
                tcl_syntax::naming::NativeVariableInputForm::Separate {
                    root: root.bytes(),
                    element: None,
                }
            } else {
                tcl_syntax::naming::NativeVariableInputForm::Combined(root.bytes())
            };
            let native_extent = symbol
                .policy()
                .recipe()
                .variable_root_tail_extent(form)
                .ok_or_else(|| refuse("the lexical variable has no selected original root tail"))?;
            let extent = tcl_syntax::backslash::native_source_literal_extent(
                raw,
                image.channel(),
                symbol.policy().string_protocol(),
                native_extent,
            )
            .ok_or_else(|| refuse("the variable root tail crosses a source unit boundary"))?;
            let mut after = std::str::from_utf8(raw)
                .map_err(|_| refuse("the editor variable source is not Unicode"))?
                .to_owned();
            after.replace_range(extent, new_tail);
            let produced = tcl_syntax::backslash::native_source_literal_bytes(
                after.as_bytes(),
                image.channel(),
                symbol.policy().string_protocol(),
            )
            .map_err(|_| refuse("the lexical replacement has no source string recipe"))?;
            if produced.as_ref() != wanted {
                return Err(refuse(
                    "the replacement changes unrelated lexical variable units",
                ));
            }
            roots.push(
                crate::original_name_edit::original_variable_root_name_edit(&image, root, &after)
                    .ok_or_else(|| {
                    refuse("the replacement changes variable delimiters or its root/index boundary")
                })?,
            );
        } else {
            if input.original_static_list_container().is_none() {
                return Err(refuse(
                    "the variable is supplied by a readonly computed value without an editable original container",
                ));
            }
            requests.push((input.clone(), wanted.into()));
        }
    }
    let mut edits = crate::original_name_edit::original_name_input_edits(&image, &requests)
        .ok_or_else(|| {
            refuse("the replacement does not preserve its original source containers")
        })?;
    edits.extend(roots);
    edits.sort_by_key(|edit| (edit.span().start(), edit.span().end()));
    edits.dedup();
    if edits
        .windows(2)
        .any(|pair| pair[0].span().end() > pair[1].span().start())
    {
        return Err(refuse(
            "variable edits overlap independent original source containers",
        ));
    }
    let edits = if analysis
        .resolved_input
        .as_ref()
        .is_some_and(|input| input.has_logical_source_name_context())
    {
        LogicalVariableSourceRenameIntent::capture(source, analysis, symbol, &tail, edits)
            .and_then(|intent| intent.into_edits(source, analysis))
            .ok_or_else(|| refuse("the generated rename does not preserve its complete original source occurrence, alias, scope and formal topology"))?
    } else {
        edits
    };
    let index = LineIndex::new(source);
    Ok(edits
        .into_iter()
        .map(|edit| TextEdit {
            range: crate::definition::span_to_range(source, &index, edit.span()),
            new_text: edit.text().to_owned(),
        })
        .collect())
}

/// Intentional source alpha-renaming, not a physical-cell or observer proof.
/// Exact generated owners and the original capture remain held until the
/// existing atomic edit consumer rechecks its full current source/input.
struct LogicalVariableSourceRenameIntent {
    image: SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    original: Vec<tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence>,
    after: String,
    generated: AnalysisResult,
    edits: Vec<crate::original_name_edit::OriginalNameSourceEdit>,
}

impl LogicalVariableSourceRenameIntent {
    fn capture(
        source: &str,
        analysis: &AnalysisResult,
        symbol: &SignatureSourceVariableSymbol,
        tail: &[u8],
        edits: Vec<crate::original_name_edit::OriginalNameSourceEdit>,
    ) -> Option<Self> {
        crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let input = analysis.resolved_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        let mut after = source.to_owned();
        for edit in edits.iter().rev() {
            let span = edit.span().as_range();
            source.get(span.clone())?;
            after.replace_range(span, edit.text());
        }
        let generated = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&after, input.analyser_profile().name);
        crate::original_context::CurrentSourceContext::capture(&after, &generated)?;
        let mut before = analysis
            .original_variable_symbols
            .iter()
            .collect::<Vec<_>>();
        let mut next = generated
            .original_variable_symbols
            .iter()
            .collect::<Vec<_>>();
        let order = |row: &&tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableOccurrence| (row.span().start(), row.span().end(), row.is_declaration());
        before.sort_by_key(order);
        next.sort_by_key(order);
        (before.len() == next.len()).then_some(())?;
        let mut groups = std::collections::HashMap::new();
        let mut inverse = std::collections::HashMap::new();
        for (original, proposed) in before.into_iter().zip(next) {
            let expected = if original.symbol() == symbol {
                original.renamed_input(tail)?
            } else {
                original.original_name_input().bytes().to_vec()
            };
            if proposed.original_name_input().bytes() != expected
                || original.receiver() != proposed.receiver()
                || original.is_declaration() != proposed.is_declaration()
                || original.rename_changes_spelling() != proposed.rename_changes_spelling()
                || std::mem::discriminant(original.original_name_input())
                    != std::mem::discriminant(proposed.original_name_input())
                || original.symbol().policy() != proposed.symbol().policy()
                || !mapped_occurrence(original.span(), proposed.span(), &edits)
                || !same_source_scope(original.symbol(), proposed.symbol(), &edits)
            {
                return None;
            }
            if groups
                .insert(original.symbol().clone(), proposed.symbol().clone())
                .is_some_and(|previous| &previous != proposed.symbol())
                || inverse
                    .insert(proposed.symbol().clone(), original.symbol().clone())
                    .is_some_and(|previous| &previous != original.symbol())
            {
                return None;
            }
            match (
                original.original_local_alias(),
                proposed.original_local_alias(),
            ) {
                (Some((old_frame, old_name)), Some((new_frame, new_name)))
                    if old_name == new_name
                        || original.rename_changes_spelling() && new_name.as_bytes() == tail =>
                {
                    if !same_source_frame(old_frame, new_frame, &edits) {
                        return None;
                    }
                }
                (None, None) => {}
                _ => return None,
            }
        }
        let selected_after = groups.get(symbol)?;
        generated
            .original_variable_rename_is_complete(selected_after)
            .then_some(())?;
        // A formal renamed to or from the reserved rest name must not silently
        // change positional/default/rest topology while its token still parses.
        let image = SourceImage::document(source);
        let after_image = SourceImage::document(&after);
        let before_headers = analysis
            .logical_source_procedure_declarations()
            .collect::<Vec<_>>();
        let after_headers = generated
            .logical_source_procedure_declarations()
            .collect::<Vec<_>>();
        (before_headers.len() == after_headers.len()).then_some(())?;
        for header in before_headers {
            let old_body = header.original_body_word(&image, input)?.span();
            let start = mapped_boundary(old_body.start(), &edits)?;
            let end = mapped_boundary(old_body.end(), &edits)?;
            let mut matching = after_headers.iter().filter(|candidate| {
                candidate
                    .original_body_word(&after_image, input)
                    .is_some_and(|body| body.span().start() == start && body.span().end() == end)
            });
            let matched = matching.next()?;
            (matching.next().is_none()
                && header.source_candidate_coordinate() == matched.source_candidate_coordinate()
                && header.formal_count_projection() == matched.formal_count_projection())
            .then_some(())?;
        }
        Some(Self {
            image,
            input: input.clone(),
            original: analysis.original_variable_symbols.clone(),
            after,
            generated,
            edits,
        })
    }

    fn into_edits(
        self,
        source: &str,
        analysis: &AnalysisResult,
    ) -> Option<Vec<crate::original_name_edit::OriginalNameSourceEdit>> {
        (self.image == SourceImage::document(source)
            && analysis.resolved_input.as_ref() == Some(&self.input)
            && analysis.original_variable_symbols == self.original
            && crate::original_context::CurrentSourceContext::capture(source, analysis).is_some()
            && self.generated.resolved_input.as_ref() == Some(&self.input)
            && crate::original_context::CurrentSourceContext::capture(&self.after, &self.generated)
                .is_some())
        .then_some(self.edits)
    }
}

fn mapped_boundary(
    offset: u32,
    edits: &[crate::original_name_edit::OriginalNameSourceEdit],
) -> Option<u32> {
    let mut delta = 0_i64;
    for edit in edits {
        let span = edit.span();
        if offset < span.start() {
            break;
        }
        if offset == span.start() {
            return u32::try_from(i64::from(offset) + delta).ok();
        }
        if offset < span.end() {
            return None;
        }
        delta += i64::try_from(edit.text().len()).ok()? - i64::from(span.end() - span.start());
    }
    u32::try_from(i64::from(offset) + delta).ok()
}

fn mapped_occurrence(
    old: tcl_lexer::Span,
    new: tcl_lexer::Span,
    edits: &[crate::original_name_edit::OriginalNameSourceEdit],
) -> bool {
    match (
        mapped_boundary(old.start(), edits),
        mapped_boundary(old.end(), edits),
    ) {
        (Some(start), Some(end)) => new.start() == start && new.end() == end,
        _ => edits.iter().any(|edit| {
            let span = edit.span();
            span.start() <= old.start()
                && old.end() <= span.end()
                && mapped_boundary(span.start(), edits)
                    .zip(mapped_boundary(span.end(), edits))
                    .is_some_and(|(start, end)| start <= new.start() && new.end() <= end)
        }),
    }
}

fn same_source_scope(
    old: &SignatureSourceVariableSymbol,
    new: &SignatureSourceVariableSymbol,
    edits: &[crate::original_name_edit::OriginalNameSourceEdit],
) -> bool {
    use tcl_compiler::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    match (old.slot(), new.slot()) {
        (
            SignatureSourceVariableSlot::C { namespace: old, .. },
            SignatureSourceVariableSlot::C { namespace: new, .. },
        ) => old == new,
        (SignatureSourceVariableSlot::Jim(_), SignatureSourceVariableSlot::Jim(_)) => true,
        (
            SignatureSourceVariableSlot::Local { frame: old, .. },
            SignatureSourceVariableSlot::Local { frame: new, .. },
        ) => same_source_frame(old, new, edits),
        _ => false,
    }
}

fn same_source_frame(
    old: &tcl_compiler::command_binding::SourceOriginalVariableFrame,
    new: &tcl_compiler::command_binding::SourceOriginalVariableFrame,
    edits: &[crate::original_name_edit::OriginalNameSourceEdit],
) -> bool {
    let Some(old_end) = u32::try_from(old.source().text.bytes().len())
        .ok()
        .and_then(|len| old.source().base().checked_add(len))
    else {
        return false;
    };
    let Some(new_end) = u32::try_from(new.source().text.bytes().len())
        .ok()
        .and_then(|len| new.source().base().checked_add(len))
    else {
        return false;
    };
    old.lexer_config() == new.lexer_config()
        && std::mem::discriminant(old.frame()) == std::mem::discriminant(new.frame())
        && old.frame().namespace_identity() == new.frame().namespace_identity()
        && old
            .source()
            .origin
            .source_image()
            .bytes()
            .get(old.source().base() as usize..old_end as usize)
            == Some(old.source().text.bytes())
        && new
            .source()
            .origin
            .source_image()
            .bytes()
            .get(new.source().base() as usize..new_end as usize)
            == Some(new.source().text.bytes())
        && mapped_boundary(old.source().base(), edits) == Some(new.source().base())
        && mapped_boundary(old_end, edits) == Some(new_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn apply(
        source: &str,
        analysis: &AnalysisResult,
        symbol: &SignatureSourceVariableSymbol,
    ) -> Result<String, RenameRefusal> {
        let index = LineIndex::new(source);
        let mut edits = original_variable_rename_edits(source, analysis, symbol, "changed")?
            .into_iter()
            .map(|edit| {
                let start = crate::definition::byte_offset_at(
                    &index,
                    source,
                    edit.range.start_line,
                    edit.range.start_character,
                ) as usize;
                let end = crate::definition::byte_offset_at(
                    &index,
                    source,
                    edit.range.end_line,
                    edit.range.end_character,
                ) as usize;
                (start, end, edit.new_text)
            })
            .collect::<Vec<_>>();
        edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        let mut after = source.to_owned();
        for (start, end, text) in edits {
            after.replace_range(start..end, &text);
        }
        Ok(after)
    }

    fn selected(
        source: &str,
        analysis: &AnalysisResult,
        at: &str,
    ) -> SignatureSourceVariableSymbol {
        let offset = u32::try_from(source.find(at).unwrap()).unwrap();
        analysis
            .original_variable_symbol_in_source(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
                offset,
            )
            .unwrap()
            .symbol()
            .clone()
    }

    #[test]
    fn original_variable_rename_keeps_opaque_siblings_and_array_indices() {
        // Implementation contract: naming.editor.original-variable-rename-opaque-root
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-opaque-root.md
        let source = r"set ::N::v\uD800(k) 1; set ::N::v\uD801 2; info exists ::N::v\uD800(k)";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let symbol = selected(source, &analysis, r"::N::v\uD800");
        analysis.global_scope.variables.clear();
        for row in &mut analysis.qualified_var_refs {
            row.qualified_name.clear();
        }
        assert_eq!(
            apply(source, &analysis, &symbol).unwrap(),
            r"set ::N::changed(k) 1; set ::N::v\uD801 2; info exists ::N::changed(k)"
        );
    }

    #[test]
    fn original_variable_rename_preserves_lexical_braces_and_separate_array_index() {
        // Implementation contract: naming.editor.original-variable-rename-lexical-geometry
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-lexical-geometry.md
        let source = "set ::N::a(k) 1; list $::N::a(k) ${::N::a(k)}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let symbol = selected(source, &analysis, "::N::a(k)");
        assert_eq!(
            apply(source, &analysis, &symbol).unwrap(),
            "set ::N::changed(k) 1; list $::N::changed(k) ${::N::changed(k)}"
        );
    }

    #[test]
    fn original_variable_rename_uses_authentic_local_frame_and_refuses_stale_or_colliding_sources()
    {
        // Implementation contract: naming.editor.original-variable-rename-frame-currency
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-frame-currency.md
        let source = "proc p {} {set local 1; list $local}; proc q {} {set local 2; list $local}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let symbol = selected(source, &analysis, "local 1");
        assert_eq!(
            apply(source, &analysis, &symbol).unwrap(),
            "proc p {} {set changed 1; list $changed}; proc q {} {set local 2; list $local}"
        );
        assert!(apply(&source.replace("local 1", "other 1"), &analysis, &symbol).is_err());
        let colliding = "set local 1; set changed 2; list $local";
        let analysis = Analyser::new().analyse(colliding, "tcl8.6");
        let symbol = selected(colliding, &analysis, "local 1");
        assert!(apply(colliding, &analysis, &symbol).is_err());
    }

    #[test]
    fn original_variable_rename_accounts_for_derived_and_independent_alias_reads() {
        // Implementation contract: naming.editor.original-variable-rename-alias-coverage
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-rename-alias-coverage.md
        let source = "namespace eval ::N {variable v 1}; proc p {} {global ::N::v; list $v}; proc q {} {upvar #0 ::N::v alias; list $alias}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let symbol = selected(source, &analysis, "v 1");
        assert_eq!(
            apply(source, &analysis, &symbol).unwrap(),
            "namespace eval ::N {variable changed 1}; proc p {} {global ::N::changed; list $changed}; proc q {} {upvar #0 ::N::changed alias; list $alias}"
        );
    }
}

#[cfg(test)]
mod afterimage_source_intent_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn rename(source: &str, old: &[u8], new: &str) -> Result<String, RenameRefusal> {
        let analysis = Analyser::new().analyse(source, "tcl");
        let symbol = analysis
            .original_variable_symbols
            .iter()
            .find(|row| row.symbol().rename_tail() == Some(old))
            .unwrap()
            .symbol();
        let edits = original_variable_rename_edits(source, &analysis, symbol, new)?;
        let index = LineIndex::new(source);
        let mut ranges = edits
            .into_iter()
            .map(|edit| {
                (
                    index.offset_at_utf16(
                        edit.range.start_line,
                        tcl_lexer::Utf16Col::new(edit.range.start_character),
                        source,
                    ) as usize,
                    index.offset_at_utf16(
                        edit.range.end_line,
                        tcl_lexer::Utf16Col::new(edit.range.end_character),
                        source,
                    ) as usize,
                    edit.new_text,
                )
            })
            .collect::<Vec<_>>();
        ranges.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        let mut after = source.to_owned();
        for (start, end, text) in ranges {
            after.replace_range(start..end, &text);
        }
        Ok(after)
    }

    #[test]
    fn intentional_variable_rename_preserves_actual_local_scope_and_default_topology() {
        let source = "# 😀\r\nproc p {} {set value 3; puts $value}\r\nproc other {} {set value 4; puts $value}";
        let after = rename(source, b"value", "café").unwrap();
        assert_eq!(
            after,
            "# 😀\r\nproc p {} {set café 3; puts $café}\r\nproc other {} {set value 4; puts $value}"
        );
        assert!(rename("proc p {value} {puts $value}", b"value", "args").is_err());
        assert!(rename("proc p {args} {puts $args}", b"args", "value").is_err());
    }

    #[test]
    fn source_rename_keeps_known_alias_and_trace_subject_correspondence_without_observer_absence() {
        let source = "set ::actual 1; proc p {} {upvar #0 ::actual local; puts $local}; trace add variable ::actual read observer";
        let after = rename(source, b"actual", "changed").unwrap();
        assert_eq!(
            after,
            "set ::changed 1; proc p {} {upvar #0 ::changed local; puts $local}; trace add variable ::changed read observer"
        );
        let source = "set ::actual 1; proc p {} {global actual; puts $actual}";
        let after = rename(source, b"actual", "changed").unwrap();
        assert_eq!(
            after,
            "set ::changed 1; proc p {} {global changed; puts $changed}"
        );
    }

    #[test]
    fn mapped_source_boundaries_do_not_invent_internal_container_offsets() {
        let source = "set actual 1; puts $actual";
        let analysis = Analyser::new().analyse(source, "tcl");
        let occurrence = analysis
            .original_variable_symbols
            .iter()
            .find(|row| {
                row.original_name_input()
                    .original_static_list_container()
                    .is_some()
            })
            .unwrap();
        let edits = crate::original_name_edit::original_name_input_edits(
            &SourceImage::document(source),
            &[(
                occurrence.original_name_input().clone(),
                NameBytes::from("changed"),
            )],
        )
        .unwrap();
        assert!(
            LogicalVariableSourceRenameIntent::capture(
                source,
                &analysis,
                occurrence.symbol(),
                b"OTHER",
                edits
            )
            .is_none()
        );
    }
}
