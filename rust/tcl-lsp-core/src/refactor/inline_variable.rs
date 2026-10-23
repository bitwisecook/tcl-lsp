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

//! Inline variable — replace a single-use variable with its value.

use tcl_compiler::analyser::AnalysisResult;
#[cfg(test)]
use tcl_compiler::analyser::types::{Scope, VarDef};
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{LexerConfig, LineIndex, Span};
use tcl_registry::CommandRegistry;

use super::{RefactorEdit, Refactoring};
use crate::code_actions::ActionKind;

/// Inline the variable defined by the `set` command at byte offset
/// `cursor`.
///
/// The explicit Logical route is a conditional source-authoring proposal with
/// retained original syntax/history and unresolved runtime observer, frame and
/// erasure/motion obligations; it is not a runtime-equivalence certificate.
/// The Native route retains its independent completed store/read proof.
///
/// Only inlines when the cursor is on a `set var value` command and the
/// variable has exactly one reference after the definition; returns
/// `None` otherwise.
#[must_use]
pub fn inline_variable(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    _registry: &CommandRegistry,
    _line_index: &LineIndex,
) -> Option<Refactoring> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let walk = super::FrameWalk::new(source, analysis)?;
    let cmd = super::find_original_command_at(source, cursor, analysis)?;
    let (var_name, ref_span, value_index) = if analysis.allows_lexical_declaration_advice() {
        let (var_name, value_index) = logical_set_operands(source, analysis, &cmd)?;
        let words = walk.source_words(source, &cmd)?;
        // Captured names have no declaration operand at this invocation.
        let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(_) =
            words.origins().get(1)?
        else {
            return None;
        };
        let name = words.original_argument_word(0)?.tokens().first()?;
        let image = tcl_lexer::SourceImage::document(source);
        let definition = analysis
            .logical_variable_scope_advice(&image, current.config(), name.span.start())?
            .variable_definition(&var_name)?;
        if definition.definition_span != name.span {
            return None;
        }
        let [reference] = definition.references.as_slice() else {
            return None;
        };
        let read_definition = analysis
            .logical_variable_scope_advice(&image, current.config(), reference.start())?
            .variable_definition(&var_name)?;
        if !std::ptr::eq(definition, read_definition) {
            return None;
        }
        (var_name, *reference, value_index)
    } else {
        original_inline_binding(source, analysis, &cmd)?
    };

    // Complete original value geometry preserves every selected delimiter.
    let words = walk.native_words(source, &cmd)?;
    let value_word = words.get(value_index)?;
    let value_text = source.get(value_word.span().as_range())?;

    // Delete the `set` command, and with it the whole line when the
    // command is alone on one: the leading indentation belongs to the
    // deleted line, so leaving it behind would push the following line
    // out by that much.
    let (mut cmd_start, mut cmd_end) = walk.command_span_offsets(source, &cmd)?;
    let line_start = source[..cmd_start as usize]
        .rfind('\n')
        .map_or(0, |nl| nl + 1);
    if source
        .get(line_start..cmd_start as usize)?
        .trim()
        .is_empty()
    {
        cmd_start = u32::try_from(line_start).ok()?;
    }
    if source.as_bytes().get(cmd_end as usize) == Some(&b'\n') {
        cmd_end += 1;
    }
    let delete = RefactorEdit {
        start: cmd_start,
        end: cmd_end,
        new_text: String::new(),
    };

    let ctx = reference_context(source, ref_span.start(), &var_name, analysis, &walk)?;
    let ref_start = ctx.reference.start();
    let ref_end = ctx.reference.end();

    // A standalone word keeps the complete value spelling. A fragment must
    // preserve the actual enclosing word's substitution components.
    let new_text = if ctx.word_is_single {
        value_text.to_owned()
    } else {
        interpolated_value_source(value_word, &ctx.word, ctx.reference)?
    };

    let replace = RefactorEdit {
        start: ref_start,
        end: ref_end,
        new_text,
    };

    let action = Refactoring {
        title: format!("Inline variable '${var_name}'"),
        edits: vec![delete, replace],
        kind: ActionKind::RefactorInline,
        data_group: None,
        disabled: None,
    };
    if analysis.allows_lexical_declaration_advice() {
        return LogicalVariableInliningRecipe::capture(
            source,
            analysis,
            &cmd,
            &var_name,
            (ref_span, value_word, &ctx.word),
            action,
        )?
        .into_source_action(source, analysis);
    }
    Some(action)
}

/// Conditional source-authoring intent, independently of a completed store or
/// physical value motion. Like `LogicalVariableExtractionRecipe`, this owner
/// offers an explicitly authored source operation, not runtime equivalence.
/// Unresolved observer/runtime/frame obligations are retained by purpose; no
/// Native Normal, source-cell erasure or completed RHS value is manufactured.
struct LogicalVariableInliningRecipe {
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    setter: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    rhs: tcl_lexer::NativeWord,
    receiver: tcl_lexer::NativeWord,
    name: String,
    declaration: Span,
    reference: Span,
    replacement: Span,
    postimage: tcl_lexer::SourceImage,
    action: Refactoring,
    obligations: InlineSourceObligations,
}

/// These remain unresolved by an authored source proposal. The recipe is not
/// accepted by Native optimisation, physical store/read or Normal APIs.
struct InlineSourceObligations {
    observer_horizon: tcl_lexer::SourceImage,
    moved_rhs: tcl_lexer::NativeWord,
    erased_statement: Span,
    substituted_read: Span,
}

