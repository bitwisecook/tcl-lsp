// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly original variable symbols shared by editor providers.
//!
//! Selection requires the complete current source image and grammar. Symbol
//! equality compares the independently selected variable-table geometry and
//! naming policy. These helpers issue no cell lifetime, current value, alias
//! execution, normal-completion or native-compilation capability.

use std::ops::ControlFlow;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::signature_scan::variable_symbol::{
    OriginalVariableAliasTemplate, SignatureSourceVariableOccurrence, SignatureSourceVariableSymbol,
};
use tcl_lexer::Span;

mod rename;
pub use rename::original_variable_rename_edits;

/// Select an authentic current-source variable occurrence. A retained but
/// unavailable original operand is definitive and blocks display-name lookup.
/// `Continue` permits the separate authored metadata consumer.
#[must_use]
pub fn select<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    line: u32,
    character: u32,
) -> ControlFlow<Option<&'a SignatureSourceVariableOccurrence>> {
    if (!analysis.original_variable_symbols.is_empty()
        || !analysis.original_variable_symbol_conflicts.is_empty())
        && analysis.body_lexer_config.is_none_or(|config| {
            !analysis
                .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        })
    {
        return ControlFlow::Break(None);
    }
    if let Some(occurrence) =
        crate::definition::original_variable_occurrence_at(source, analysis, line, character)
    {
        return ControlFlow::Break(Some(occurrence));
    }
    let offset = crate::definition::byte_offset_at(
        &tcl_lexer::LineIndex::new(source),
        source,
        line,
        character,
    );
    if analysis.body_lexer_config.is_some_and(|config| {
        analysis
            .original_variable_root_in_source(
                &tcl_lexer::SourceImage::document(source),
                config,
                offset,
            )
            .is_some()
    }) || crate::definition::original_variable_cursor_retained(source, analysis, line, character)
    {
        return ControlFlow::Break(None);
    }
    ControlFlow::Continue(())
}

/// One readonly navigation result. A caller template remains distinct from
/// original table-name geometry: it proves neither an entered alias nor a
/// successful store. Refactoring continues to use the original-only selector.
pub(crate) enum NavigationVariable<'a> {
    Original(&'a SignatureSourceVariableOccurrence),
    Alias(Box<OriginalVariableAliasTemplate>),
    LogicalFormal {
        procedure: Box<
            tcl_compiler::registry_invocation::logical_formals::OriginalLogicalProcedureBindings,
        >,
        ordinal: usize,
    },
    Caller {
        name: String,
        offset: u32,
        bindings: Vec<crate::caller_frame::CallerFrameBinding>,
    },
}

impl NavigationVariable<'_> {
    pub(crate) fn declaration_spans(&self, analysis: &AnalysisResult) -> Vec<Span> {
        match self {
            Self::Original(occurrence) => declaration_spans(analysis, occurrence.symbol()),
            Self::Alias(template) => declaration_spans(analysis, template.target_symbol()),
            Self::LogicalFormal { procedure, ordinal } => {
                vec![procedure.formals()[*ordinal].declaration_span()]
            }
            Self::Caller { bindings, .. } => crate::caller_frame::primary_binding(bindings)
                .map(|binding| vec![binding.arg_span])
                .unwrap_or_default(),
        }
    }

    pub(crate) fn reference_spans(
        &self,
        source: &str,
        analysis: &AnalysisResult,
        include_declaration: bool,
    ) -> Vec<Span> {
        match self {
            Self::Original(occurrence) => alias_template_reference_spans(
                source,
                analysis,
                occurrence.symbol(),
                include_declaration,
            ),
            Self::Alias(template) => alias_template_reference_spans(
                source,
                analysis,
                template.target_symbol(),
                include_declaration,
            ),
            Self::LogicalFormal { procedure, ordinal } => {
                let formal = &procedure.formals()[*ordinal];
                let mut spans = formal.references().iter().map(tcl_compiler::registry_invocation::logical_formals::OriginalLogicalFormalReference::name_span).collect::<Vec<_>>();
                if include_declaration {
                    spans.push(formal.declaration_span());
                }
                spans
            }
            Self::Caller {
                name,
                offset,
                bindings,
            } => crate::caller_frame::caller_frame_reference_spans_from_bindings(
                analysis, source, *offset, name, bindings,
            )
            .into_iter()
            .filter(|span| {
                include_declaration
                    || !bindings
                        .iter()
                        .any(|binding| !binding.read_only && binding.arg_span == *span)
            })
            .collect(),
        }
    }

    pub(crate) fn hover_text(&self, analysis: &AnalysisResult) -> Option<String> {
        match self {
            Self::Original(occurrence) => hover_text(analysis, occurrence),
            Self::Alias(template) => Some(format!(
                "**Conditional variable alias** `{}` → `{}`\n\nThe original declaration relates these names. Target element, successful alias and binding continuity are unproved.",
                tcl_syntax::native_string::resident_name_label(template.local_name().as_bytes()),
                tcl_syntax::native_string::resident_name_label(
                    template.target_name_input().bytes()
                ),
            )),
            Self::LogicalFormal { procedure, ordinal } => Some(format!(
                "`{}` — declared Logical procedure parameter",
                procedure.formals()[*ordinal].name()
            )),
            Self::Caller { name, bindings, .. } => crate::caller_frame::primary_binding(bindings)
                .map(|binding| crate::caller_frame::caller_frame_hover_text(name, binding)),
        }
    }
}

