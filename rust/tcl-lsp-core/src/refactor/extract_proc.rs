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

//! Extract into proc — move a run of commands into a new proc and call it.
//!
//! # The problem a line-based extraction cannot solve
//!
//! A `proc` gets its own variable frame.  Anything the moved code *assigns*
//! therefore stops being the caller's variable and becomes a private local
//! that disappears when the proc returns.  Extracting the middle two lines of
//!
//! ```tcl
//! set x 0
//! set x 1
//! puts $x
//! puts "after=$x"
//! ```
//!
//! into `proc extracted_proc {x} { set x 1; puts $x }` prints `after=0`, not
//! `after=1`: the write moved into the proc's own `x`.
//!
//! Passing the variable in as a parameter is exactly what causes this.  A
//! parameter is a *copy*; the caller never sees it change.
//!
//! # What this transform does instead
//!
//! It classifies every variable the selection touches, using the registry's
//! own [`ArgRole::VarWrite`] / [`ArgRole::VarRead`] / [`ArgRole::LoopVarList`]
//! roles plus the `$name` references in the text — never a keyword list.  The
//! classification runs over the selection's whole **statement tree**, not just
//! its top-level commands: `foreach n {1 2 3} {set total …}` writes `total`
//! from inside the loop body, and a top-level-only reading of it hands back a
//! proc that silently drops the assignment.  Which nested words are part of
//! that tree is [`crate::references::nested_dispatch_regions`]'s answer — the
//! same-frame walker Find-References uses — so a `proc` body or an
//! `apply` lambda inside the selection, which opens a frame of its own, stays
//! out of it.  The three outcomes are:
//!
//! * **read, never written** → an ordinary value parameter.  A copy is
//!   correct here, because nothing writes it back.
//! * **written, and read again after the selection** → passed *by name* and
//!   re-bound in the proc with `upvar 1`, so the assignment lands in the
//!   caller's frame exactly as it did before.
//! * **written, and never read again** → a genuine local.  It becomes a proc
//!   local, which is better than the status quo: the variable no longer
//!   leaks into the caller at all.
//!
//! The generated proc is placed immediately **before** the enclosing
//! top-level command rather than at line 0, so it lands after the file's
//! `package require` / `namespace` prologue, and its name is made unique
//! against the workspace's symbols.
//!
//! # What it refuses, and why
//!
//! | Refused | Because |
//! |---|---|
//! | The selection does not cover whole commands | Half a command is not a program; splicing the remainder into a call changes the parse. |
//! | The selection contains a frame-sensitive command (`return`, `break`, `continue`, `upvar`, `uplevel`, `global`, `variable`, `info level`, …) | Those act on the *call frame*. `return` would return from the new proc instead of the caller; `break` would escape a loop that is no longer around it; `upvar 1` would alias one frame too far. Membership is the registry's own frame-sensitivity union. |
//! | A written variable's name is computed (`set $n 1`) or is an array element (`set a(x) 1`) | Which variable is written is a run-time fact, and an array element is not a place a scalar `upvar` protocol can carry. |
//! | Any command head in the selection is computed (`$cmd …`, `{*}$words`) | Its argument roles — and therefore which variables it reads and writes — are unknown. |
//! | The selection sits inside a namespace or class definition body | The extracted proc would be created in a different namespace from the one the moved code ran in, silently changing what its unqualified calls and variables resolve to. |
//!
//! Every refusal carries a plain-English reason, surfaced as the code
//! action's LSP `disabled.reason` so the editor greys the entry out and
//! explains itself.

use std::collections::{BTreeMap, BTreeSet};

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_registry::{ArgRole, CommandRegistry, Traits};

#[cfg(test)]
use tcl_dialect::BracedVarStyle;
use tcl_lexer::LexerConfig;

use super::source_rewrite::script_word_spellings;
use super::{RefactorEdit, Refactoring};
use crate::code_actions::{ActionCommand, ActionKind};

/// The base name the generated proc gets before collision-avoidance.
const BASE_NAME: &str = "extracted_proc";

/// How far the collision-avoidance suffix search runs — see
/// [`unique_proc_name`].
const MAX_NAME_SUFFIX: u32 = 1000;

/// Extract the commands covered by `selection` into a new proc.
///
/// Returns `None` when the selection covers no command at all — there is
/// nothing to offer.  Returns a [`Refactoring`] with `disabled` set when a
/// run of commands *is* selected but cannot be extracted without changing
/// behaviour.
#[must_use]
pub fn extract_proc(
    source: &str,
    selection: (u32, u32),
    analysis: &AnalysisResult,
    _registry: &CommandRegistry,
) -> Option<Refactoring> {
    let (sel_start, sel_end) = selection;
    if sel_end <= sel_start {
        return None;
    }
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let config = current.config();
    let registry = current.registry();
    let walk = super::FrameWalk::new(source, analysis)?;
    let scope = original_enclosing_scope(source, sel_start, &walk)?;
    let scope_text = source.get(scope.start as usize..scope.end as usize)?;
    let commands: Vec<SegmentedCommand> =
        segment_commands_with_offset_and_config(scope_text, scope.start, config)
            .into_iter()
            .filter(|command| !command.name().is_empty())
            .collect();
    let extents = commands
        .iter()
        .map(|command| walk.command_span_offsets(source, command))
        .collect::<Option<Vec<_>>>()?;
    let selected: Vec<&SegmentedCommand> = commands
        .iter()
        .zip(extents)
        .filter_map(|(command, (start, end))| {
            (start >= sel_start && end <= sel_end).then_some(command)
        })
        .collect();
    if selected.is_empty() {
        return None;
    }
    let title = "Extract selection into proc".to_string();
    match plan_extraction(
        source, &selected, &commands, scope, analysis, registry, config,
    ) {
        Ok(plan) => {
            let proposal = Refactoring {
                title,
                edits: plan.edits,
                kind: ActionKind::RefactorExtract,
                data_group: None,
                disabled: None,
            };
            check_proc_extraction(source, &proposal, &plan.name, config)?;
            Some(proposal)
        }
        Err(reason) => Some(Refactoring {
            title,
            edits: Vec::new(),
            kind: ActionKind::RefactorExtract,
            data_group: None,
            disabled: Some(reason),
        }),
    }
}

