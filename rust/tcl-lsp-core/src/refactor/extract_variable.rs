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
/// returns a disabled action with no edits. The existing explicit Logical
/// authoring feature proposes conditional source edits, not runtime cell,
/// observer, store or evaluation equivalence. Its retained recipe checks source
/// occupancy, original word ordering and insertion geometry independently of
/// source-advice eligibility. Hosted lexical compatibility remains separate.
#[must_use]
pub fn extract_variable(
    source: &str,
    selection: (u32, u32),
    var_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    let (start, end) = selection;
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let config = current.config();
    let image = SourceImage::document(source);
    if end <= start {
        return None;
    }
    let selected = source.get(start as usize..end as usize)?;
    if selected.trim().is_empty() {
        return None;
    }
    let proposed_reference = format!("${var_name}");
    let proposed =
        tcl_lexer::word_parts::whole_var_ref(proposed_reference.as_bytes(), config.nested())
            .ok()??;
    if proposed.index.is_some()
        || proposed.name != var_name.as_bytes()
        || tcl_syntax::naming::is_qualified(var_name.as_bytes())
    {
        return None;
    }
    let registry = current.registry();
    // Structural source selection precedes purpose-specific advice. An
    // unavailable edit licence does not make a malformed selection a candidate.
    let value = logical_value_word(selected, config.nested())?;
    let command = super::find_original_command_at(source, start, analysis)?;
    let walk = super::FrameWalk::new(source, analysis)?;
    let (command_start, command_end) = walk.command_span_offsets(source, &command)?;
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
    if analysis
        .resolved_input
        .as_ref()?
        .has_logical_source_name_context()
    {
        return LogicalVariableExtractionRecipe::capture(
            source,
            analysis,
            &command,
            Span::new(start, end),
            var_name,
            value,
        )?
        .into_source_action(source, analysis, line_index);
    }
    if !crate::rename::is_safe_symbol_name(var_name) {
        return None;
    }
    let proposed_reference = tcl_syntax::naming::scalar_source_reference(var_name, config, false)?;
    let setter = unique_logical_command(registry, analysis, LoweringHookId::Set)?;
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
    Some(extracted_variable_action(
        var_name,
        Span::new(start, end),
        insertion,
        assignment,
        proposed_reference,
    ))
}

fn extracted_variable_action(
    var_name: &str,
    selection: Span,
    insertion: u32,
    assignment: String,
    reference: String,
) -> Refactoring {
    Refactoring {
        title: format!("Extract into variable '${var_name}'"),
        edits: vec![
            RefactorEdit {
                start: insertion,
                end: insertion,
                new_text: assignment,
            },
            RefactorEdit {
                start: selection.start(),
                end: selection.end(),
                new_text: reference,
            },
        ],
        kind: ActionKind::RefactorExtract,
        data_group: None,
        disabled: None,
    }
}

/// The existing Logical extraction feature proposes an authored edit, not an
/// execution-equivalence promise. This recipe owns actual source, invocation,
/// scalar spelling, ordering and insertion data; unresolved runtime observers,
/// inserted-cell/store semantics and reached-frame obligations remain separate.
struct LogicalVariableExtractionRecipe {
    image: SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    invocation: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    selected: tcl_lexer::NativeWord,
    name: String,
    setter: String,
    value: String,
    reference: String,
    command_start: u32,
}

