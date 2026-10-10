// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current source declaration queries for tools without an execution position.
//!
//! These queries retain every source declaration. They do not select a live
//! procedure, resolve aliases, or choose among redefinitions by document order.

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_lexer::SourceImage;

/// Original procedure metadata from this complete current source and grammar.
/// Equal displayed names and repeated publications remain separate records.
/// `None` means the original inventory is unavailable, not an empty document.
#[must_use]
pub fn declarations<'a>(source: &str, analysis: &'a AnalysisResult) -> Option<Vec<&'a ProcDef>> {
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let mut declarations = if analysis.allows_lexical_declaration_advice() {
        analysis.all_procs.values().collect::<Vec<_>>()
    } else {
        let mut declarations = Vec::new();
        for row in analysis.original_procedure_declarations() {
            if row.name_input().source_image() != &image
                || row.name_input().lexer_config() != config
                || row.name_input().policy() != row.name().policy()
            {
                return None;
            }
            declarations.push(row.metadata());
        }
        declarations
    };
    for row in analysis.original_vendor_procedure_declarations() {
        if !row.name_input().matches_source(&image, config) {
            return None;
        }
        declarations.push(row.metadata());
    }
    declarations.sort_by_key(|proc| (proc.name_span.start(), proc.name_span.end()));
    Some(declarations)
}

/// Source declarations named by an API's literal command-name value at global
/// scope. The request is Unicode document-channel text, not Tcl script syntax;
/// backslash escapes are not evaluated. Each declaration's selected name recipe
/// projects the value. Missing scope never enables a same-tail namespace guess.
///
/// Hosted source inputs have a separate header-only purpose: supported literal
/// declaration spelling is matched exactly, without global qualification. An
/// owned unavailable value withdraws this query's completeness.
///
/// All matching declarations are returned, so the consumer must handle
/// ambiguity explicitly. Matching supplies no publication or dispatch proof.
#[must_use]
pub fn for_literal_name<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    name: &str,
) -> Option<Vec<&'a ProcDef>> {
    let current = declarations(source, analysis)?;
    if analysis.allows_lexical_declaration_advice() {
        let wanted = crate::normalise_qualified_command_name(name);
        return Some(
            current
                .into_iter()
                .filter(|proc| {
                    crate::normalise_qualified_command_name(&proc.qualified_name) == wanted
                })
                .collect(),
        );
    }
    if analysis.has_original_vendor_source_names() {
        // Hosted source headers have no global command publication protocol.
        // Match only their literal declaration spelling, without qualification
        // normalisation, tail matching or runtime name-resolution claims.
        let mut matching = Vec::new();
        for row in analysis.original_vendor_procedure_declarations() {
            let units = row.name_input().literal_units(row.purpose())?;
            if units == name.as_bytes() {
                matching.push(row.metadata());
            }
        }
        return Some(matching);
    }
    let mut matching = Vec::new();
    for row in analysis.original_procedure_declarations() {
        let selected =
            crate::original_declaration::literal_name_matches_publication(name, row.name())?;
        if selected {
            matching.push(row.metadata());
        }
    }
    Some(matching)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_procedure_tool_queries_keep_declarations_and_refuse_scope_guesses() {
        let source = "namespace eval a {proc duplicate {first} {}}\nnamespace eval z {proc duplicate {second} {}}\nproc p {} {}\nproc p {value} {}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_procs.clear();
            assert_eq!(
                declarations(source, &analysis).unwrap().len(),
                4,
                "{dialect}"
            );
            assert!(
                for_literal_name(source, &analysis, "duplicate")
                    .unwrap()
                    .is_empty()
            );
            let chosen = for_literal_name(source, &analysis, "z::duplicate").unwrap();
            assert_eq!(chosen.len(), 1, "{dialect}");
            assert_eq!(chosen[0].params[0].name, "second");
            assert_eq!(for_literal_name(source, &analysis, "p").unwrap().len(), 2);
            assert!(declarations(&format!("# changed\n{source}"), &analysis).is_none());
        }
    }

    #[test]
    fn original_procedure_tool_literal_values_use_the_selected_native_ingress() {
        let source = "proc {a b} {} {}\nproc {é😀} {} {}\nproc {literal\\u0041} {} {}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let analysis = Analyser::new().analyse(source, dialect);
            for name in ["a b", "é😀", r"literal\u0041"] {
                assert_eq!(
                    for_literal_name(source, &analysis, name).unwrap().len(),
                    1,
                    "{dialect}: {name}"
                );
            }
            assert!(
                for_literal_name(source, &analysis, "literalA")
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