fn check_proc_extraction(
    source: &str,
    proposal: &Refactoring,
    name: &str,
    config: LexerConfig,
) -> Option<()> {
    let [definition, call] = proposal.edits.as_slice() else {
        return None;
    };
    let after = proposal.try_apply(source).ok()?;
    let added = definition.new_text.len();
    let start = definition.start as usize;
    let declaration = after.get(start..start.checked_add(added)?)?;
    let words = script_word_spellings(declaration, config)?;
    let [header] = words.as_slice() else {
        return None;
    };
    let ["proc", declared, _, _] = header.as_slice() else {
        return None;
    };
    let start = (call.start as usize).checked_add(added)?;
    let invocation = after.get(start..start.checked_add(call.new_text.len())?)?;
    let words = script_word_spellings(invocation, config.nested())?;
    let [command] = words.as_slice() else {
        return None;
    };
    (*declared == name && command.first() == Some(declared)).then_some(())
}

/// Source-bound generated Name continuation under the actual original input.
/// The afterimage is independently parsed by the existing declaration owners;
/// a reporting name or raw substring cannot supply the target. Failure omits
/// only the optional rename UI and grants no source-action/edit permission.
#[must_use]
pub fn extract_proc_rename_command_for_analysis(
    refactoring: &Refactoring,
    source: &str,
    analysis: &AnalysisResult,
) -> Option<ActionCommand> {
    crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    if refactoring.disabled.is_some() {
        return None;
    }
    let insertion = refactoring
        .edits
        .iter()
        .filter(|edit| edit.start == edit.end)
        .min_by_key(|edit| edit.start)?;
    if refactoring
        .edits
        .iter()
        .any(|edit| edit.start < insertion.start)
    {
        return None;
    }
    let after = refactoring.try_apply(source).ok()?;
    let mut analyser = tcl_compiler::analyser::Analyser::new().with_resolved_input(input.clone());
    let generated = analyser.analyse(&after, "tcl");
    crate::original_context::CurrentSourceContext::capture(&after, &generated)?;
    let mut words = Vec::new();
    if input.has_logical_source_name_context() {
        for header in generated.logical_source_procedure_declarations() {
            if header.original_definition_site()?.offset == insertion.start {
                words.push(header.original_name_word()?.clone());
            }
        }
    } else {
        for header in generated.original_procedure_declarations() {
            if header.declaration_site().offset == insertion.start {
                words.push(header.name_input().original_word().clone());
            }
        }
    }
    let word = words.first()?;
    (words.len() == 1
        && word.image() == &tcl_lexer::SourceImage::document(&after)
        && word.config() == input.lexer_config())
    .then_some(())?;
    let token = after.get(word.span().as_range())?.to_owned();
    let lines = tcl_lexer::LineIndex::new_lsp(&after);
    let start = lines.position_at_utf16(word.span().start(), &after);
    let end = lines.position_at_utf16(word.span().end(), &after);
    (start.line == end.line).then_some(())?;
    Some(ActionCommand {
        command: "tclLsp.renameSymbolAtPosition".to_owned(),
        args: vec![start.line, start.character.get(), end.character.get()],
        string_args: vec![after, token],
    })
}

/// The byte range of the innermost script region containing `offset`.
#[derive(Clone, Copy)]
struct Scope {
    start: u32,
    end: u32,
}

/// A successful extraction: the definition insertion plus the call that
/// replaces the selection.
struct Plan {
    name: String,
    edits: Vec<RefactorEdit>,
}

/// Original frame geometry only; an unknown region cannot borrow lexical
/// command spelling to choose its enclosing execution scope.
fn original_enclosing_scope(
    source: &str,
    offset: u32,
    walk: &super::FrameWalk<'_>,
) -> Option<Scope> {
    let mut scope = Scope {
        start: 0,
        end: u32::try_from(source.len()).ok()?,
    };
    for depth in 0..=crate::references::MAX_DISPATCH_SCAN_DEPTH.0 {
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
            return None;
        }
        let mut next = None;
        for command in if depth == 0 {
            walk.segment_document(source)
        } else {
            walk.segment_nested(
                source.get(scope.start as usize..scope.end as usize)?,
                scope.start,
            )
        } {
            if command.is_partial {
                return None;
            }
            let mut regions = walk.same_frame_regions(source, &command);
            regions.extend(walk.frame_shifted_regions(source, &command));
            if !walk.complete() {
                return None;
            }
            for (start, end) in regions {
                let (start, end) = (u32::try_from(start).ok()?, u32::try_from(end).ok()?);
                if start <= offset
                    && offset < end
                    && (start > scope.start || end < scope.end)
                    && next.is_none_or(|current: Scope| end - start < current.end - current.start)
                {
                    next = Some(Scope { start, end });
                }
            }
        }
        let Some(next) = next else {
            return Some(scope);
        };
        scope = next;
    }
    None
}

/// Refuse an extraction on a release profile where the interpreter can
/// observe the motion: a trace established anywhere in the document can fire
/// on a moved name, and a selected command that introspects the current frame
/// or evaluates a computed script there sees the new procedure frame.
///
/// Everything else the shared plan decides. tclsh 8.4.20, 8.5.19, 8.6.18,
/// 9.0.4 and 9.1.0 ran the rewrites of this module's `tp_*` tests before and
/// after (`catch {source file} m`; standard output and message compared):
/// identical in every case.
fn reject_observable_motion(
    source: &str,
    selected: &[&SegmentedCommand],
    analysis: &AnalysisResult,
    walk: &super::FrameWalk<'_>,
) -> Result<(), String> {
    use tcl_registry::Traits;
    let mut traced = false;
    let coverage = crate::executable_regions::visit_checked_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            traced |= walk
                .source_traits(source, command)
                .is_some_and(|traits| traits.contains(Traits::ESTABLISHES_VARIABLE_TRACE));
            false
        },
    );
    if traced {
        return Err(
            "the document establishes a trace, which can observe the moved code".to_owned(),
        );
    }
    if coverage != crate::executable_regions::PotentialSourceVisit::Complete {
        return Err("the document's commands are not all known, so a trace on the moved code cannot be ruled out".to_owned());
    }
    for root in selected {
        let mut nested = Vec::new();
        walk.nested_same_frame_commands(source, root, &mut nested);
        for command in std::iter::once(*root).chain(nested.iter()) {
            let traits = walk
                .source_traits(source, command)
                .ok_or_else(|| "the original command's frame effects are unavailable".to_owned())?;
            if traits.intersects(Traits::CURRENT_FRAME_INTROSPECTION | Traits::DYNAMIC_EVAL_BODY) {
                return Err(format!(
                    "the selection uses '{}', which observes or evaluates in the call frame",
                    command.name()
                ));
            }
        }
    }
    Ok(())
}