/// Select current original variable navigation, allowing an undeclared read
/// only when the independently sealed caller template joins the call and
/// read in their actual source frame. Unavailable original operands remain
/// definitive; no display-name or same-named command fallback is permitted.
/// Logical callers retain their separate authored navigation consumer.
pub(crate) fn select_navigation<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    line: u32,
    character: u32,
    resolution: crate::definition::CallResolution<'_>,
) -> ControlFlow<Option<NavigationVariable<'a>>> {
    // naming.minifier.logical-formal-binding-alpha
    // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
    // The sealed factory positively selects the full Logical source owner;
    // inherited authored simulation in the compatibility flag is separate.
    if let Some(formal) = logical_formal_navigation(source, analysis, line, character) {
        return ControlFlow::Break(Some(formal));
    }
    let selected = select(source, analysis, line, character);
    if analysis.allows_lexical_declaration_advice() {
        return selected.map_break(|selected| selected.map(NavigationVariable::Original));
    }
    let Some(config) = analysis.body_lexer_config else {
        return ControlFlow::Break(None);
    };
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return ControlFlow::Break(None);
    }
    let offset = crate::definition::byte_offset_at(
        &tcl_lexer::LineIndex::new(source),
        source,
        line,
        character,
    );
    if let Some(template) = alias_template_at_offset(source, analysis, config, offset) {
        return ControlFlow::Break(Some(NavigationVariable::Alias(Box::new(template))));
    }
    let Some(profile) = analysis.resolved_profile() else {
        return ControlFlow::Break(None);
    };
    match selected {
        ControlFlow::Break(Some(occurrence)) => {
            if !declaration_spans(analysis, occurrence.symbol()).is_empty() {
                return ControlFlow::Break(Some(NavigationVariable::Original(occurrence)));
            }
            // The genuine original read supplies geometry. The template,
            // rather than this text projection, supplies the frame relation.
            let tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(
                root,
            ) = occurrence.original_name_input()
            else {
                return ControlFlow::Break(None);
            };
            let Some(name) = root
                .name_span()
                .and_then(|span| source.get(span.as_range()))
            else {
                return ControlFlow::Break(None);
            };
            let bindings = crate::caller_frame::caller_frame_bindings(
                analysis, source, profile, resolution, offset, name,
            );
            ControlFlow::Break((!bindings.is_empty()).then(|| NavigationVariable::Caller {
                name: name.to_owned(),
                offset,
                bindings,
            }))
        }
        ControlFlow::Break(None) => ControlFlow::Break(None),
        ControlFlow::Continue(()) => {
            let Some((word, _, _)) =
                crate::hover::find_word_span_at_position(source, line, character)
            else {
                return ControlFlow::Continue(());
            };
            if crate::caller_frame::binding_at_offset(
                analysis, source, profile, resolution, offset, &word,
            )
            .is_none()
            {
                return ControlFlow::Continue(());
            }
            let bindings = crate::caller_frame::caller_frame_bindings(
                analysis, source, profile, resolution, offset, &word,
            );
            ControlFlow::Break((!bindings.is_empty()).then(|| NavigationVariable::Caller {
                name: word,
                offset,
                bindings,
            }))
        }
    }
}

