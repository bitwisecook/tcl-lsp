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

//! Extract one original value word with the document's actual analysis axes.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LexerConfig, LineIndex, SourceImage, Span};
use tcl_registry::{CommandRegistry, SemanticOperationId, hooks::LoweringHookId};

use super::{RefactorEdit, Refactoring};
use crate::code_actions::ActionKind;

/// Extract `[selection.0, selection.1)` using the retained full source grammar
/// and Registry. Native naming proposals supply only name availability and
/// current setter selection; missing insertion, store or movement authority
/// returns a disabled action with no edits. Compatibility extraction is
/// restricted to the explicit lexical declaration-advice domain.
#[must_use]
pub fn extract_variable(
    source: &str,
    selection: (u32, u32),
    var_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    let (start, end) = selection;
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    if end <= start || !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let selected = source.get(start as usize..end as usize)?;
    if selected.trim().is_empty() {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    if !analysis.allows_lexical_declaration_advice() {
        let reason = original_assignment_candidate(source, Span::new(start, end), var_name, analysis)
            .err().unwrap_or_else(|| "Variable extraction requires an independently proved inserted store and evaluation movement".to_owned());
        return Some(Refactoring {
            title: format!("Extract into variable '{var_name}'"),
            edits: Vec::new(),
            kind: ActionKind::RefactorExtract,
            data_group: None,
            disabled: Some(reason),
        });
    }
    if !crate::rename::is_safe_symbol_name(var_name) {
        return None;
    }
    let value = logical_value_word(selected, config)?;
    let setter = unique_logical_command(registry, analysis, LoweringHookId::Set)?;
    let command = super::find_command_at(source, start, None, registry, config)?;
    let (command_start, command_end) = super::command_span_offsets(source, &command);
    if start < command_start || end > command_end {
        return None;
    }
    let command_words =
        tcl_lexer::native_script_words_in(image, Span::new(command_start, command_end), config)
            .ok()?;
    if command_words.fatal_tail.is_some()
        || !command_words
            .commands
            .iter()
            .flat_map(|command| command.words.iter().skip(1))
            .any(|word| word.word_span() == Span::new(start, end))
    {
        return None;
    }
    match analysis
        .retained_command_realm()?
        .binding_at(setter, command_start)
    {
        tcl_compiler::realm::RealmBindingFact::Unchanged => {}
        tcl_compiler::realm::RealmBindingFact::Command(name)
            if registry
                .get(name)
                .is_some_and(|spec| spec.lowering_hook == Some(LoweringHookId::Set)) => {}
        _ => return None,
    }
    let line_start = line_index.line_start(line_index.line_at(command_start));
    let before = source.get(line_start as usize..command_start as usize)?;
    let (insertion, assignment) = if before.trim().is_empty() {
        (line_start, format!("{before}{setter} {var_name} {value}\n"))
    } else {
        // A one-line body or a preceding command stays in its actual body;
        // column zero could place the new assignment outside that frame.
        (command_start, format!("{setter} {var_name} {value}; "))
    };
    Some(Refactoring {
        title: format!("Extract into variable '${var_name}'"),
        edits: vec![
            RefactorEdit {
                start: insertion,
                end: insertion,
                new_text: assignment,
            },
            RefactorEdit {
                start,
                end,
                new_text: format!("${var_name}"),
            },
        ],
        kind: ActionKind::RefactorExtract,
        data_group: None,
        disabled: None,
    })
}

fn unique_logical_command<'a>(
    registry: &'a CommandRegistry,
    analysis: &AnalysisResult,
    hook: LoweringHookId,
) -> Option<&'a str> {
    let profile = analysis.resolved_profile()?;
    let context = crate::document_context_for_profile(profile);
    let mut names = registry
        .command_names_for_semantic_operation(SemanticOperationId::StructuredLowering(hook))
        .filter(|name| {
            registry
                .get_for_surface(name, Some(context.authoring_query()))
                .is_some_and(|spec| spec.lowering_hook == Some(hook))
        });
    let first = names.next()?;
    names.next().is_none().then_some(first)
}

