// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly current-source command selection over the shared body geometry.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::SegmentedCommand;

/// Select the innermost command in the authentic original source structure.
/// Body and lambda extents come from the shared source-role owner. This
/// geometry supplies no execution, frame, store, motion or edit permission.
#[must_use]
pub fn find_original_command_at(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
) -> Option<SegmentedCommand> {
    let config = analysis.body_lexer_config?;
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    let mut selected: Option<SegmentedCommand> = None;
    let mut selected_width = u32::MAX;
    for command in structure.commands {
        let (start, end) = super::command_span_offsets(source, &command);
        if cursor < start || cursor > end {
            continue;
        }
        let width = end.checked_sub(start)?;
        if selected.is_none() || width < selected_width {
            selected = Some(command);
            selected_width = width;
        } else if width == selected_width
            && selected.as_ref().is_some_and(|previous| {
                previous.span != command.span || previous.all_tokens != command.all_tokens
            })
        {
            return None;
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_command_cursor_uses_actual_body_geometry_after_reports_are_erased() {
        // Implementation contract: naming.source.original-refactor-command-selection
        // docs/design/analysis/name-resolution-proofs/original-refactor-command-selection.md
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let source = "proc p {} {\n    set timeout 30\n    puts $timeout\n}\np";
            let mut analysis = Analyser::new().analyse(source, engine);
            analysis.all_procs.clear();
            analysis.command_invocations.clear();
            analysis.global_scope.variables.clear();
            let cursor = u32::try_from(source.find("set timeout").unwrap()).unwrap();
            let command = find_original_command_at(source, cursor, &analysis).unwrap();
            assert_eq!(command.span.start(), cursor, "{engine}");
            assert_eq!(command.name(), "set", "{engine}");
            assert_eq!(command.texts, ["set", "timeout", "30"], "{engine}");
            assert!(
                find_original_command_at(&source.replace("30", "31"), cursor, &analysis).is_none(),
                "complete source currency remains required for {engine}"
            );
        }
    }

    #[test]
    fn original_command_cursor_keeps_lambda_and_substitution_extents_separate() {
        // Implementation contract: naming.source.original-refactor-command-selection
        // docs/design/analysis/name-resolution-proofs/original-refactor-command-selection.md
        let source = "apply {{arg} {set local [list $arg]; return $local}} VALUE";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.command_invocations.clear();
        for (needle, expected) in [
            ("set local", "set"),
            ("list $arg", "list"),
            ("return $local", "return"),
        ] {
            let cursor = u32::try_from(source.find(needle).unwrap()).unwrap();
            let command = find_original_command_at(source, cursor, &analysis).unwrap();
            assert_eq!(command.span.start(), cursor);
            assert_eq!(command.name(), expected);
        }
        let quoted = r#"proc p {} "set local \u0031""#;
        let analysis = Analyser::new().analyse(quoted, "tcl8.6");
        let cursor = u32::try_from(quoted.find("set local").unwrap()).unwrap();
        let command = find_original_command_at(quoted, cursor, &analysis).unwrap();
        assert_eq!(command.span.start(), 0);
        assert_eq!(command.name(), "proc");
    }
}