impl LogicalVariableInliningRecipe {
    fn capture(
        source: &str,
        analysis: &AnalysisResult,
        command: &SegmentedCommand,
        name: &str,
        substitution: (Span, &tcl_lexer::NativeWord, &tcl_lexer::NativeWord),
        action: Refactoring,
    ) -> Option<Self> {
        let (reference, rhs, receiver) = substitution;
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        let input = analysis.resolved_input.as_ref()?;
        if !input.has_logical_source_name_context() {
            return None;
        }
        let image = tcl_lexer::SourceImage::document(source);
        let walk = super::FrameWalk::new(source, analysis)?;
        let setter = walk.source_words(source, command)?;
        let actual_read = reference_context(source, reference.start(), name, analysis, &walk)?;
        if &actual_read.word != receiver {
            return None;
        }
        let replacement = actual_read.reference;
        let (selected_name, value_index) = logical_set_operands(source, analysis, command)?;
        if selected_name != name
            || setter.origins().iter().enumerate().any(|(index, origin)| {
                !matches!(origin, tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) if *written == index)
            })
            || walk.native_words(source, command)?.get(value_index)? != rhs
            || rhs.image() != &image
            || receiver.image() != &image
            || rhs.config() != current.config()
            || receiver.config() != current.config()
            || replacement.start() < receiver.word_span().start()
            || replacement.end() > receiver.word_span().end()
            || rhs.group().expand
            || receiver.group().expand
        {
            return None;
        }
        let declaration = setter.original_argument_word(0)?.tokens().first()?.span;
        let definition = analysis
            .logical_variable_scope_advice(&image, current.config(), declaration.start())?
            .variable_definition(name)?;
        let read_definition = analysis
            .logical_variable_scope_advice(&image, current.config(), reference.start())?
            .variable_definition(name)?;
        if definition.definition_span != declaration
            || definition.references.as_slice() != [reference]
            || !std::ptr::eq(definition, read_definition)
            || reference.start() < walk.command_span_offsets(source, command)?.1
        {
            return None;
        }
        // The authored recipe does not move RHS evaluation across another
        // source statement or earlier dynamic operand in the receiver call.
        // The actual original region/word owners supply both inventories.
        let read_command = super::find_original_command_at(source, reference.start(), analysis)?;
        let (read_start, _) = walk.command_span_offsets(source, &read_command)?;
        source_read_follows_setter(source, analysis, &walk, command, &read_command)?;
        let read_words = walk.native_words(source, &read_command)?;
        let receiver_index = read_words.iter().position(|word| word == receiver)?;
        if read_words.iter().take(receiver_index).any(|word| {
            word.group().expand
                || tcl_syntax::word_rules::original_static_word_unicode_value(word).is_none()
        }) {
            return None;
        }
        // A source-history gate, not a proof that no runtime observer exists:
        // require complete selected command inventory and refuse authored
        // reflection, trace, scope-alias and dynamic-evaluation operations.
        let mut history_supported = true;
        let coverage = crate::executable_regions::visit_checked_analysis_executable_commands(
            source,
            analysis,
            &mut |command, _, _| {
                let observers = tcl_registry::Traits::INTROSPECTS_BY_NAME
                    | tcl_registry::Traits::CURRENT_FRAME_INTROSPECTION
                    | tcl_registry::Traits::ESTABLISHES_VARIABLE_TRACE
                    | tcl_registry::Traits::CREATES_SCOPE_ALIAS
                    | tcl_registry::Traits::DYNAMIC_EVAL_BODY;
                if walk
                    .source_traits(source, command)
                    .is_none_or(|traits| traits.intersects(observers))
                {
                    history_supported = false;
                }
                false
            },
        );
        if !history_supported
            || !walk.complete()
            || coverage != crate::executable_regions::PotentialSourceVisit::Complete
        {
            return None;
        }
        // Validate the exact proposed receiver invocation in its original
        // nested grammar. A complete document alone would not parse script
        // text inside a braced procedure body. Preserve the whole argv topology
        // and every unedited original word; offsets here are presentation only.
        let (_, read_end) = walk.command_span_offsets(source, &read_command)?;
        let replace = action
            .edits
            .iter()
            .find(|edit| edit.start == replacement.start() && edit.end == replacement.end())?;
        let mut proposed_call = source
            .get(usize::try_from(read_start).ok()?..usize::try_from(read_end).ok()?)?
            .to_owned();
        let local_start = usize::try_from(replacement.start().checked_sub(read_start)?).ok()?;
        let local_end = usize::try_from(replacement.end().checked_sub(read_start)?).ok()?;
        proposed_call.replace_range(local_start..local_end, &replace.new_text);
        let call_image = tcl_lexer::SourceImage::document(&proposed_call);
        let call_plan = tcl_lexer::native_script_words_in_with_purpose(
            call_image,
            Span::new(0, u32::try_from(proposed_call.len()).ok()?),
            current.config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        let [proposed_command] = call_plan.commands.as_slice() else {
            return None;
        };
        if call_plan.fatal_tail.is_some()
            || proposed_command.words.len() != read_words.len()
            || proposed_command.words[receiver_index].group().expand
        {
            return None;
        }
        for (index, (original, proposed)) in
            read_words.iter().zip(&proposed_command.words).enumerate()
        {
            if index != receiver_index && original.try_text().ok()? != proposed.try_text().ok()? {
                return None;
            }
        }
        // This is the complete actual proposed source, not an evaluated value.
        // Shared lexer/group/word ownership validates all enclosing grammar,
        // including delimiters around the replacement and removed statement.
        let proposed = action.apply(source);
        let postimage = tcl_lexer::SourceImage::document(&proposed);
        let post = tcl_lexer::native_script_words_in(
            postimage.clone(),
            Span::new(0, u32::try_from(proposed.len()).ok()?),
            current.config(),
        )
        .ok()?;
        if post.fatal_tail.is_some() || action.edits.len() != 2 {
            return None;
        }
        Some(Self {
            image,
            input: input.clone(),
            setter,
            rhs: rhs.clone(),
            receiver: receiver.clone(),
            name: name.to_owned(),
            declaration,
            reference,
            replacement,
            postimage,
            action,
            obligations: InlineSourceObligations {
                observer_horizon: tcl_lexer::SourceImage::document(source),
                moved_rhs: rhs.clone(),
                erased_statement: {
                    let (start, end) = walk.command_span_offsets(source, command)?;
                    Span::new(start, end)
                },
                substituted_read: replacement,
            },
        })
    }

