// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly original expression-function selection, independent of script heads.

use std::ops::ControlFlow;

use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_compiler::command_binding::{
    OriginalMathFunctionOccurrence, SourceCommandDefinitionKind, SourceCommandReference,
};
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_lexer::{SourceImage, Span};
use tcl_registry::CommandSpec;

/// One current expression identifier and its independently selected target.
/// Missing metadata remains unknown. Fixed registrations do not acquire a
/// command slot, script-head input, wrapper availability or edit authority.
pub struct OriginalMathFunctionSelection<'a> {
    occurrence: OriginalMathFunctionOccurrence,
    procedure: Option<&'a SourceDeclarationMetadata<ProcDef>>,
    builtin: Option<&'a CommandSpec>,
}

impl<'a> OriginalMathFunctionSelection<'a> {
    /// Owned original expression identifier, topology and per-version purpose.
    #[must_use]
    pub const fn occurrence(&self) -> &OriginalMathFunctionOccurrence {
        &self.occurrence
    }

    /// Actual source-procedure reference, including a separately closed alias
    /// target. This can retain a foreign declaration without local metadata;
    /// its independent declaring document must join that exact allocation.
    #[must_use]
    pub fn proc_reference(&self) -> Option<&SourceCommandReference> {
        let reference = self.occurrence.command_reference()?;
        reference
            .linked_definition()
            .or_else(|| reference.definition())
            .filter(|definition| definition.kind() == SourceCommandDefinitionKind::Procedure)?;
        Some(reference)
    }

    /// Unique current original local declaration joined by full allocation site.
    /// Reporting maps, function spelling and namespace labels cannot supply it.
    #[must_use]
    pub const fn procedure_metadata(&self) -> Option<&'a SourceDeclarationMetadata<ProcDef>> {
        self.procedure
    }

    /// Metadata for the independently selected native/registered identity.
    /// Fixed-table functions bypass the command-wrapper availability purpose;
    /// this is documentation, without result or numeric coercion authority.
    #[must_use]
    pub const fn builtin_spec(&self) -> Option<&'a CommandSpec> {
        self.builtin
    }
}