/// Build the extraction, or the reason it cannot be built.
/// Original selected command/body/case descriptors for a conditional source
/// extraction recipe. These are source operands and topology, not a moved
/// execution frame, current cell or procedure-call equivalence permission.
struct LogicalExtractionSourceSelection {
    image: tcl_lexer::SourceImage,
    input: tcl_compiler::analyser::ResolvedAnalysisInput,
    invocations: BTreeMap<(u32, u32), Vec<SourceExtractionCommand>>,
    bodies: Vec<tcl_compiler::registry_invocation::OriginalSourceScriptBody>,
    block: tcl_lexer::Span,
}

struct SourceExtractionCommand {
    command: SegmentedCommand,
    words: tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
}

impl LogicalExtractionSourceSelection {
    fn capture(
        source: &str,
        analysis: &AnalysisResult,
        selected: &[&SegmentedCommand],
        walk: &super::FrameWalk<'_>,
    ) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        if !input.has_logical_source_name_context() {
            return None;
        }
        let image = tcl_lexer::SourceImage::document(source);
        let context = input.context_registry();
        let mut invocations = BTreeMap::<_, Vec<_>>::new();
        let mut bodies = Vec::new();
        for root in selected {
            let mut nested = Vec::new();
            walk.nested_same_frame_commands(source, root, &mut nested);
            if !walk.complete() {
                return None;
            }
            for command in std::iter::once(*root).chain(&nested) {
                let words = walk.source_words(source, command)?;
                let selected_bodies = words.source_script_bodies_for_mutation_coverage(&context)?;
                if selected_bodies.iter().any(|body| {
                    !body.matches_source(&image, input.lexer_config())
                        || !body.matches_context(&context)
                }) {
                    return None;
                }
                bodies.extend(selected_bodies);
                invocations
                    .entry((command.span.start(), command.span.end()))
                    .or_default()
                    .push(SourceExtractionCommand {
                        command: command.clone(),
                        words,
                    });
            }
        }
        let start = walk.command_span_offsets(source, selected.first()?)?.0;
        let end = walk.command_span_offsets(source, selected.last()?)?.1;
        Some(Self {
            image,
            input: input.clone(),
            invocations,
            bodies,
            block: tcl_lexer::Span::new(start, end),
        })
    }

    fn words_for(
        &self,
        command: &SegmentedCommand,
    ) -> Option<&tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        let mut candidates = self
            .invocations
            .get(&(command.span.start(), command.span.end()))?
            .iter()
            .filter(|retained| retained.command.all_tokens == command.all_tokens);
        let first = &candidates.next()?.words;
        candidates
            .all(|other| other.words == *first)
            .then_some(first)
    }

    fn block_span(&self, source: &str, analysis: &AnalysisResult) -> Option<tcl_lexer::Span> {
        let input = analysis.resolved_input.as_ref()?;
        let config = input.lexer_config();
        if input != &self.input
            || self.image.bytes() != source.as_bytes()
            || !analysis.matches_original_source_image(&self.image, config)
            || self
                .invocations
                .values()
                .flatten()
                .any(|retained| !retained.words.matches_source(&self.image, config))
            || self.bodies.iter().any(|body| {
                !body.matches_source(&self.image, config)
                    || !body.matches_context(&input.context_registry())
            })
        {
            return None;
        }
        Some(self.block)
    }
}