    fn into_source_action(self, source: &str, analysis: &AnalysisResult) -> Option<Refactoring> {
        let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
        if self.image.bytes() != source.as_bytes()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || !self.setter.matches_source(&self.image, current.config())
            || self.rhs.image() != &self.image
            || self.receiver.image() != &self.image
            || self.rhs.config() != current.config()
            || self.receiver.config() != current.config()
            || self.postimage.bytes() != self.action.apply(source).as_bytes()
        {
            return None;
        }
        let definition = analysis
            .logical_variable_scope_advice(&self.image, current.config(), self.declaration.start())?
            .variable_definition(&self.name)?;
        let read = analysis
            .logical_variable_scope_advice(&self.image, current.config(), self.reference.start())?
            .variable_definition(&self.name)?;
        if !std::ptr::eq(definition, read)
            || definition.definition_span != self.declaration
            || definition.references.as_slice() != [self.reference]
        {
            return None;
        }
        let unresolved = &self.obligations;
        if unresolved.observer_horizon != self.image
            || unresolved.moved_rhs != self.rhs
            || unresolved.substituted_read != self.replacement
            || !self.action.edits.iter().any(|edit| {
                edit.new_text.is_empty()
                    && edit.start <= unresolved.erased_statement.start()
                    && edit.end >= unresolved.erased_statement.end()
            })
        {
            return None;
        }
        // Consumption is ONLY by this conditional source-authoring sink.
        Some(self.action)
    }
}

/// Reborrow the actual script and substitution chain, rather than parsing an
/// unfinished prefix ending inside a bracket. This is source ordering only;
/// it does not discharge the recipe's retained runtime motion obligations.
fn source_read_follows_setter(
    source: &str,
    analysis: &AnalysisResult,
    walk: &super::FrameWalk<'_>,
    setter: &SegmentedCommand,
    read: &SegmentedCommand,
) -> Option<()> {
    let coverage = crate::SourceSyntaxCoverage::capture(source, analysis)?;
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config?;
    let setter_words = walk.native_words(source, setter)?;
    let read_words = walk.native_words(source, read)?;
    let setter_start = setter_words.first()?.span().start();
    let read_end = read_words.last()?.span().end();
    let mut regions = coverage.script_regions(source, analysis)?.to_vec();
    regions.sort_by_key(|(region, _)| region.end() - region.start());
    let region = regions.into_iter().find_map(|(region, _)| {
        if region.start() > setter_start || region.end() < read_end {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            region,
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        plan.commands
            .iter()
            .any(|command| command.words == setter_words)
            .then_some(plan)
    })?;
    let setter_index = region
        .commands
        .iter()
        .position(|command| command.words == setter_words)?;
    let next = region.commands.get(setter_index.checked_add(1)?)?;
    let mut words = next.words.clone();
    let mut plan = region;
    loop {
        if words == read_words {
            return Some(());
        }
        // Every outer invocation must reach this exact child through static
        // earlier operands. An earlier substitution or variable read is not
        // silently crossed by descending to the inner read.
        let read_start = read_words.first()?.span().start();
        let (index, word) = words
            .iter()
            .enumerate()
            .find(|(_, word)| word.span().start() <= read_start && read_end <= word.span().end())?;
        if words[..index].iter().any(|word| {
            word.group().expand
                || tcl_syntax::word_rules::original_static_word_unicode_value(word).is_none()
        }) || word.group().expand
        {
            return None;
        }
        let mut child = None;
        for part in word.executable_parts().all_parts() {
            match part.part {
                tcl_lexer::ExecutablePart::Command { body }
                    if body.start() <= read_start && read_end <= body.end() =>
                {
                    child = Some(body);
                    break;
                }
                tcl_lexer::ExecutablePart::Text(_) => {}
                _ => return None,
            }
        }
        let child = plan.reborrow_command_substitution_plan(word, child?).ok()?;
        // The inner read cannot borrow a later command after an intervening
        // statement in this substitution. Repeated descent retains each owner.
        words.clone_from(&child.commands.first()?.words);
        plan = child;
    }
}

/// Logical source edits use selected Set structure, independently of a
/// Native store or Normal receipt. Captured data has no current word to edit.
fn logical_set_operands(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
) -> Option<(String, usize)> {
    let walk = super::FrameWalk::new(source, analysis)?;
    let words = walk.source_words(source, command)?;
    if words.arguments().len() != 2
        || words.with_source_schema(&walk.source_context(), |schema| {
            schema.semantics.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::Set)
        }) != Some(true)
        || !words
            .roles()?
            .contains(&(0, tcl_registry::ArgRole::VarWrite))
    {
        return None;
    }
    let name = std::str::from_utf8(words.arguments()[0].literal_bytes()?)
        .ok()?
        .to_owned();
    let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(value) =
        words.origins().get(2)?
    else {
        return None;
    };
    command.argv.get(*value)?;
    Some((name, *value))
}