fn logical_formal_navigation(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
) -> Option<NavigationVariable<'static>> {
    let offset = crate::definition::byte_offset_at(
        &tcl_lexer::LineIndex::new(source),
        source,
        line,
        character,
    );
    if let Some(procedures) =
        tcl_compiler::registry_invocation::logical_formals::original_logical_procedure_bindings(
            source, analysis,
        )
    {
        for procedure in procedures {
            if let Some(ordinal) = procedure
                .formals()
                .iter()
                .position(|formal| procedure.formal_at(offset) == Some(formal))
            {
                return Some(NavigationVariable::LogicalFormal {
                    procedure: Box::new(procedure),
                    ordinal,
                });
            }
        }
    }
    None
}

fn alias_template_at_offset(
    source: &str,
    analysis: &AnalysisResult,
    config: tcl_lexer::LexerConfig,
    offset: u32,
) -> Option<OriginalVariableAliasTemplate> {
    let templates = analysis.original_variable_alias_templates_in_source(
        &tcl_lexer::SourceImage::document(source),
        config,
    )?;
    let mut matching = templates
        .into_iter()
        .filter(|template| template.span().start() <= offset && offset < template.span().end())
        .collect::<Vec<_>>();
    matching.sort_by_key(|template| template.span().end() - template.span().start());
    let first = matching.first()?;
    let shortest = first.span().end() - first.span().start();
    matching
        .iter()
        .take_while(|template| template.span().end() - template.span().start() == shortest)
        .all(|template| {
            template.target_symbol() == first.target_symbol()
                && template.original_frame() == first.original_frame()
                && template.local_name() == first.local_name()
        })
        .then(|| first.clone())
}

fn alias_template_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    symbol: &SignatureSourceVariableSymbol,
    include_declaration: bool,
) -> Vec<Span> {
    let mut spans = reference_spans(analysis, symbol, include_declaration);
    if let Some(config) = analysis.body_lexer_config
        && let Some(templates) = analysis.original_variable_alias_templates_in_source(
            &tcl_lexer::SourceImage::document(source),
            config,
        )
    {
        spans.extend(
            templates
                .into_iter()
                .filter(|template| {
                    template.target_symbol() == symbol
                        && (include_declaration || !template.is_declaration())
                })
                .map(|template| template.span()),
        );
    }
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

/// Original naming occurrences of one selected symbol, without reconstructed
/// reporting keys. The caller independently checks its current consumer source.
pub fn occurrences<'a>(
    analysis: &'a AnalysisResult,
    symbol: &'a SignatureSourceVariableSymbol,
) -> impl Iterator<Item = &'a SignatureSourceVariableOccurrence> {
    analysis
        .original_variable_symbols
        .iter()
        .filter(move |occurrence| occurrence.symbol() == symbol)
}

/// Exact declaration-name extents for readonly source navigation.
#[must_use]
pub fn declaration_spans(
    analysis: &AnalysisResult,
    symbol: &SignatureSourceVariableSymbol,
) -> Vec<Span> {
    spans(analysis, symbol, true, false)
}

/// Exact occurrence extents, with an explicit declaration inclusion policy.
#[must_use]
pub fn reference_spans(
    analysis: &AnalysisResult,
    symbol: &SignatureSourceVariableSymbol,
    include_declaration: bool,
) -> Vec<Span> {
    spans(analysis, symbol, include_declaration, true)
}