fn plan_extraction(
    source: &str,
    selected: &[&SegmentedCommand],
    scope_commands: &[SegmentedCommand],
    scope: Scope,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Result<Plan, String> {
    let walk = super::FrameWalk::new(source, analysis)
        .ok_or_else(|| "the original document context is unavailable".to_owned())?;
    if !analysis.allows_lexical_declaration_advice() {
        reject_observable_motion(source, selected, analysis, &walk)?;
    }
    reject_enclosing_definition_body(source, scope, &walk)?;
    reject_frame_sensitive_selection(source, selected, &walk)?;
    let source_selection = logical_extraction_source_selection(source, analysis, selected, &walk)?;

    let roles = if let Some(selection) = source_selection.as_ref() {
        classify_selected_variables(source, selected, &walk, Some(selection))?
    } else {
        classify_variables(source, selected, &walk)?
    };

    // The selection's own byte range, snapped to the commands it covers.
    let unavailable_extent = || "the original selected command extent is unavailable".to_owned();
    let block_start = walk
        .command_span_offsets(source, selected[0])
        .ok_or_else(unavailable_extent)?
        .0;
    let block_end = walk
        .command_span_offsets(source, selected[selected.len() - 1])
        .ok_or_else(unavailable_extent)?
        .1;
    if let Some(selection) = source_selection.as_ref()
        && selection.block_span(source, analysis)
            != Some(tcl_lexer::Span::new(block_start, block_end))
    {
        return Err("the selected original source descriptors changed".to_owned());
    }
    let block = source
        .get(block_start as usize..block_end as usize)
        .ok_or_else(|| "the selected range is unreadable".to_string())?;

    // A written variable that is read again by the frame the selection runs
    // in must keep reaching the caller; one that is not becomes a proc local.
    let mut used_after: BTreeSet<String> = BTreeSet::new();
    let mut nested_tail = Vec::new();
    collect_observed_variables(
        source,
        &walk,
        (block_start, block_end),
        config,
        &mut used_after,
        &mut nested_tail,
    )?;

    let by_name: Vec<String> = roles
        .written
        .keys()
        .filter(|name| used_after.contains(*name))
        .cloned()
        .collect();
    // A name the selection both reads and writes is still an input when the
    // read comes first and it does not leave by name: `foreach x $x {…}` binds
    // `x` from a list the caller must hand over, so without that parameter the
    // moved code reads a variable nothing has set.
    let by_value: Vec<String> = roles
        .read
        .iter()
        .filter(|(name, first_read)| {
            !by_name.iter().any(|taken| taken == *name)
                && roles
                    .written
                    .get(*name)
                    .is_none_or(|visible| *first_read < visible)
        })
        .map(|(name, _)| name.clone())
        .collect();

    if !walk.complete() {
        return Err(
            "a nested command's original frame or operand grammar is unavailable".to_owned(),
        );
    }
    let name = unique_proc_name(source, analysis, registry)?;
    let indent = line_indent_at(source, block_start);
    let unavailable_reference = || {
        "a captured variable has no complete reference under the original source grammar".to_owned()
    };
    let input = analysis
        .resolved_input
        .as_ref()
        .ok_or_else(unavailable_reference)?;
    let syntax = (config, input.unit_profile().parameter_grammar());
    let definition = render_definition(&name, &by_value, &by_name, block, &indent, syntax)
        .ok_or_else(unavailable_reference)?;
    let call = render_call_for_config(&name, &by_value, &by_name, config)
        .ok_or_else(unavailable_reference)?;

    // Place the definition immediately before the enclosing *top-level*
    // command, so it lands after the file's `package require` / `namespace`
    // prologue rather than at line 0 — and, when the selection is inside a
    // proc, before that proc rather than inside it.
    let insert_at = top_level_command_start(source, scope_commands, scope, block_start, &walk)
        .ok_or_else(|| "the original insertion command extent is unavailable".to_owned())?;
    Ok(Plan {
        name,
        edits: vec![
            RefactorEdit {
                start: insert_at,
                end: insert_at,
                new_text: definition,
            },
            RefactorEdit {
                start: block_start,
                end: block_end,
                new_text: call,
            },
        ],
    })
}

fn collect_observed_variables(
    source: &str,
    walk: &super::FrameWalk<'_>,
    block: (u32, u32),
    config: LexerConfig,
    used_after: &mut BTreeSet<String>,
    nested_tail: &mut Vec<SegmentedCommand>,
) -> Result<(), String> {
    let (block_start, block_end) = block;
    for (start, end) in observing_regions(source, walk, block_start, block_end) {
        let text = source.get(start as usize..end as usize).unwrap_or_default();
        // Segmented at its real offset so the same-frame walk below can
        // address nested bodies in `source`'s own coordinates.
        for command in segment_commands_with_offset_and_config(text, start, config) {
            if command.name().is_empty() {
                continue;
            }
            used_after.extend(
                command_variable_references(source, &command, walk)?
                    .into_iter()
                    .map(|reference| reference.name),
            );
            used_after.extend(role_named_variables(source, &command, walk)?);
            // A read decides `upvar` versus proc local, so a role-named read
            // nested in a control-flow body has to count exactly as a
            // top-level one does: `if {$ok} {incr total}` carries no `$total`
            // for the text scan to find, and missing it would turn the
            // selection's write into a proc local and lose the caller's value.
            nested_tail.clear();
            walk.nested_same_frame_commands(source, &command, nested_tail);
            for inner in nested_tail.iter() {
                used_after.extend(
                    command_variable_references(source, inner, walk)?
                        .into_iter()
                        .map(|reference| reference.name),
                );
                used_after.extend(role_named_variables(source, inner, walk)?);
            }
        }
    }
    Ok(())
}

fn logical_extraction_source_selection(
    source: &str,
    analysis: &AnalysisResult,
    selected: &[&SegmentedCommand],
    walk: &super::FrameWalk<'_>,
) -> Result<Option<LogicalExtractionSourceSelection>, String> {
    Ok(
        if analysis.resolved_input.as_ref().is_some_and(
            tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
        ) {
            Some(
                LogicalExtractionSourceSelection::capture(source, analysis, selected, walk)
                    .ok_or_else(|| {
                        "the selected original body/case descriptors are unavailable".to_owned()
                    })?,
            )
        } else {
            None
        },
    )
}

/// The byte ranges that can still observe what the selection writes.
///
/// A variable the selection assigns has to keep reaching the caller's frame
/// whenever anything in that frame reads it again, so the question runs to the
/// nearest boundary that opens a variable frame of its own — a `proc` body, a
/// `namespace eval`, an `apply` lambda — or the end of the file.  An `if` or
/// `foreach` body is *not* such a boundary: it shares the caller's variables,
/// and stopping the scan at it classifies a selection made inside a loop as
/// writing a proc local, dropping every value the loop accumulates.
///
/// Each enclosing same-frame body counts in full rather than only after the
/// selection, because such a body can run again and read on its next pass what
/// this one assigned.
fn observing_regions(
    source: &str,
    walk: &super::FrameWalk<'_>,
    block_start: u32,
    block_end: u32,
) -> Vec<(u32, u32)> {
    let mut frame = (0, u32::try_from(source.len()).unwrap_or(u32::MAX));
    let mut enclosing: Vec<(u32, u32)> = Vec::new();
    let mut search = frame;
    let mut depth = 0;
    while !crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        let Some((region, opens_frame)) =
            containing_region(source, walk, search, block_start, depth)
        else {
            break;
        };
        // Every step has to narrow the window, or a region that reports itself
        // would spin here.
        if region.0 <= search.0 && region.1 >= search.1 {
            break;
        }
        if opens_frame {
            frame = region;
            enclosing.clear();
        } else {
            enclosing.push(region);
        }
        search = region;
        depth += 1;
    }
    let mut regions = vec![(block_end, frame.1)];
    regions.extend(enclosing.into_iter().map(|(start, _)| (start, block_start)));
    regions
}