/// A complete word uses original lexical geometry; multiple value words need
/// a complete checked expression from the actual independently selected parser.
/// Recovery ASTs and operator substring guesses cannot issue that structure.
fn logical_value_word(selected: &str, config: LexerConfig) -> Option<String> {
    let image = SourceImage::document(selected);
    let plan = tcl_lexer::native_script_words_in(
        image,
        Span::new(0, u32::try_from(selected.len()).ok()?),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if command.words.len() == 1 {
        return Some(selected.to_owned());
    }
    // This entry has no independently installed logical expression parser
    // facet. A compatible profile cannot supply that separate capability.
    // Multiword expression extraction stays unavailable until its checked
    // logical parser and selected evaluator are retained by the same owner.
    None
}

/// Retain the independently selected point-owned naming proposal. Even a
/// successful proposal does not authorise an inserted store or source motion.
fn original_assignment_candidate(
    source: &str,
    selected: Span,
    var_name: &str,
    analysis: &AnalysisResult,
) -> Result<(), String> {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use tcl_compiler::ssa::SsaSourceView;
    use tcl_compiler::var_resolve::VariableCellKey;
    let unavailable =
        || "No current original scalar availability and setter proposal is retained".to_owned();
    let config = analysis.body_lexer_config.ok_or_else(unavailable)?;
    let registry = analysis.resolved_registry().ok_or_else(unavailable)?;
    let profile = analysis.resolved_profile().ok_or_else(unavailable)?;
    let image = SourceImage::document(source);
    let unit = CompilationUnit::build_with_options(
        source,
        UnitBuildOptions {
            registry,
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
    );
    let mut found = false;
    for function in unit.analysable_body_function_units() {
        if function.complexity_guarded || function.dynamic_barrier_blocks_value_motion() {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for index in 0..body.statements.len() {
                let view = SsaSourceView::at_statement(&function.ssa, block, index);
                let Some(tokens) = view.source_tokens() else {
                    continue;
                };
                let Some(binding) = tokens.source_binding.as_ref() else {
                    continue;
                };
                for written in 1..tokens.words().len() {
                    let Some(input) = binding.original_written_name_input(tokens, written) else {
                        continue;
                    };
                    let Some(key) = input.original_word_key() else {
                        continue;
                    };
                    if key.source_image() != &image
                        || key.lexer_config() != config
                        || key.original_word().word_span() != selected
                    {
                        continue;
                    }
                    if found {
                        return Err(
                            "The selected word has multiple source operation owners".to_owned()
                        );
                    }
                    let policy = input.policy();
                    let native_name = tcl_syntax::backslash::native_source_literal_bytes(
                        var_name.as_bytes(),
                        image.channel(),
                        policy.string_protocol(),
                    )
                    .map_err(|_| unavailable())?;
                    tcl_syntax::naming::native_scalar_source_spelling(
                        &native_name,
                        image.channel(),
                        config,
                        policy.recipe(),
                    )
                    .ok_or_else(|| {
                        "The proposed name has no exact scalar source spelling".to_owned()
                    })?;
                    let proposal = view
                        .fresh_scalar_assignment_proposal(&native_name, registry)
                        .ok_or_else(unavailable)?;
                    if proposal.variable().policy() != policy
                        || proposal.variable().lexer_config() != config
                        || proposal.command().lexer_config() != config
                        || !proposal.command().matches_original_point(binding)
                    {
                        return Err(unavailable());
                    }
                    // The complete function's typed cells include future uses
                    // and definitions. A compatibility label cannot prove that
                    // introducing this local would leave later capture unchanged.
                    for cell in function.ssa.cell_keys() {
                        match cell.root() {
                            VariableCellKey::Authored(_) => return Err("Future local-name coverage is unavailable".to_owned()),
                            VariableCellKey::Activation { simple, .. } if simple.as_bytes() == native_name.as_ref() =>
                                return Err("The proposed local is named elsewhere in the complete function".to_owned()),
                            _ => {}
                        }
                    }
                    found = true;
                }
            }
        }
    }
    if found {
        Ok(())
    } else {
        Err(
            "The selection is not a pure original complete word at a reached local source point"
                .to_owned(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        Analyser::new().analyse(source, "f5-irules")
    }
    fn run(source: &str, start: u32, end: u32, name: &str) -> Option<Refactoring> {
        let analysis = analyse(source);
        extract_variable(
            source,
            (start, end),
            name,
            &analysis,
            &LineIndex::new(source),
        )
    }

    #[test]
    fn logical_complete_value_words_keep_explicit_compatibility_behaviour() {
        // Implementation contract: naming.refactor.logical-single-word-extraction
        // docs/design/analysis/name-resolution-proofs/refactor-logical-single-word-extraction.md
        let source = "puts {salt and pepper}";
        let action = run(source, 5, u32::try_from(source.len()).unwrap(), "seasoning").unwrap();
        assert!(action.disabled.is_none());
        assert_eq!(
            action.apply(source),
            "set seasoning {salt and pepper}\nputs $seasoning"
        );
        let source = "puts [string length $name]";
        let action = run(source, 5, u32::try_from(source.len()).unwrap(), "length").unwrap();
        assert!(
            action
                .apply(source)
                .contains("set length [string length $name]")
        );
    }

    /// `expr_op_spellings()` includes the
    /// iRules word operators (`and`/`or`/`contains`/…), and an ordinary
    /// quoted string containing one of those words as English prose must
    /// NOT be mistaken for a real operator token — `set myvar "salt and
    /// pepper"` is already valid, wrapping it in `expr {…}` breaks it.
    #[test]
    fn logical_invalid_multiword_value_never_becomes_a_four_argument_set() {
        // Implementation contract: naming.refactor.logical-single-word-extraction
        // docs/design/analysis/name-resolution-proofs/refactor-logical-single-word-extraction.md
        let source = "puts helper {a and b} $x";
        assert!(run(source, 5, u32::try_from(source.len()).unwrap(), "result").is_none());
        assert!(
            run("puts $a + $b", 5, 12, "result").is_none(),
            "an absent independently selected logical expression parser is not operator advice"
        );
        assert!(run("puts literal", 5, 12, "a b").is_none());
        assert!(run("puts literal", 5, 12, "arr(k)").is_none());
        assert!(run("puts a; puts b", 5, 14, "result").is_none());
        assert!(run("puts literal", 0, 0, "result").is_none());
    }

    #[test]
    fn logical_one_line_body_insertion_remains_inside_the_original_body() {
        // Implementation contract: naming.refactor.logical-single-word-extraction
        // docs/design/analysis/name-resolution-proofs/refactor-logical-single-word-extraction.md
        let source = "proc p {} {puts literal}";
        let start = u32::try_from(source.find("literal").unwrap()).unwrap();
        let action = run(source, start, start + 7, "result").unwrap();
        assert_eq!(
            action.apply(source),
            "proc p {} {set result literal; puts $result}"
        );
    }

    #[test]
    fn original_variable_extraction_never_promotes_naming_proposals_to_motion_permission() {
        // Implementation contract: naming.refactor.original-variable-extraction-permission
        // docs/design/analysis/name-resolution-proofs/refactor-original-variable-extraction-permission.md
        for source in [
            "proc p {} {puts literal}; p",
            "proc p {} {puts literal; puts $result}; p",
            "proc p {} {proc set {args} {}; puts literal}; p",
            "proc p {} {puts [unknown]}; p",
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl8.6");
            let start = u32::try_from(
                source
                    .find("literal")
                    .or_else(|| source.find("[unknown]"))
                    .unwrap(),
            )
            .unwrap();
            let length = if source.get(start as usize..).unwrap().starts_with("literal") {
                7
            } else {
                9
            };
            let action = extract_variable(
                source,
                (start, start + length),
                "café",
                &analysis,
                &LineIndex::new(source),
            )
            .unwrap();
            assert!(action.disabled.is_some());
            assert!(action.edits.is_empty());
            analysis.all_procs.clear();
            analysis.global_scope.variables.clear();
            analysis.dialect = "f5-irules".to_owned();
            let same = extract_variable(
                source,
                (start, start + length),
                "café",
                &analysis,
                &LineIndex::new(source),
            )
            .unwrap();
            assert_eq!(action.disabled, same.disabled);
            assert!(
                extract_variable(
                    &format!("#{source}"),
                    (start, start + length),
                    "result",
                    &analysis,
                    &LineIndex::new(source)
                )
                .is_none()
            );
        }
    }
}