/// Complete current original expression-function inventory. This is readonly
/// syntax/selection correspondence, not a complete runtime function roster.
/// `None` means absent source/configuration/Registry ownership, not no calls.
#[must_use]
pub fn occurrences(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<OriginalMathFunctionOccurrence>> {
    let config = analysis.body_lexer_config?;
    let registry = analysis.resolved_registry()?;
    let image = SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let region = Span::new(0, u32::try_from(image.len()).ok()?);
    let retained = analysis.original_math_functions_in_source(&image, config, region)?;
    retained
        .iter()
        .all(|row| row.matches_source(&image, config, registry))
        .then_some(retained)
}

/// Original expression identifiers referring to an independently current
/// procedure declaration. These are readonly references: a command-word edit
/// planner must refuse incomplete coverage until a separate expression-name
/// edit owner is available. `None` means missing correspondence, not no calls.
#[must_use]
pub fn procedure_references_in(
    source: &str,
    analysis: &AnalysisResult,
    declaration_source: &str,
    declaration_analysis: &AnalysisResult,
    declaration: &SourceDeclarationMetadata<ProcDef>,
    follow_links: bool,
) -> Option<Vec<OriginalMathFunctionOccurrence>> {
    let declaring_config = declaration_analysis.body_lexer_config?;
    let declaring_image = SourceImage::document(declaration_source);
    if !declaration_analysis.matches_original_source_image(&declaring_image, declaring_config)
        || declaration.name_input().source_image() != &declaring_image
        || declaration.name_input().lexer_config() != declaring_config
        || !declaration_analysis
            .original_procedure_declarations()
            .any(|current| current == declaration)
    {
        return None;
    }
    let retained = occurrences(source, analysis)?;
    if retained.iter().any(|row| {
        row.dispatch() == tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable
            && row.command_reference().is_none()
    }) {
        // An unresolved command-function can denote a foreign declaration;
        // filtering it out would turn missing coverage into an empty result.
        return None;
    }
    Some(
        retained
            .into_iter()
            .filter(|occurrence| {
                crate::original_declaration::math_function_targets_declaration_in(
                    source,
                    analysis,
                    declaration_source,
                    declaration_analysis,
                    occurrence,
                    declaration,
                    follow_links,
                )
            })
            .collect(),
    )
}

/// Exact original function identifier containing this byte offset. Equal
/// reporting spellings, command spans and independently copied inputs cannot
/// reselect a function occurrence; conflicting authentic rows abstain.
#[must_use]
pub fn at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<OriginalMathFunctionOccurrence> {
    let rows = occurrences(source, analysis)?;
    let mut matching = rows
        .into_iter()
        .filter(|row| row.span().start() <= offset && offset < row.span().end());
    let first = matching.next()?;
    matching.all(|row| row == first).then_some(first)
}

/// Select a readonly Native function target before ordinary command advice.
/// An original occurrence with missing target metadata is still terminal.
/// Advisory recorded function regions are May barriers only; they never mint
/// a target if the genuine expression producer is unavailable. Explicit lexical
/// analysis keeps its separate function advice after complete source currency.
#[must_use]
pub fn select_at_offset<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    offset: u32,
) -> ControlFlow<Option<OriginalMathFunctionSelection<'a>>> {
    let Some(config) = analysis.body_lexer_config else {
        return ControlFlow::Break(None);
    };
    let Some(registry) = analysis.resolved_registry() else {
        return ControlFlow::Break(None);
    };
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return ControlFlow::Break(None);
    }
    if analysis.allows_lexical_declaration_advice() {
        return ControlFlow::Continue(());
    }
    let Some(occurrence) = at(source, analysis, offset) else {
        return if analysis.command_invocations.iter().any(|invocation| {
            invocation.is_mathfunc_call
                && invocation.range.start() <= offset
                && offset < invocation.range.end()
        }) {
            ControlFlow::Break(None)
        } else {
            ControlFlow::Continue(())
        };
    };
    let builtin = occurrence.selected_registry_spec(registry);
    let procedure = occurrence.command_reference().and_then(|reference| {
        let definition = reference
            .linked_definition()
            .or_else(|| reference.definition())?;
        if definition.kind() != SourceCommandDefinitionKind::Procedure {
            return None;
        }
        let site = &definition.allocation().site;
        let mut selected = analysis
            .original_procedure_declarations()
            .filter(|declaration| {
                declaration.declaration_site() == site
                    && declaration.name_input().source_image() == &image
                    && declaration.name_input().lexer_config() == config
            });
        let first = selected.next()?;
        selected.all(|other| other == first).then_some(first)
    });
    ControlFlow::Break(Some(OriginalMathFunctionSelection {
        occurrence,
        procedure,
        builtin,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;
    use tcl_registry::mathfunc::NativeMathFunctionDispatch;

    #[test]
    fn original_math_selection_keeps_fixed_and_command_metadata_purposes_separate() {
        // Implementation contract: naming.core.original-math-function-selection
        // docs/design/analysis/name-resolution-proofs/core-original-math-function-selection.md
        let source = "expr {abs(-2)}\n";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.command_invocations.clear();
            analysis.all_procs.clear();
            let offset = u32::try_from(source.find("abs").unwrap()).unwrap();
            let ControlFlow::Break(Some(selected)) = select_at_offset(source, &analysis, offset)
            else {
                panic!("{dialect}: missing genuine function");
            };
            assert_eq!(selected.occurrence().bytes(), b"abs");
            assert!(selected.builtin_spec().is_some(), "{dialect}");
            assert!(selected.procedure_metadata().is_none());
            assert!(selected.proc_reference().is_none());
            if matches!(dialect, "tcl8.4" | "jim") {
                assert_eq!(
                    selected.occurrence().dispatch(),
                    NativeMathFunctionDispatch::FixedTable
                );
                assert!(selected.occurrence().command_reference().is_none());
            } else {
                assert_eq!(
                    selected.occurrence().dispatch(),
                    NativeMathFunctionDispatch::CommandTable
                );
            }
        }
    }

    #[test]
    fn original_math_selection_joins_actual_override_allocation_after_reporting_clear() {
        // Implementation contract: naming.core.original-math-function-selection
        // docs/design/analysis/name-resolution-proofs/core-original-math-function-selection.md
        let source = "proc ::tcl::mathfunc::abs {argument} {return OVERRIDE}\nexpr {abs(-2)}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.command_invocations.clear();
        let offset = u32::try_from(source.rfind("abs(").unwrap()).unwrap();
        let ControlFlow::Break(Some(selected)) = select_at_offset(source, &analysis, offset) else {
            panic!("missing genuine override");
        };
        assert!(selected.proc_reference().is_some());
        assert!(selected.builtin_spec().is_none());
        assert_eq!(
            selected.procedure_metadata().unwrap().metadata().params[0].name,
            "argument"
        );
        assert!(occurrences(&format!("# changed\n{source}"), &analysis).is_none());
        assert!(matches!(
            select_at_offset(&format!("# changed\n{source}"), &analysis, offset),
            ControlFlow::Break(None)
        ));
    }

    #[test]
    fn original_math_references_cannot_supply_a_command_word_rename() {
        // Implementation contract: naming.core.original-math-rename-coverage
        // docs/design/analysis/name-resolution-proofs/core-original-math-rename-coverage.md
        let source = "proc ::tcl::mathfunc::local {argument} {return $argument}\nexpr {local(1)}\n::tcl::mathfunc::local 2\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        let declaration = analysis.original_procedure_declarations().next().unwrap();
        let references =
            procedure_references_in(source, &analysis, source, &analysis, declaration, true)
                .unwrap();
        assert_eq!(references.len(), 1);
        assert_eq!(source.get(references[0].span().as_range()), Some("local"));
        assert!(references[0].command_reference().is_some());
        let function_call = analysis
            .command_invocations
            .iter()
            .find(|call| call.is_mathfunc_call)
            .unwrap();
        assert!(function_call.original_name_input.is_none());
        let offset = u32::try_from(source.find("local").unwrap()).unwrap();
        let refusal = crate::rename::rename_in_program(
            source,
            crate::profile_for_dialect("tcl8.6"),
            0,
            offset,
            "changed",
            &analysis,
            crate::definition::CallResolution {
                registry: analysis.resolved_registry(),
                program: None,
            },
        )
        .unwrap_err();
        assert!(
            refusal.reason.contains("expression identifier"),
            "{}",
            refusal.reason
        );
        assert!(
            procedure_references_in(
                &format!("# displaced\n{source}"),
                &analysis,
                source,
                &analysis,
                declaration,
                true
            )
            .is_none()
        );
    }

    #[test]
    fn original_math_unknown_target_does_not_borrow_an_unrelated_same_named_procedure() {
        // Implementation contract: naming.core.original-math-function-selection
        // docs/design/analysis/name-resolution-proofs/core-original-math-function-selection.md
        let source = "proc ghost {argument} {return WRONG}\nexpr {ghost(1)}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.rfind("ghost(").unwrap()).unwrap();
        assert!(analysis.all_procs.contains_key("::ghost"));
        let ControlFlow::Break(Some(selected)) = select_at_offset(source, &analysis, offset) else {
            panic!("missing original unresolved function");
        };
        assert!(selected.proc_reference().is_none());
        assert!(selected.procedure_metadata().is_none());
        assert!(selected.builtin_spec().is_none());
    }

    #[test]
    fn original_math_rename_coverage_keeps_unknown_command_functions_terminal() {
        let owner = "proc ::tcl::mathfunc::local {argument} {return $argument}\n";
        let analysis = Analyser::new().analyse(owner, "tcl8.6");
        let row = analysis.original_procedure_declarations().next().unwrap();
        let consumer = "expr {local(1)}\n";
        let reader = Analyser::new().analyse(consumer, "tcl8.6");
        let calls = occurrences(consumer, &reader).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].dispatch(),
            tcl_registry::mathfunc::NativeMathFunctionDispatch::CommandTable
        );
        assert!(calls[0].command_reference().is_none());
        assert!(procedure_references_in(consumer, &reader, owner, &analysis, row, true).is_none());
        for dialect in ["tcl8.4", "jimtcl"] {
            let owner_analysis = Analyser::new().analyse(owner, dialect);
            let consumer_analysis = Analyser::new().analyse(consumer, dialect);
            let row = owner_analysis
                .original_procedure_declarations()
                .next()
                .unwrap();
            let calls = occurrences(consumer, &consumer_analysis).unwrap();
            assert_eq!(
                calls[0].dispatch(),
                tcl_registry::mathfunc::NativeMathFunctionDispatch::FixedTable
            );
            assert!(
                procedure_references_in(
                    consumer,
                    &consumer_analysis,
                    owner,
                    &owner_analysis,
                    row,
                    true
                )
                .unwrap()
                .is_empty()
            );
        }
    }
}