/// The innermost region of `search` containing `offset`, and whether it opens
/// a variable frame of its own.
fn containing_region(
    source: &str,
    walk: &super::FrameWalk<'_>,
    search: (u32, u32),
    offset: u32,
    depth: u32,
) -> Option<((u32, u32), bool)> {
    let text = source.get(search.0 as usize..search.1 as usize)?;
    for command in if depth == 0 {
        walk.segment_document(source)
    } else {
        walk.segment_nested(text, search.0)
    } {
        if command.name().is_empty() {
            continue;
        }
        if let Some(region) =
            region_containing(&walk.frame_shifted_regions(source, &command), offset)
        {
            return Some((region, true));
        }
        if let Some(region) = region_containing(&walk.same_frame_regions(source, &command), offset)
        {
            return Some((region, false));
        }
    }
    None
}

/// The first of `regions` that contains `offset`.
fn region_containing(regions: &[(usize, usize)], offset: u32) -> Option<(u32, u32)> {
    regions.iter().copied().find_map(|(start, end)| {
        let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
            return None;
        };
        (offset >= start && offset < end).then_some((start, end))
    })
}

/// The variables the selection reads and writes, each with the offset that
/// decides whether a read is an input.
///
/// The offsets are what separates `foreach x $x {…}`, whose list word reads
/// the caller's `x` before the loop rebinds it, from `foreach n {1 2 3} {…$n…}`,
/// whose body only ever sees the name the loop just bound.
struct VariableRoles {
    /// The first offset at which the selection reads each name.
    read: BTreeMap<String, u32>,
    /// The first offset from which each written name's new value is visible:
    /// the end of the assigning command, or the start of a loop body for the
    /// name that loop binds.
    written: BTreeMap<String, u32>,
}

fn command_variable_references(
    source: &str,
    command: &SegmentedCommand,
    walk: &super::FrameWalk<'_>,
) -> Result<Vec<super::VariableReference>, String> {
    walk.variable_references(source, command)
        .ok_or_else(|| "the original variable components are unavailable".to_owned())
}

/// Classify the selection's variable use from the registry's argument roles
/// plus the `$name` references in its text.
///
/// The write set comes from [`ArgRole::VarWrite`], so `set`, `incr`,
/// `lappend`, `append`, `dict set`, `lassign`, and anything else a spec
/// declares are all covered without this module naming any of them — plus
/// [`ArgRole::LoopVarList`], the names a loop binds on every iteration
/// (`foreach n …`, `dict for {k v} …`), which the caller's frame keeps after
/// the loop exactly as an assignment does.
///
/// The classification runs over the selection's whole **statement tree**, not
/// only its top-level commands: a write nested in a control-flow body is
/// still the caller's write, and reading it as anything else hands back a
/// proc that silently drops the assignment — `foreach n {1 2 3} {set total …}`
/// extracted with `total` as a value parameter never updates the caller's
/// `total`.  [`nested_same_frame_commands`]
/// decides what "nested" means, so a body that opens its own frame stays out.
fn classify_variables(
    source: &str,
    selected: &[&SegmentedCommand],
    walk: &super::FrameWalk<'_>,
) -> Result<VariableRoles, String> {
    classify_selected_variables(source, selected, walk, None)
}

fn classify_selected_variables(
    source: &str,
    selected: &[&SegmentedCommand],
    walk: &super::FrameWalk<'_>,
    retained: Option<&LogicalExtractionSourceSelection>,
) -> Result<VariableRoles, String> {
    let mut roles = VariableRoles {
        read: BTreeMap::new(),
        written: BTreeMap::new(),
    };
    let mut nested = Vec::new();
    for command in selected {
        nested.clear();
        walk.nested_same_frame_commands(source, command, &mut nested);
        if !walk.complete() {
            return Err("the original same-frame traversal is incomplete".to_owned());
        }
        for inner in std::iter::once(*command).chain(nested.iter()) {
            for reference in command_variable_references(source, inner, walk)? {
                if reference.array {
                    return Err(
                        "an array element read needs an independent array capture contract"
                            .to_owned(),
                    );
                }
                roles
                    .read
                    .entry(reference.name)
                    .and_modify(|first| *first = (*first).min(reference.span.start()))
                    .or_insert(reference.span.start());
            }
            if let Some(retained) = retained {
                let words = retained.words_for(inner).ok_or_else(|| {
                    "the selected original command descriptor is unavailable".to_owned()
                })?;
                classify_command_words(source, inner, walk, words, &mut roles)?;
            } else {
                classify_command(source, inner, walk, &mut roles)?;
            }
        }
    }
    Ok(roles)
}