/// Source selection and rendering data, without a runtime name lookup grant.
struct OriginalInlineContext<'a> {
    image: tcl_lexer::SourceImage,
    config: LexerConfig,
    registry: &'a CommandRegistry,
    metadata: Option<tcl_compiler::registry_invocation::InvocationMetadataContext<'a>>,
    original: tcl_compiler::ir::CommandTokens,
    offset: u32,
}

type OriginalInlineBinding = (String, tcl_lexer::Span, usize);

/// A literal setter and its single physical value read. Name geometry alone
/// cannot establish store preservation, observers or reaching contents.
fn original_inline_binding(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
) -> Option<OriginalInlineBinding> {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    let input = analysis.resolved_input.as_ref()?;
    let mut context = OriginalInlineContext {
        image: tcl_lexer::SourceImage::document(source),
        config: analysis.body_lexer_config?,
        registry: analysis.resolved_registry()?,
        metadata: None,
        original: tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            analysis.body_lexer_config?,
            command,
        ),
        offset: command.span.start(),
    };
    let unit = CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry: context.registry,
            config: context.config,
            dialect: Some(input.unit_profile()),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        input,
    );
    let mut selected = None;
    for function in unit.analysable_body_function_units() {
        context.metadata =
            function.invocation_metadata_context_for_module(context.registry, &unit.ir_module);
        let opaque = function.cfg.has_opaque_native_accesses();
        let barrier = function.dynamic_barrier_blocks_value_motion();
        if opaque || barrier {
            continue;
        }
        for (&block, body) in &function.ssa.blocks {
            for index in 0..body.statements.len() {
                if let Some(candidate) = point_inline_binding(&context, function, block, index) {
                    if selected.is_some() {
                        return None;
                    }
                    selected = Some(candidate);
                }
            }
        }
    }
    selected
}

fn point_inline_binding(
    context: &OriginalInlineContext<'_>,
    function: &tcl_compiler::compilation_unit::FunctionUnit,
    block: tcl_compiler::cfg::BlockId,
    index: usize,
) -> Option<OriginalInlineBinding> {
    use tcl_compiler::ir::{WordExpr, WordPart};
    use tcl_compiler::ssa::SsaSourceView;
    let view = SsaSourceView::at_statement(&function.ssa, block, index);
    let tokens = view.source_tokens()?;
    if function
        .abs_span(tokens.words().first()?.source().span)
        .start()
        != context.offset
    {
        return None;
    }
    let normal =
        tcl_compiler::registry_invocation::normal_transfer_invocation_with_metadata_context(
            context.registry,
            Some(context.metadata?),
            tokens,
        );
    let normal = normal?;
    let value_index = normal
        .written_argument(normal.stored_value_argument()?)?
        .checked_add(1)?;
    let value = context.original.words().get(value_index)?;
    if !matches!(
        value,
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
    ) && !matches!(value, WordExpr::Template { parts, .. } if parts.iter().all(|part| matches!(part, WordPart::Text { .. })))
    {
        return None;
    }
    normal.stored_value_word(
        &tokens.source_binding.as_ref()?.variable_context,
        context.registry,
    )?;
    let statement = function.ssa.blocks.get(&block)?.statements.get(index)?;
    let mut selected = None;
    for (&symbol, &version) in &statement.defs {
        if let Some((label, read_span)) =
            value_read_binding(context, function, view, block, index, symbol, version)
        {
            if selected.is_some() {
                return None;
            }
            selected = Some((label, read_span, value_index));
        }
    }
    selected
}

fn value_read_binding(
    context: &OriginalInlineContext<'_>,
    function: &tcl_compiler::compilation_unit::FunctionUnit,
    view: tcl_compiler::ssa::SsaSourceView<'_>,
    block: tcl_compiler::cfg::BlockId,
    index: usize,
    symbol: tcl_compiler::ssa::Symbol,
    version: tcl_compiler::ssa::Version,
) -> Option<(String, tcl_lexer::Span)> {
    use tcl_compiler::def_use::{DefKind, UseKind};
    use tcl_compiler::ssa::SsaSourceView;
    let definition = view.original_definition_name_with_metadata_context(
        symbol,
        context.registry,
        context.metadata,
    )?;
    let input = definition.original_name_input();
    let key = input.original_word_key()?;
    if key.source_image() != &context.image
        || key.lexer_config() != context.config
        || !view.normal_store_contents_preserved(symbol, context.registry)
    {
        return None;
    }
    let chain = function
        .def_use
        .chain_for_cell(&function.ssa, definition.cell(), version)?;
    let [use_site] = chain.uses.as_slice() else {
        return None;
    };
    if chain.definition.kind != DefKind::Statement
        || use_site.kind != UseKind::Operand
        || use_site.block != chain.definition.block
    {
        return None;
    }
    let use_index = usize::try_from(use_site.statement_index)
        .ok()
        .filter(|use_index| *use_index > index)?;
    let use_view = SsaSourceView::at_statement(&function.ssa, block, use_index);
    let use_tokens = use_view.source_tokens()?;
    let mut reads = use_tokens.variable_accesses.iter().filter(|access| {
        use_view
            .replaceable_read_at(&access.source, &access.original_spelling, context.registry)
            .is_some_and(|reference| {
                reference.symbol == symbol && reference.version == Some(version)
            })
    });
    let read = reads.next()?;
    if reads.next().is_some() {
        return None;
    }
    let label = tcl_syntax::native_string::resident_name_label(input.bytes());
    Some((label, function.abs_span(read.source.span)))
}