impl LogicalVariableExtractionRecipe {
    fn capture(
        source: &str,
        analysis: &AnalysisResult,
        command: &tcl_compiler::segmenter::SegmentedCommand,
        selected: Span,
        name: &str,
        value: String,
    ) -> Option<Self> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let input = analysis.resolved_input.as_ref()?;
        if !input.has_logical_source_name_context() || !crate::rename::is_safe_symbol_name(name) {
            return None;
        }
        let config = current.config();
        let walk = super::FrameWalk::new(source, analysis)?;
        let (command_start, command_end) = walk.command_span_offsets(source, command)?;
        let invocation = walk.source_words(source, command)?;
        let mut selected_words = (0..invocation.arguments().len()).filter_map(|argument| {
            invocation
                .original_argument_word(argument)
                .filter(|word| word.word_span() == selected)
        });
        let word = selected_words.next()?.clone();
        if selected_words.any(|other| *other != word) {
            return None;
        }
        // Preserve the source's left-to-right operand evaluation order: only
        // static, unexpanded words may precede the extracted word. This does
        // not prove that an added setter/read is invisible to runtime observers.
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(source),
            Span::new(command_start, command_end),
            config,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let [original] = plan.commands.as_slice() else {
            return None;
        };
        for before in original
            .words
            .iter()
            .take_while(|before| before.word_span() != selected)
        {
            if before.group().expand
                || tcl_syntax::word_rules::original_static_word_unicode_value(before).is_none()
            {
                return None;
            }
        }
        if !Self::source_spelling_available(source, analysis, name, &walk) {
            return None;
        }
        let setter = unique_logical_command(current.registry(), analysis, LoweringHookId::Set)?;
        match analysis
            .retained_command_realm()?
            .binding_at(setter, command_start)
        {
            tcl_compiler::realm::RealmBindingFact::Unchanged => {}
            tcl_compiler::realm::RealmBindingFact::Command(selected)
                if current
                    .registry()
                    .get(selected)
                    .is_some_and(|spec| spec.lowering_hook == Some(LoweringHookId::Set)) => {}
            _ => return None,
        }
        Some(Self {
            image: SourceImage::document(source),
            input: input.clone(),
            invocation,
            selected: word,
            name: name.to_owned(),
            setter: setter.to_owned(),
            value,
            reference: tcl_syntax::naming::scalar_source_reference(name, config, false)?,
            command_start,
        })
    }

    fn source_spelling_available(
        source: &str,
        analysis: &AnalysisResult,
        name: &str,
        walk: &super::FrameWalk<'_>,
    ) -> bool {
        let mut unoccupied = true;
        let coverage = crate::executable_regions::visit_checked_analysis_executable_commands(
            source,
            analysis,
            &mut |command, _, _| {
                let names = super::extract_proc::logical_source_variable_spellings(
                    source, analysis, command, walk,
                );
                if names.as_ref().is_none_or(|names| names.contains(name)) {
                    unoccupied = false;
                }
                let traits = walk.source_traits(source, command);
                let observers = tcl_registry::Traits::INTROSPECTS_BY_NAME
                    | tcl_registry::Traits::CURRENT_FRAME_INTROSPECTION
                    | tcl_registry::Traits::TARGETS_VARIABLE_BY_NAME
                    | tcl_registry::Traits::ESTABLISHES_VARIABLE_TRACE
                    | tcl_registry::Traits::CREATES_SCOPE_ALIAS
                    | tcl_registry::Traits::DYNAMIC_EVAL_BODY;
                if traits.is_none_or(|traits| traits.intersects(observers)) {
                    unoccupied = false;
                }
                false
            },
        );
        unoccupied
            && walk.complete()
            && coverage == crate::executable_regions::PotentialSourceVisit::Complete
    }

    fn into_source_action(
        self,
        source: &str,
        analysis: &AnalysisResult,
        line_index: &LineIndex,
    ) -> Option<Refactoring> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image.bytes() != source.as_bytes()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || !self
                .invocation
                .matches_source(&self.image, current.config())
            || self.selected.image() != &self.image
            || self.selected.config() != current.config()
        {
            return None;
        }
        let line_start = line_index.line_start(line_index.line_at(self.command_start));
        let before = source.get(line_start as usize..self.command_start as usize)?;
        let (insertion, assignment) = if before.trim().is_empty() {
            (
                line_start,
                format!("{before}{} {} {}\n", self.setter, self.name, self.value),
            )
        } else {
            (
                self.command_start,
                format!("{} {} {}; ", self.setter, self.name, self.value),
            )
        };
        let selected = self.selected.word_span();
        Some(Refactoring {
            title: format!("Extract into variable '${}'", self.name),
            edits: vec![
                RefactorEdit {
                    start: insertion,
                    end: insertion,
                    new_text: assignment,
                },
                RefactorEdit {
                    start: selected.start(),
                    end: selected.end(),
                    new_text: self.reference,
                },
            ],
            kind: ActionKind::RefactorExtract,
            data_group: None,
            disabled: None,
        })
    }
}