/// Complete represented source spellings for one command, using the same
/// original component/role owner as extraction. These are authored names, not
/// occupied runtime cells. Unknown layouts cannot certify a fresh spelling.
pub(super) fn logical_source_variable_spellings(
    source: &str,
    analysis: &AnalysisResult,
    command: &SegmentedCommand,
    walk: &super::FrameWalk<'_>,
) -> Option<BTreeSet<String>> {
    let words = walk.source_words(source, command)?;
    let selected_roles = words.roles()?;
    if selected_roles
        .iter()
        .any(|(_, role)| matches!(role, ArgRole::StaticVarList | ArgRole::LambdaLiteral))
    {
        return None;
    }
    let mut roles = VariableRoles {
        read: BTreeMap::new(),
        written: BTreeMap::new(),
    };
    for reference in command_variable_references(source, command, walk).ok()? {
        roles.read.insert(reference.name, reference.span.start());
    }
    classify_command(source, command, walk, &mut roles).ok()?;
    let mut names = roles
        .read
        .into_keys()
        .chain(roles.written.into_keys())
        .collect::<BTreeSet<_>>();
    for &(parameter, role) in selected_roles {
        if role != ArgRole::ParamList {
            continue;
        }
        let parameter_word = words.original_argument_word(parameter)?;
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config?;
        let input = analysis.resolved_input.as_ref()?;
        let names_at = selected_roles
            .iter()
            .filter(|(_, role)| *role == ArgRole::Name)
            .filter_map(|(argument, _)| words.original_argument_word(*argument))
            .collect::<Vec<_>>();
        let bodies_at = selected_roles
            .iter()
            .filter(|(_, role)| *role == ArgRole::Body)
            .filter_map(|(argument, _)| words.original_argument_word(*argument))
            .collect::<Vec<_>>();
        let mut represented = false;
        for declaration in analysis.checked_logical_source_procedure_declarations(&image, config)? {
            if declaration.original_definition_site()?.offset != command.argv.first()?.span.start()
                || declaration.original_parameter_word() != Some(parameter_word)
                || !declaration
                    .original_name_word()
                    .is_some_and(|name| names_at.contains(&name))
                || !declaration
                    .original_body_word(&image, input)
                    .is_some_and(|body| bodies_at.contains(&body))
            {
                continue;
            }
            names.extend(
                declaration
                    .original_source_formals()?
                    .iter()
                    .map(|parameter| parameter.name.clone()),
            );
            represented = true;
        }
        for name in names_at {
            if let Some(member) =
                analysis.logical_source_member_at(&image, config, name.span().start())
                && member.original_name_word() == Some(name)
                && member.original_parameter_word() == Some(parameter_word)
                && member
                    .original_body_word()
                    .is_some_and(|body| bodies_at.contains(&body))
            {
                names.extend(
                    member
                        .original_source_formals()?
                        .into_iter()
                        .map(|parameter| parameter.name),
                );
                represented = true;
            }
        }
        if !represented {
            return None;
        }
    }
    Some(names)
}