/// Exact selected variable component and its unchanged containing word.
struct RefContext {
    reference: Span,
    word_is_single: bool,
    word: tcl_lexer::NativeWord,
}

/// Reborrow the single original reference and containing whole word. Child
/// commands come from the shared source structure; token ends cannot supply
/// a closer or substitute an enclosing array reference for an index child.
fn reference_context(
    source: &str,
    ref_off: u32,
    name: &str,
    analysis: &AnalysisResult,
    walk: &super::FrameWalk<'_>,
) -> Option<RefContext> {
    let command = super::find_original_command_at(source, ref_off, analysis)?;
    let references = walk.variable_references(source, &command)?;
    let mut matching = references.iter().filter(|reference| {
        reference.name == name
            && reference.span.start() <= ref_off
            && ref_off < reference.span.end()
    });
    let reference = matching.next()?;
    if matching.next().is_some() {
        return None;
    }
    let words = walk.native_words(source, &command)?;
    let mut containers = words.iter().enumerate().filter(|(_, word)| {
        word.span().start() <= reference.span.start() && reference.span.end() <= word.span().end()
    });
    let (index, word) = containers.next()?;
    if containers.next().is_some() {
        return None;
    }
    // An index child needs its own insertion grammar, beyond a scalar-word
    // splice. Its enclosing array component cannot donate that permission.
    if word.executable_parts().all_parts().any(|part| {
        matches!(
            part.part,
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
        ) && part.span != reference.span
            && part.span.start() <= reference.span.start()
            && reference.span.end() <= part.span.end()
    }) {
        return None;
    }
    Some(RefContext {
        reference: reference.span,
        word_is_single: command.single_token_word.get(index).copied()?
            && word.span() == reference.span,
        word: word.clone(),
    })
}

/// Propose a fragment from the actual value word, then check the complete
/// enclosing word under its own configuration. Component equality is lexical;
/// the caller retains the independent binding, read and movement permissions.
fn interpolated_value_source(
    value: &tcl_lexer::NativeWord,
    receiver: &tcl_lexer::NativeWord,
    reference: Span,
) -> Option<String> {
    use tcl_lexer::{WordKind, WordPart};
    if value.group().expand || receiver.group().expand {
        return None;
    }
    let content = value.content_span().ok()?;
    let content = value.image().try_text().ok()?.get(content.as_range())?;
    let fragment = if value.group().kind == WordKind::Braced {
        // Literal content may not become substitutions in its new context.
        // The whole-word comparison below detects that activation.
        content.to_owned()
    } else {
        tcl_syntax::backslash::substituting_quoted_source_fragment(content, value.config())?
    };
    let replacement =
        if let Some(bytes) = tcl_syntax::word_rules::original_static_word_unicode_value(value) {
            vec![WordPart::Text(std::borrow::Cow::Owned(bytes))]
        } else {
            value
                .parts()
                .ok()?
                .into_iter()
                .map(|part| part.part)
                .collect()
        };
    let mut expected = Vec::new();
    let mut found = false;
    for part in receiver.parts().ok()? {
        if part.start == reference.start() as usize && part.end == reference.end() as usize {
            if found
                || !matches!(&part.part, WordPart::Variable(variable) if variable.index.is_none())
            {
                return None;
            }
            found = true;
            expected.extend(replacement.iter().cloned());
        } else {
            expected.push(part.part);
        }
    }
    if !found {
        return None;
    }
    let original = std::str::from_utf8(receiver.written_bytes()).ok()?;
    let from = usize::try_from(reference.start().checked_sub(receiver.span().start())?).ok()?;
    let to = usize::try_from(reference.end().checked_sub(receiver.span().start())?).ok()?;
    let mut proposal = original.to_owned();
    proposal.get(from..to)?;
    proposal.replace_range(from..to, &fragment);
    let parsed = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(&proposal),
        Span::new(0, u32::try_from(proposal.len()).ok()?),
        receiver.config(),
    )
    .ok()?;
    let [command] = parsed.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    if parsed.fatal_tail.is_some()
        || word.group().expand
        || word.written_bytes() != proposal.as_bytes()
    {
        return None;
    }
    let actual = word.parts().ok()?.into_iter().map(|part| part.part);
    (coalesced_source_parts(expected)? == coalesced_source_parts(actual)?).then_some(fragment)
}

/// Literal runs may join across a replaced substitution, without changing any
/// variable, index, command or expression component.
fn coalesced_source_parts<'a>(
    parts: impl IntoIterator<Item = tcl_lexer::WordPart<'a>>,
) -> Option<Vec<tcl_lexer::WordPart<'a>>> {
    use tcl_lexer::WordPart;
    let mut result = Vec::new();
    for part in parts {
        match part {
            WordPart::ParseError(_) => return None,
            WordPart::Text(bytes) => {
                if let Some(WordPart::Text(prior)) = result.last_mut() {
                    prior.to_mut().extend_from_slice(&bytes);
                } else if !bytes.is_empty() {
                    result.push(WordPart::Text(bytes));
                }
            }
            other => result.push(other),
        }
    }
    Some(result)
}

