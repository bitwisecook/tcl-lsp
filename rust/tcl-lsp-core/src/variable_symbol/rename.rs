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
    let index = LineIndex::new(source);
    Ok(edits
        .into_iter()
        .map(|edit| TextEdit {
            range: crate::definition::span_to_range(source, &index, edit.span()),
            new_text: edit.text().to_owned(),
        })
        .collect())
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