fn classify_command_words(
    source: &str,
    command: &SegmentedCommand,
    walk: &super::FrameWalk<'_>,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    roles: &mut VariableRoles,
) -> Result<(), String> {
    let head = command.name();
    let unavailable = || format!("the original variable roles of '{head}' are unavailable");
    let selected_roles = words.roles().ok_or_else(unavailable)?;
    let (command_start, command_end) = walk
        .command_span_offsets(source, command)
        .ok_or_else(unavailable)?;
    let binds_at = words
        .written_argument_roles()
        .into_iter()
        .find_map(|(ordinal, role)| {
            (role == ArgRole::Body)
                .then(|| command.argv.get(ordinal + 1).map(|word| word.span.start()))
                .flatten()
        })
        .unwrap_or(command_end);
    for &(ordinal, role) in selected_roles {
        if !matches!(
            role,
            ArgRole::VarRead | ArgRole::VarWrite | ArgRole::LoopVarList
        ) {
            continue;
        }
        let name = words
            .arguments()
            .get(ordinal)
            .and_then(|word| word.literal_bytes())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .ok_or_else(unavailable)?;
        match role {
            ArgRole::VarRead => {
                roles.read.entry(name.to_owned()).or_insert(command_start);
            }
            ArgRole::VarWrite => {
                record_write(roles, carriable_written_name(head, name)?, command_end);
            }
            ArgRole::LoopVarList => {
                let names = tcl_syntax::word_rules::WordValueRules::from_grammar(
                    &walk.config.grammar_over(walk.dialect.grammar),
                )
                .split_list(name)
                .map_err(|_| unavailable())?;
                for name in names {
                    record_write(roles, carriable_written_name(head, &name)?, binds_at);
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

/// Fold one command's registry-declared variable roles into `roles`.
///
/// A write becomes visible only once the command has evaluated its own
/// arguments, so the offset recorded for it is the command's end — which is
/// what makes the `$y` of `set y [expr {$y + 1}]` an input.  A loop binding is
/// visible from its body instead: the words before that body, the list a
/// `foreach` iterates among them, still read the caller's variable.
fn classify_command(
    source: &str,
    command: &SegmentedCommand,
    walk: &super::FrameWalk<'_>,
    roles: &mut VariableRoles,
) -> Result<(), String> {
    let head = command.name();
    let unavailable = || format!("the original variable roles of '{head}' are unavailable");
    let words = walk.source_words(source, command).ok_or_else(unavailable)?;
    classify_command_words(source, command, walk, &words, roles)
}

/// Record `name` as written, keeping the earliest offset at which its new
/// value is visible.
fn record_write(roles: &mut VariableRoles, name: String, at: u32) {
    roles
        .written
        .entry(name)
        .and_modify(|first| *first = (*first).min(at))
        .or_insert(at);
}

/// `name` as a variable this transform can carry back to the caller, or the
/// reason it cannot.
fn carriable_written_name(head: &str, name: &str) -> Result<String, String> {
    if tcl_syntax::naming::split_element_ref_bytes(name.as_bytes()).is_some() {
        return Err(format!(
            "'{head}' writes to the array element `{name}`; carrying an \
             array element back to the caller needs more than the scalar \
             `upvar` protocol this transform uses"
        ));
    }
    Ok(name.to_string())
}

/// Refuse a selection containing a command that acts on the call frame.
///
/// Membership comes from the central selected-traits frame predicate — the
/// registry's union of block terminators, control transfers, scope aliases,
/// and barriers — so this names no command.  Every member changes meaning
/// when a `proc` boundary is introduced between it and its frame: `return`
/// would return from the new proc rather than the caller, `break` would
/// escape a loop that is no longer enclosing it, and `upvar 1` would alias
/// the caller's caller.
fn reject_frame_sensitive_selection(
    source: &str,
    selected: &[&SegmentedCommand],
    walk: &super::FrameWalk<'_>,
) -> Result<(), String> {
    for root in selected {
        let mut nested = Vec::new();
        walk.nested_same_frame_commands(source, root, &mut nested);
        for command in std::iter::once(*root).chain(nested.iter()) {
            let traits = walk
                .source_traits(source, command)
                .ok_or_else(|| "the original command's frame effects are unavailable".to_owned())?;
            if tcl_registry::traits::is_frame_sensitive(traits) {
                return Err(format!(
                    "the selection uses '{}', which acts on the call frame",
                    command.name()
                ));
            }
        }
    }
    Ok(())
}

/// Refuse a selection nested inside a namespace or class definition body.
///
/// The extracted proc would be created in a different namespace from the one
/// the moved code ran in, so its unqualified command calls and `variable`
/// references would silently resolve elsewhere.  Both tests are registry
/// data: [`Traits::DECLARES_NAMESPACE`] for `namespace eval` and its kin, and
/// a spec's `definition_body` grammar for a class/type definer's body.
fn reject_enclosing_definition_body(
    source: &str,
    scope: Scope,
    walk: &super::FrameWalk<'_>,
) -> Result<(), String> {
    if scope.start == 0 {
        return Ok(());
    }
    for command in walk.segment_document(source) {
        let (start, end) = walk
            .command_span_offsets(source, &command)
            .ok_or_else(|| "the original enclosing declaration extent is unavailable".to_owned())?;
        if scope.start < start || scope.end > end {
            continue;
        }
        let words = walk
            .source_words(source, &command)
            .ok_or_else(|| "the original enclosing declaration is unavailable".to_owned())?;
        let definition = words
            .with_source_schema(&walk.source_context(), |schema| {
                schema.semantics.traits.contains(Traits::DECLARES_NAMESPACE)
                    || schema
                        .authored_source_descriptors()
                        .command
                        .definition_body
                        .is_some()
            })
            .ok_or_else(|| "the original declaration grammar is unavailable".to_owned())?;
        if definition {
            return Err(format!(
                "the selection is inside a '{}' definition body with a distinct naming context",
                command.name()
            ));
        }
    }
    Ok(())
}

/// The byte offset of the line start of the outermost top-level command
/// containing `offset` — where the extracted definition is inserted.
fn top_level_command_start(
    source: &str,
    scope_commands: &[SegmentedCommand],
    scope: Scope,
    offset: u32,
    walk: &super::FrameWalk<'_>,
) -> Option<u32> {
    // A selection already at top level inserts before its own first command.
    let anchor = if scope.start == 0 {
        scope_commands
            .iter()
            .map(|command| {
                walk.command_span_offsets(source, command)
                    .map(|extent| extent.0)
            })
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .filter(|start| *start <= offset)
            .max()
            .unwrap_or(offset)
    } else {
        walk.segment_document(source)
            .iter()
            .map(|command| walk.command_span_offsets(source, command))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .filter(|&(start, end)| start <= offset && offset < end)
            .map(|(start, _)| start)
            .min()
            .unwrap_or(offset)
    };
    Some(line_start_of(source, anchor))
}

/// The byte offset of the start of the line containing `offset`.
fn line_start_of(source: &str, offset: u32) -> u32 {
    let index = (offset as usize).min(source.len());
    u32::try_from(source[..index].rfind('\n').map_or(0, |at| at + 1)).unwrap_or(0)
}

/// The leading whitespace of the line containing `offset`.
fn line_indent_at(source: &str, offset: u32) -> String {
    let start = line_start_of(source, offset) as usize;
    source[start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// Render the generated proc definition.
///
/// A by-name parameter is spelled `<var>Name` and immediately re-bound with
/// `upvar 1`, so the moved code keeps writing to the caller's variable under
/// its original spelling and the body needs no rewriting at all.
fn render_definition(
    name: &str,
    by_value: &[String],
    by_name: &[String],
    block: &str,
    block_indent: &str,
    syntax: (LexerConfig, Option<tcl_dialect::ParameterGrammar>),
) -> Option<String> {
    use std::fmt::Write as _;
    let (config, parameter_grammar) = syntax;
    let mut params: Vec<String> = by_value.to_vec();
    params.extend(by_name.iter().map(|var| format!("{var}Name")));
    let mut body = String::new();
    for var in by_name {
        // The parameter read must reproduce its exact name under the selected
        // source grammar. The target is name data and stays list-quoted.
        let _ = writeln!(
            body,
            "    upvar 1 {} {}",
            tcl_syntax::naming::scalar_source_reference(&format!("{var}Name"), config, false)?,
            tcl_syntax::word_rules::lexical_unicode_source_word(var, config)?
        );
    }
    let moved_start = body.len();
    for line in block.lines() {
        let stripped = line.strip_prefix(block_indent).unwrap_or(line);
        if stripped.trim().is_empty() {
            body.push('\n');
        } else {
            let _ = writeln!(body, "    {}", stripped.trim_end());
        }
    }
    let original = script_word_spellings(block, config.nested())?;
    if !script_word_spellings(&body[moved_start..], config.nested())
        .is_some_and(|words| words == original)
    {
        body.truncate(moved_start);
        body.push_str(block);
        body.push('\n');
    }
    let formals = required_formal_word(&params, config, parameter_grammar)?;
    Some(format!("proc {name} {formals} {{\n{body}}}\n\n"))
}

/// A formal word contains a list of specifiers, each itself a Tcl list.
/// A selected binding must assign every supplied argument to its intended name.
fn required_formal_word(
    names: &[String],
    config: LexerConfig,
    selected: Option<tcl_dialect::ParameterGrammar>,
) -> Option<String> {
    use tcl_dialect::ParameterGrammar;
    use tcl_syntax::formal_params::{
        FormalArgumentBinding, bind_formal_arguments, parse_formal_parameters_in,
    };
    if names.iter().collect::<BTreeSet<_>>().len() != names.len() {
        return None;
    }
    let specs = names
        .iter()
        .map(|name| tcl_syntax::list::list_element(name));
    let list = tcl_syntax::list::join_list(specs);
    let actual =
        tcl_syntax::list::split_list_with_syntax(&list, config.list_parse, config.escapes).ok()?;
    if actual.len() != names.len() {
        return None;
    }
    for (spec, name) in actual.iter().zip(names) {
        let fields =
            tcl_syntax::list::split_list_with_syntax(spec, config.list_parse, config.escapes)
                .ok()?;
        if fields.as_slice() != [name.as_str()] {
            return None;
        }
    }
    for grammar in [ParameterGrammar::Tcl, ParameterGrammar::Jim] {
        if selected.is_some_and(|required| required != grammar) {
            continue;
        }
        let params = parse_formal_parameters_in(&list, grammar).ok()?;
        if !params.iter().map(|p| &p.name).eq(names) {
            return None;
        }
        let bindings = bind_formal_arguments(&params, names.len(), grammar).ok()?;
        if !bindings.iter().enumerate().all(|(index,binding)| matches!(binding,FormalArgumentBinding::Value{parameter,argument} if *parameter == index && *argument == index)) {return None;}
    }
    let braced = format!("{{{list}}}");
    tcl_syntax::word_rules::lexical_unicode_source_word_matches(&braced, &list, config)
        .then_some(braced)
}

/// Render the call that replaces the selection.
///
/// A by-value parameter is passed as `$var`; a by-name one is passed as the
/// *name* itself, which is what `upvar` in the proc binds against.
fn render_call_for_config(
    name: &str,
    by_value: &[String],
    by_name: &[String],
    config: LexerConfig,
) -> Option<String> {
    let mut words = vec![name.to_string()];
    for var in by_value {
        words.push(tcl_syntax::naming::scalar_source_reference(
            var, config, false,
        )?);
    }
    for var in by_name {
        words.push(tcl_syntax::word_rules::lexical_unicode_source_word(
            var, config,
        )?);
    }
    Some(words.join(" "))
}

#[cfg(test)]
fn render_call(name: &str, by_value: &[String], by_name: &[String]) -> String {
    render_call_for_config(
        name,
        by_value,
        by_name,
        LexerConfig {
            braced_var: BracedVarStyle::Tcl9Nesting,
            ..LexerConfig::default()
        },
    )
    .expect("fixed call-rendering fixture has a complete reference")
}

/// A proc name not already taken by a workspace symbol or a registry command.
///
/// The generated name is a placeholder the user renames straight away, but it
/// must not *shadow* something in the meantime: defining `proc lsort {…}` by
/// accident would silently replace the builtin for the rest of the file.
///
/// Native source worlds match exact published byte slots and policies. The
/// generated definition is global; a declaration in another namespace does
/// not occupy that slot. Reporting-only dialects keep lexical declaration
/// advice explicitly separate. Unknown occupancy or exhaustion refuses.
fn unique_proc_name(
    source: &str,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> Result<String, String> {
    if analysis.allows_lexical_declaration_advice() {
        return first_fresh_proc_name(|candidate| {
            let global = tcl_syntax::naming::root_unrooted_key(candidate);
            Some(
                registry.get(candidate).is_some()
                    || analysis
                        .all_procs
                        .values()
                        .any(|proc| proc.qualified_name == global),
            )
        });
    }
    let config = analysis
        .body_lexer_config
        .ok_or("the original lexer configuration is unavailable")?;
    if !analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config) {
        return Err("the original source changed after analysis".to_owned());
    }
    let world = analysis.original_completed_command_world().ok_or(
        "the current source command table is unknown, so a fresh procedure slot cannot be selected",
    )?;
    first_fresh_proc_name(|candidate| {
        if registry.get(candidate).is_some() {
            return Some(true);
        }
        for publication in world.declarations() {
            if crate::original_declaration::literal_name_matches_slot(
                candidate,
                publication.slot(),
                publication.policy(),
            )? {
                return Some(true);
            }
        }
        // Reserve genuine declared slots throughout the document too. This
        // avoids changing a later declaration when the new proc is inserted.
        for declaration in analysis.original_procedure_declarations() {
            if crate::original_declaration::literal_name_matches_publication(
                candidate,
                declaration.name(),
            )? {
                return Some(true);
            }
        }
        for declaration in analysis.original_class_declarations() {
            if crate::original_declaration::literal_name_matches_publication(
                candidate,
                declaration.name(),
            )? {
                return Some(true);
            }
        }
        Some(false)
    })
}

fn first_fresh_proc_name(mut taken: impl FnMut(&str) -> Option<bool>) -> Result<String, String> {
    for suffix in 1..=MAX_NAME_SUFFIX {
        let candidate = if suffix == 1 {
            BASE_NAME.to_owned()
        } else {
            format!("{BASE_NAME}_{suffix}")
        };
        match taken(&candidate) {
            Some(false) => return Ok(candidate),
            Some(true) => {}
            None => {
                return Err(
                    "the proposed procedure name has no native publication geometry".to_owned(),
                );
            }
        }
    }
    Err(
        "all generated procedure names are occupied; choose an unused name before extracting"
            .to_owned(),
    )
}

#[cfg(test)]
fn variable_references(text: &str, style: BracedVarStyle) -> BTreeSet<String> {
    super::syntax_variable_references(
        text,
        LexerConfig {
            braced_var: style,
            ..LexerConfig::default()
        },
    )
    .unwrap_or_default()
    .into_iter()
    .map(|reference| reference.name)
    .collect()
}

/// The variable names `command` names through a registry `VarRead` /
/// `VarWrite` role — a use that carries no `$`, so the `$name` scan misses it.
fn role_named_variables(
    source: &str,
    command: &SegmentedCommand,
    walk: &super::FrameWalk<'_>,
) -> Result<Vec<String>, String> {
    let words = walk
        .source_words(source, command)
        .ok_or_else(|| "an observing command's original roles are unavailable".to_owned())?;
    words
        .roles()
        .ok_or_else(|| "an observing command's original roles are incomplete".to_owned())?
        .iter()
        .filter(|(_, role)| matches!(role, ArgRole::VarRead | ArgRole::VarWrite))
        .map(|(ordinal, _)| {
            words
                .arguments()
                .get(*ordinal)
                .and_then(|word| word.literal_bytes())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .map(str::to_owned)
                .ok_or_else(|| "an observing variable name is unavailable".to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_extract_tests;

#[cfg(test)]
mod selected_source_variable_tests;

#[cfg(test)]
mod exact_logical_paramlist_owner_tests;

#[cfg(test)]
mod logical_extraction_selected_source_tests;