/// Yield every `VarDef` in `scope` and its descendants.
#[cfg(test)]
fn walk_scopes(scope: &Scope) -> Vec<&VarDef> {
    walk_scopes_at_depth(scope, 0)
}

#[cfg(test)]
fn walk_scopes_at_depth(scope: &Scope, depth: u32) -> Vec<&VarDef> {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return Vec::new();
    }
    let mut out: Vec<&VarDef> = scope.variables.values().collect();
    for child in &scope.children {
        out.extend(walk_scopes_at_depth(child, depth + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use tcl_compiler::analyser::Analyser;

    fn run(source: &str, cursor: u32) -> Option<String> {
        let reg = super::super::test_registry();
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let li = LineIndex::new(source);
        inline_variable(source, cursor, &analysis, &reg, &li).map(|r| r.apply(source))
    }

    #[test]
    fn logical_inline_set_uses_selected_operation_and_preserves_capture_geometry() {
        // naming.refactor.original-variable-inline-permission

        let mut registry = CommandRegistry::build_default();
        let mut setter = registry.get("set").unwrap().clone();
        setter.name = "selected_write";
        setter.surface = Some(tcl_dialect::model::SpecSurface::TCL90_PLUS);
        registry.insert(setter);
        let registry = std::sync::Arc::new(registry);
        let source = "interp alias {} write {} selected_write held\nwrite VALUE\nputs $held";
        let analysis = super::super::test_logical_analysis(source, "tcl9.0", registry.clone());
        let cursor = u32::try_from(source.find("write VALUE").unwrap()).unwrap();
        let command = super::super::find_original_command_at(source, cursor, &analysis).unwrap();
        assert_eq!(
            logical_set_operands(source, &analysis, &command),
            Some(("held".into(), 1))
        );
        let older = super::super::test_logical_analysis(source, "tcl8.4", registry.clone());
        assert!(logical_set_operands(source, &older, &command).is_none());
        let captured = "interp alias {} write {} selected_write held VALUE\nwrite\nputs $held";
        let analysis = super::super::test_logical_analysis(captured, "tcl9.0", registry);
        let cursor = u32::try_from(captured.find("\nwrite\n").unwrap() + 1).unwrap();
        let command = super::super::find_original_command_at(captured, cursor, &analysis).unwrap();
        assert!(logical_set_operands(captured, &analysis, &command).is_none());
        let replaced = "proc set args {}; set held VALUE\nputs $held";
        let analysis = Analyser::new().analyse(replaced, "tcl");
        let cursor = u32::try_from(replaced.find("set held").unwrap()).unwrap();
        let command = super::super::find_original_command_at(replaced, cursor, &analysis).unwrap();
        assert!(logical_set_operands(replaced, &analysis, &command).is_none());
    }

    #[test]
    fn logical_inline_keeps_original_value_and_reference_component_extents() {
        for (source, expected) in [
            ("set held ${other}\nputs ${held}", Some("puts ${other}")),
            (
                "set held VALUE\nputs pre-${held}-post",
                Some("puts pre-VALUE-post"),
            ),
            ("set held VALUE\nputs $array($held)", None),
            ("set held {literal $inert}\nputs \"$held\"", None),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let registry = analysis.resolved_registry().unwrap();
            let index = LineIndex::new(source);
            let action = inline_variable(source, 0, &analysis, registry, &index);
            if let Some(expected) = expected {
                assert_eq!(action.unwrap().apply(source), expected);
            } else {
                assert!(action.is_none());
            }
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(inline_variable(source, 0, &missing, registry, &index).is_none());
        }
    }

    #[test]
    fn logical_inline_edits_require_current_source_and_complete_metadata_owner() {
        // naming.core.original-dispatch-region-context

        let source = "set held VALUE\nputs $held";
        let analysis = Analyser::new().analyse(source, "tcl");
        let registry = analysis.resolved_registry().unwrap();
        let index = LineIndex::new(source);
        assert_eq!(
            inline_variable(source, 0, &analysis, registry, &index)
                .unwrap()
                .apply(source),
            "puts VALUE"
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().strict_quoting =
            !analysis.body_lexer_config.unwrap().strict_quoting;
        let mut foreign = analysis.clone();
        let input = analysis.resolved_input.as_ref().unwrap();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        ));
        for unavailable in [missing, stale, foreign] {
            assert!(inline_variable(source, 0, &unavailable, registry, &index).is_none());
        }
        assert!(
            inline_variable("set held OTHER\nputs $held", 0, &analysis, registry, &index).is_none()
        );
    }

    /// `walk_scopes_at_depth` recurses once per nested namespace / proc
    /// scope, and `MAX_SCOPE_WALK_DEPTH` (`crate::lib`) is what stops that
    /// recursion from overflowing `cargo test`'s bare ~2 MiB per-test
    /// stack: unguarded, it goes over at around 100 levels. The assertion
    /// is that 80 levels return at all, not what they return.
    #[test]
    fn deeply_nested_namespaces_survive_scope_walk() {
        // Implementation contract: naming.source.recursive-driver-state-transport

        const DEPTH: usize = 80;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("set x 1\nputs $x\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        let analysis = Analyser::new().analyse(&source, "tcl8.6").clone();
        let _ = walk_scopes(&analysis.global_scope);
    }

    #[test]
    fn inline_single_use() {
        let source = "set name \"hello\"\nputs $name";
        let applied = run(source, 0).expect("result");
        assert!(!applied.contains("set name"), "{applied:?}");
        assert!(applied.contains("\"hello\""), "{applied:?}");
    }

    #[test]
    fn original_inline_variable_requires_exact_definition_and_unobserved_read_without_ui_maps() {
        // Implementation contract: naming.refactor.original-variable-inline-permission

        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let source = "set café 7\nputs ${café}";
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.global_scope.variables.clear();
            let registry = analysis.resolved_registry().unwrap();
            let index = LineIndex::new(source);
            let change = inline_variable(source, 0, &analysis, registry, &index).expect(dialect);
            assert_eq!(change.apply(source), "puts 7", "{dialect}");
            assert!(inline_variable("# stale source", 0, &analysis, registry, &index).is_none());
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(inline_variable(source, 0, &missing, registry, &index).is_none());
        }
    }

    #[test]
    fn original_inline_variable_rejects_reassigned_cells_observers_and_value_effect_movement() {
        // Implementation contract: naming.refactor.original-variable-inline-permission

        for source in [
            "set x 1; set x 2; puts $x",
            "set x [incr counter]; puts $x",
            "set x 1; unset x; set x 2; puts $x",
            "proc observer {args} {puts observed}; trace variable x r observer; set x 1; puts $x",
            "set x 1; upvar #0 x alias; set alias 2; puts $x",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let cursor = u32::try_from(source.find("set x").unwrap()).unwrap();
            assert!(
                inline_variable(
                    source,
                    cursor,
                    &analysis,
                    analysis.resolved_registry().unwrap(),
                    &LineIndex::new(source)
                )
                .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn inline_braced_reference() {
        let source = "set x 1\nputs ${x}";
        assert_eq!(run(source, 0).as_deref(), Some("puts 1"));
    }

    #[test]
    fn does_not_inline_braced_literal_with_subst_into_quotes() {
        // `set x {a $b}` holds the LITERAL `a $b`. Inlining it
        // into `"v: $x"` would turn `$b` into a live substitution, so the
        // refactor must decline rather than corrupt the value.
        let source = "set x {a $b}\nputs \"v: $x\"";
        assert_eq!(run(source, 0), None);
        // A `[cmd]` in a braced literal is equally unsafe.
        let source2 = "set x {a [cmd]}\nputs \"v: $x\"";
        assert_eq!(run(source2, 0), None);
        // FP-guard: a braced literal WITHOUT substitution chars still inlines.
        let source3 = "set x {a b}\nputs \"v: $x\"";
        assert_eq!(run(source3, 0).as_deref(), Some("puts \"v: a b\""));
    }

    #[test]
    fn inline_preserves_following_line_without_trailing_newline() {
        let source = "set x 1\nset y $x";
        assert_eq!(run(source, 0).as_deref(), Some("set y 1"));
    }

    #[test]
    fn inline_into_command_substitution() {
        // The only use sits inside `[…]`, so the reference resolves through
        // the substitution's own command.
        let source = "set timeout 30\nset result [list URL -timeout $timeout]";
        assert_eq!(
            run(source, 0).as_deref(),
            Some("set result [list URL -timeout 30]")
        );
        let unproved = "set timeout 30\nset result [http::geturl $url -timeout $timeout]";
        assert!(run(unproved, 0).is_none());
    }

    #[test]
    fn inline_into_nested_command_substitution() {
        let source = "set n 2\nputs [format %d [string repeat x $n]]";
        assert_eq!(
            run(source, 0).as_deref(),
            Some("puts [format %d [string repeat x 2]]")
        );
    }

    #[test]
    fn no_inline_of_a_reference_inside_a_braced_word() {
        // `expr {$n + 1}` holds `$n` in a braced literal, which `expr`
        // substitutes itself; the segmenter has no VAR token there, so
        // there is no reference span to rewrite.
        let source = "set n 2\nputs [expr {$n + 1}]";
        assert_eq!(run(source, 0), None);
    }

    #[test]
    fn inline_keeps_the_following_line_indentation() {
        // Deleting the `set` line must take its indentation with it.
        let source = "proc p {} {\n    set timeout 30\n    puts $timeout\n}\np";
        assert_eq!(
            run(source, 16).as_deref(),
            Some("proc p {} {\n    puts 30\n}\np")
        );
    }

    #[test]
    fn no_inline_multiple_uses() {
        assert!(run("set x 42\nputs $x\nputs $x", 0).is_none());
    }

    #[test]
    fn no_inline_read_form() {
        assert!(run("set x", 0).is_none());
    }

    #[test]
    fn no_inline_non_set() {
        assert!(run("puts \"hello\"", 0).is_none());
    }
}

#[cfg(test)]
mod selected_declaration_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn logical_inline_selects_the_exact_name_operand_and_same_scope_reference() {
        let source = "proc first {} {set held FIRST; puts $held}; proc second {} {set held SECOND; puts $held}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let registry = analysis.resolved_registry().unwrap();
        let cursor = u32::try_from(source.find("set held SECOND").unwrap()).unwrap();
        let action = inline_variable(source, cursor, &analysis, registry, &LineIndex::new(source))
            .expect("the declaration in the selected procedure");
        assert_eq!(
            action.apply(source),
            "proc first {} {set held FIRST; puts $held}; proc second {} {; puts SECOND}"
        );

        let repeated = "set held FIRST; set held SECOND; puts $held";
        let analysis = Analyser::new().analyse(repeated, "tcl");
        let cursor = u32::try_from(repeated.find("set held SECOND").unwrap()).unwrap();
        assert!(
            inline_variable(
                repeated,
                cursor,
                &analysis,
                analysis.resolved_registry().unwrap(),
                &LineIndex::new(repeated)
            )
            .is_none()
        );

        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            inline_variable(
                repeated,
                0,
                &missing,
                analysis.resolved_registry().unwrap(),
                &LineIndex::new(repeated)
            )
            .is_none()
        );
        assert!(
            inline_variable(
                "# changed",
                0,
                &analysis,
                analysis.resolved_registry().unwrap(),
                &LineIndex::new("# changed")
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod original_interpolation_source_tests {
    use super::*;

    fn word(source: &str, config: LexerConfig) -> tcl_lexer::NativeWord {
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert!(parsed.fatal_tail.is_none());
        let [command] = parsed.commands.as_slice() else {
            panic!("one original command required");
        };
        let [word] = command.words.as_slice() else {
            panic!("one original word required");
        };
        assert_eq!(word.written_bytes(), source.as_bytes());
        word.clone()
    }

    #[test]
    fn original_inline_fragments_keep_components_and_complete_receiver_words() {
        for dialect in ["tcl8.6", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            for (value, receiver, expected) in [
                (r#""a\"b""#, r#""v:$held""#, Some(r#"a\"b"#)),
                ("{a b}", r#""v:$held""#, Some("a b")),
                ("{a $b}", r#""v:$held""#, None),
                ("{a [cmd]}", r#""v:$held""#, None),
                ("{a b}", "prefix${held}", None),
                ("$other", r#""v:${held}""#, Some("$other")),
                ("$other", r#""${held}suffix""#, None),
                ("${other}", r#""${held}suffix""#, Some("${other}")),
            ] {
                let value = word(value, config);
                let receiver = word(receiver, config);
                let parts = receiver.parts().unwrap();
                let reference = parts
                    .iter()
                    .find(|part| matches!(&part.part, tcl_lexer::WordPart::Variable(variable) if variable.name == b"held" && variable.index.is_none()))
                    .unwrap();
                let reference = Span::new(
                    u32::try_from(reference.start).unwrap(),
                    u32::try_from(reference.end).unwrap(),
                );
                assert_eq!(
                    interpolated_value_source(&value, &receiver, reference).as_deref(),
                    expected,
                    "{dialect}"
                );
            }
        }
    }

    #[test]
    fn logical_inline_quote_proposals_keep_the_current_binding_and_source_gates() {
        let source = "set held \"a\\\"b\"\nputs \"v:$held\"";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        let registry = analysis.resolved_registry().unwrap();
        let lines = LineIndex::new(source);
        assert_eq!(
            inline_variable(source, 0, &analysis, registry, &lines)
                .unwrap()
                .apply(source),
            "puts \"v:a\\\"b\""
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(inline_variable(source, 0, &missing, registry, &lines).is_none());
        assert!(
            inline_variable(
                "set held CHANGED\nputs \"v:$held\"",
                0,
                &analysis,
                registry,
                &lines
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod logical_inline_source_intent_tests {
    use super::*;

    fn authored(source: &str) -> Option<Refactoring> {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        inline_variable(
            source,
            0,
            &analysis,
            analysis.resolved_registry()?,
            &LineIndex::new(source),
        )
    }

    #[test]
    fn conditional_intent_retains_original_rhs_and_complete_postimage() {
        let source = "set held ${other}\nputs ${held}";
        assert_eq!(authored(source).unwrap().apply(source), "puts ${other}");
        let source = "set held VALUE\nputs pre-${held}-post";
        assert_eq!(
            authored(source).unwrap().apply(source),
            "puts pre-VALUE-post"
        );
    }

    #[test]
    fn source_intent_refuses_future_reflection_and_unknown_history() {
        assert!(authored("set held VALUE\nputs $held\ninfo locals").is_none());
        assert!(authored("set held VALUE\nputs $held\nunknown_observer").is_none());
    }

    #[test]
    fn source_intent_does_not_cross_statements_or_earlier_dynamic_operands() {
        assert!(authored("set held ${other}\nset other CHANGED\nputs $held").is_none());
        assert!(authored("set held ${other}\nputs [set other CHANGED] $held").is_none());
    }
}

#[cfg(test)]
mod logical_inline_substitution_ancestry_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn conditional_inline_reborrows_nested_substitution_ancestry() {
        for source in [
            "set n 2\nputs [format %d [string repeat x $n]]",
            "proc p {} {set n 2; puts [format %d [string repeat x $n]]}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let cursor = u32::try_from(source.find("set n").unwrap()).unwrap();
            let action = inline_variable(
                source,
                cursor,
                &analysis,
                analysis.resolved_registry().unwrap(),
                &LineIndex::new(source),
            )
            .expect("conditional source intent retains genuine nested ancestry");
            assert!(action.apply(source).contains("[string repeat x 2]"));
        }
    }

    #[test]
    fn substitution_ancestry_cannot_cross_prior_commands_or_dynamic_operands() {
        for source in [
            "set n 2; puts [puts before; list $n]",
            "set n 2; puts $other [list $n]",
            "set n 2; puts [list $other] [list $n]",
            "set n 2; puts [format %d [puts before; list $n]]",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            assert!(
                inline_variable(
                    source,
                    0,
                    &analysis,
                    analysis.resolved_registry().unwrap(),
                    &LineIndex::new(source)
                )
                .is_none(),
                "{source}"
            );
        }
    }
}