fn unique_logical_command<'a>(
    registry: &'a CommandRegistry,
    analysis: &AnalysisResult,
    hook: LoweringHookId,
) -> Option<&'a str> {
    let context = tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        analysis.resolved_input.as_ref()?,
        analysis.body_lexer_config?,
        analysis.resolved_profile(),
    )?;
    let mut names = registry
        .command_names_for_semantic_operation(SemanticOperationId::StructuredLowering(hook))
        .filter(|name| {
            context
                .context()
                .resolve_spec(registry, name)
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
    use tcl_compiler::ssa::SsaSourceView;
    let unavailable =
        || "No current original scalar availability and setter proposal is retained".to_owned();
    let config = analysis.body_lexer_config.ok_or_else(unavailable)?;
    let registry = analysis.resolved_registry().ok_or_else(unavailable)?;
    let input = analysis.resolved_input.as_ref().ok_or_else(unavailable)?;
    tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )
    .ok_or_else(unavailable)?;
    let image = SourceImage::document(source);
    let unit = assignment_candidate_unit(source, registry, config, input);
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
                    require_assignment_name_coverage(function, &native_name)?;
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

fn assignment_candidate_unit(
    source: &str,
    registry: &CommandRegistry,
    config: LexerConfig,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> tcl_compiler::compilation_unit::CompilationUnit {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            config,
            dialect: Some(input.unit_profile()),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    )
}

fn require_assignment_name_coverage(
    function: &tcl_compiler::compilation_unit::FunctionUnit,
    native_name: &[u8],
) -> Result<(), String> {
    use tcl_compiler::var_resolve::VariableCellKey;
    // Missing locations cannot certify future-name coverage, even when a
    // separate source-symbol projection is unavailable.
    if function
        .ssa
        .variable_symbols()
        .any(|symbol| function.ssa.cell_key(symbol).is_none())
    {
        return Err("Future local-name coverage is unavailable".to_owned());
    }
    // Location inventory includes future represented uses and definitions;
    // its labels cannot prove that introducing this local avoids capture.
    for cell in function.ssa.cell_keys() {
        match cell.root() {
            VariableCellKey::Authored(_) => {
                return Err("Future local-name coverage is unavailable".to_owned());
            }
            VariableCellKey::Activation { simple, .. } if simple.as_bytes() == native_name => {
                return Err(
                    "The proposed local is named elsewhere in the complete function".to_owned(),
                );
            }
            _ => {}
        }
    }
    Ok(())
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

    #[test]
    fn logical_extraction_keeps_complete_original_scalar_value_closers() {
        let source = "puts ${source}\nputs done\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let end = u32::try_from("puts ${source}".len()).unwrap();
        let action = extract_variable(
            source,
            (5, end),
            "value",
            &analysis,
            &LineIndex::new(source),
        )
        .expect("a complete original scalar value word");
        assert!(action.disabled.is_none(), "{:?}", action.disabled);
        assert_eq!(
            action.apply(source),
            "set value ${source}\nputs $value\nputs done\n"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            extract_variable(source, (5, end), "value", &missing, &LineIndex::new(source))
                .is_none()
        );
        assert!(
            extract_variable(
                "puts ${changed}\nputs done\n",
                (5, end),
                "value",
                &analysis,
                &LineIndex::new(source)
            )
            .is_none()
        );
    }

    /// `expr_op_spellings()` includes the
    /// iRules word operators (`and`/`or`/`contains`/…), and an ordinary
    /// quoted string containing one of those words as English prose must
    /// NOT be mistaken for a real operator token — `set myvar "salt and
    /// pepper"` is already valid, wrapping it in `expr {…}` breaks it.
    #[test]
    fn logical_invalid_multiword_value_never_becomes_a_four_argument_set() {

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

        let source = "proc p {} {puts literal}";
        let start = u32::try_from(source.find("literal").unwrap()).unwrap();
        let action = run(source, start, start + 7, "result").unwrap();
        assert_eq!(
            action.apply(source),
            "proc p {} {set result literal; puts $result}"
        );
    }

    #[test]
    fn logical_extraction_keeps_supplied_availability_and_source_configuration() {

        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("set").unwrap().clone()
        });
        let commands = registry.snapshot().shared_registry();
        let current = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.6")
                .with_command_store(std::sync::Arc::clone(&commands)),
        );
        let older = std::sync::Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4").with_command_store(commands),
        );
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..LexerConfig::for_file_grammar(profile.grammar)
        };
        let source = "puts {salt and pepper}";
        let index = LineIndex::new(source);
        let end = u32::try_from(source.len()).unwrap();
        let analyse = |context| {
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile, profile, context, config,
            );
            Analyser::new()
                .with_resolved_input(input)
                .analyse(source, profile.name)
        };
        let analysis = analyse(std::sync::Arc::clone(&current));
        let action = extract_variable(source, (5, end), "seasoning", &analysis, &index)
            .expect("available source setter");
        assert!(action.disabled.is_none());
        assert_eq!(
            action.apply(source),
            "set seasoning {salt and pepper}\nputs $seasoning"
        );
        assert!(extract_variable(source, (5, end), "seasoning", &analyse(older), &index).is_none());
        assert!(
            crate::original_context::CurrentSourceContext::capture(source, &analysis).is_some()
        );
        assert!(analysis.allows_retained_logical_declaration_advice());

        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(extract_variable(source, (5, end), "seasoning", &missing, &index).is_none());
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            config,
        ));
        assert!(extract_variable(source, (5, end), "seasoning", &foreign, &index).is_none());
        let mut stale = analysis;
        stale.body_lexer_config.as_mut().unwrap().braced_var =
            tcl_dialect::BracedVarStyle::Tcl9Nesting;
        assert!(extract_variable(source, (5, end), "seasoning", &stale, &index).is_none());
    }

    #[test]
    fn original_variable_extraction_never_promotes_naming_proposals_to_motion_permission() {

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
    #[test]
    fn original_extraction_validates_complete_word_before_unlicensed_advice() {

        // Geometry is read-only; this genuine Native input supplies no motion or store licence.
        let source = "proc p {} {puts literal}; p";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let index = LineIndex::new(source);
        let start = u32::try_from(source.find("literal").unwrap()).unwrap();
        let valid = extract_variable(source, (start, start + 7), "result", &analysis, &index)
            .expect("one original complete value word remains a candidate");
        assert!(valid.disabled.is_some());
        assert!(valid.edits.is_empty());
        for malformed_name in ["a b", "arr(k)", "ns::result", "::result", "", "a; puts x"] {
            assert!(
                extract_variable(
                    source,
                    (start, start + 7),
                    malformed_name,
                    &analysis,
                    &index
                )
                .is_none()
            );
        }
        let unicode = extract_variable(source, (start, start + 7), "café", &analysis, &index)
            .expect("a complete Unicode scalar reference remains valid unlicensed advice");
        assert!(unicode.disabled.is_some());
        assert!(unicode.edits.is_empty());
        assert!(
            extract_variable(source, (start, start + 6), "result", &analysis, &index).is_none()
        );
        let source = "puts helper {a and b} $x";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            extract_variable(
                source,
                (5, u32::try_from(source.len()).unwrap()),
                "result",
                &analysis,
                &LineIndex::new(source),
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod logical_source_recipe_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn proposal(source: &str, selected: &str, name: &str) -> Option<Refactoring> {
        let start = u32::try_from(source.find(selected)?).ok()?;
        let end = start.checked_add(u32::try_from(selected.len()).ok()?)?;
        let analysis = Analyser::new().analyse(source, "tcl");
        extract_variable(
            source,
            (start, end),
            name,
            &analysis,
            &LineIndex::new(source),
        )
    }

    #[test]
    fn original_source_recipe_keeps_supported_edits_and_declines_capture_or_reordering() {
        let source = "puts [string length $name]";
        let action = proposal(source, "[string length $name]", "length").unwrap();
        assert!(action.disabled.is_none());
        assert_eq!(
            action.apply(source),
            "set length [string length $name]\nputs $length"
        );
        assert!(proposal("puts $value; puts literal", "literal", "value").is_none());
        assert!(proposal("proc p {value} {puts literal}", "literal", "value").is_none());
        assert!(
            proposal(
                "puts $channel [string length $name]",
                "[string length $name]",
                "length"
            )
            .is_none()
        );
        assert!(proposal("info locals; puts literal", "literal", "value").is_none());
    }
}