fn spans(
    analysis: &AnalysisResult,
    symbol: &SignatureSourceVariableSymbol,
    declarations: bool,
    references: bool,
) -> Vec<Span> {
    let mut spans: Vec<_> = occurrences(analysis, symbol)
        .filter(|occurrence| {
            if occurrence.is_declaration() {
                declarations
            } else {
                references
            }
        })
        .map(SignatureSourceVariableOccurrence::span)
        .collect();
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

/// Render original source naming facts. This does not infer a value or type
/// from the optional reporting scope tree.
#[must_use]
pub fn hover_text(
    analysis: &AnalysisResult,
    occurrence: &SignatureSourceVariableOccurrence,
) -> Option<String> {
    let declarations = declaration_spans(analysis, occurrence.symbol()).len();
    if declarations == 0 {
        return None;
    }
    let references = reference_spans(analysis, occurrence.symbol(), false).len();
    let input = occurrence.original_name_input();
    let label = tcl_syntax::native_string::resident_name_label(input.bytes());
    Some(format!(
        "**Variable** `{label}`\n\n{declarations} declaration(s) and {references} reference(s) in this document"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_alias_navigation_joins_name_templates_without_original_cell_selection() {
        // Implementation contract: naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        let source = "proc p {i} {upvar ::n::a($i) data; lappend data value; puts $data}; proc sibling {} {puts $data}";
        for version in tcl_dialect::TclVersion::ALL {
            let mut analysis = Analyser::new().analyse(source, version.dialect_name());
            analysis.all_variables.clear();
            analysis.all_procs.clear();
            analysis.global_scope.variables.clear();
            analysis.global_scope.children.clear();
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let target = source.find("::n::a").unwrap();
            let expected = [
                target,
                source.find(" data;").unwrap() + 1,
                source.find("data value").unwrap(),
                source.find("$data").unwrap(),
            ];
            for offset in expected.iter().copied().skip(1) {
                let selected = select_navigation(
                    source,
                    &analysis,
                    0,
                    u32::try_from(offset + usize::from(source.as_bytes()[offset] == b'$')).unwrap(),
                    crate::definition::CallResolution::document_only()
                        .with_registry(context.commands()),
                );
                let ControlFlow::Break(Some(NavigationVariable::Alias(template))) = selected else {
                    panic!(
                        "{}: readonly alias source selection at {offset}",
                        version.dialect_name()
                    );
                };
                assert_eq!(
                    declaration_spans(&analysis, template.target_symbol()),
                    [Span::new(
                        u32::try_from(target).unwrap(),
                        u32::try_from(source.find("($i)").unwrap() + 4).unwrap()
                    )]
                );
                let navigation = NavigationVariable::Alias(template.clone());
                assert_eq!(
                    navigation
                        .reference_spans(source, &analysis, true)
                        .iter()
                        .map(|span| usize::try_from(span.start()).unwrap())
                        .collect::<Vec<_>>(),
                    expected
                );
                assert!(
                    navigation
                        .hover_text(&analysis)
                        .unwrap()
                        .contains("Conditional variable alias")
                );
                assert!(navigation.hover_text(&analysis).unwrap().contains(
                    "Target element, successful alias and binding continuity are unproved"
                ));
                assert!(
                    !matches!(select(source, &analysis, 0, u32::try_from(offset).unwrap()), ControlFlow::Break(Some(original)) if original.symbol() == template.target_symbol()),
                    "the new navigation template does not mint original cell selection"
                );
            }
            let sibling = source.rfind("$data").unwrap();
            assert!(!matches!(
                select_navigation(
                    source,
                    &analysis,
                    0,
                    u32::try_from(sibling + 1).unwrap(),
                    crate::definition::CallResolution::document_only()
                        .with_registry(context.commands())
                ),
                ControlFlow::Break(Some(NavigationVariable::Alias(_)))
            ));
            let mut foreign = analysis.clone();
            foreign.resolved_input = Some(
                Analyser::new()
                    .analyse(source, "jim")
                    .resolved_input
                    .unwrap(),
            );
            assert!(matches!(
                select_navigation(
                    source,
                    &foreign,
                    0,
                    u32::try_from(expected[1]).unwrap(),
                    crate::definition::CallResolution::document_only()
                        .with_registry(context.commands())
                ),
                ControlFlow::Break(None)
            ));
        }
    }

    #[test]
    fn original_navigation_keeps_caller_templates_separate_from_unbound_symbols() {
        // Implementation contract: naming.core.original-caller-frame-navigation
        // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
        let source = "proc make {n} {upvar 1 $n target; set target 1}\nproc caller {} {make shared; puts $shared}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0");
        analysis.all_procs.clear();
        analysis.global_scope.children.clear();
        let position = source.lines().nth(1).unwrap().find("$shared").unwrap() + 2;
        let ControlFlow::Break(Some(selected)) = select_navigation(
            source,
            &analysis,
            1,
            u32::try_from(position).unwrap(),
            crate::definition::CallResolution::document_only(),
        ) else {
            panic!("an original caller template joins this read")
        };
        assert!(matches!(&selected, NavigationVariable::Caller { .. }));
        assert_eq!(selected.declaration_spans(&analysis).len(), 1);
        assert_eq!(selected.reference_spans(source, &analysis, true).len(), 2);
        assert_eq!(selected.reference_spans(source, &analysis, false).len(), 1);
        assert!(
            selected
                .hover_text(&analysis)
                .unwrap()
                .contains("Caller-frame variable")
        );
        assert!(matches!(
            select_navigation(
                &source.replace("target 1", "target 2"),
                &analysis,
                1,
                u32::try_from(position).unwrap(),
                crate::definition::CallResolution::document_only(),
            ),
            ControlFlow::Break(None)
        ));
        for level in ["0", "\\#0", "2"] {
            let source = source.replace("upvar 1", &format!("upvar {level}"));
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            assert!(
                matches!(
                    select_navigation(
                        &source,
                        &analysis,
                        1,
                        u32::try_from(position).unwrap(),
                        crate::definition::CallResolution::document_only(),
                    ),
                    ControlFlow::Break(None)
                ),
                "upvar {level} supplies no caller-frame template"
            );
        }
    }

    #[test]
    fn original_variable_editor_selection_uses_bytes_and_complete_source_currency() {
        // Implementation contract: naming.editor.original-variable-symbol-selection
        // docs/design/analysis/name-resolution-proofs/editor-original-variable-symbol-selection.md
        let source = r"set ::N::v\uD800 1
set ::N::v\uD801 2
info exists ::N::v\uD800";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        for reference in &mut analysis.qualified_var_refs {
            reference.qualified_name.clear();
        }
        let ControlFlow::Break(Some(selected)) = select(source, &analysis, 2, 14) else {
            panic!("an authentic original variable operand is selected");
        };
        assert_eq!(declaration_spans(&analysis, selected.symbol()).len(), 1);
        assert_eq!(
            reference_spans(&analysis, selected.symbol(), false).len(),
            1
        );
        assert_eq!(reference_spans(&analysis, selected.symbol(), true).len(), 2);
        assert!(
            hover_text(&analysis, selected)
                .unwrap()
                .contains("1 declaration(s) and 1 reference(s)")
        );
        assert!(matches!(
            select(&source.replace("exists", "unset "), &analysis, 2, 14),
            ControlFlow::Break(None)
        ));
    }
}

#[cfg(test)]
mod logical_formal_tests {
    #[test]
    fn logical_navigation_reuses_declared_formal_scope_and_exact_reads() {
        // naming.minifier.logical-formal-binding-alpha
        // docs/design/analysis/name-resolution-proofs/logical-formal-binding-alpha.md
        let source = "proc general {longleft longright} {list $longleft $longright $longleft}";
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let analysis = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let offset = source.find("$longleft").unwrap() + 1;
        let std::ops::ControlFlow::Break(Some(selected)) = super::select_navigation(
            source,
            &analysis,
            0,
            u32::try_from(offset).unwrap(),
            crate::definition::CallResolution::document_only(),
        ) else {
            panic!("original Logical formal selection is unavailable");
        };
        assert!(matches!(
            selected,
            super::NavigationVariable::LogicalFormal { .. }
        ));
        let declarations = selected.declaration_spans(&analysis);
        assert_eq!(declarations.len(), 1);
        assert_eq!(source.get(declarations[0].as_range()), Some("longleft"));
        let references = selected.reference_spans(source, &analysis, false);
        assert_eq!(references.len(), 2);
        assert!(
            references
                .iter()
                .all(|span| source.get(span.as_range()) == Some("longleft"))
        );
        assert_eq!(selected.reference_spans(source, &analysis, true).len(), 3);
        assert!(tcl_compiler::registry_invocation::logical_formals::original_logical_procedure_bindings(&format!("{source} "), &analysis).is_none());
    }
}
