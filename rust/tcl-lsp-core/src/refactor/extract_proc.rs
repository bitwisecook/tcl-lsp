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
//! `after=1`: the write moved into the proc's own `x` (issue #1201).
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

use tcl_dialect::BracedVarStyle;
use tcl_lexer::LexerConfig;

use super::{RefactorEdit, Refactoring, command_span_offsets};
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
    registry: &CommandRegistry,
) -> Option<Refactoring> {
    let (sel_start, sel_end) = selection;
    if sel_end <= sel_start {
        return None;
    }
    // The document's own lexing grammar — `analysis.dialect` carries the
    // name the host analysed this document under (issue: dialect-drift).
    let config =
        LexerConfig::from_grammar(crate::environment_for_dialect(&analysis.dialect).grammar());
    let scope = enclosing_scope(source, sel_start, registry, config);
    let scope_text = source.get(scope.start as usize..scope.end as usize)?;
    let commands: Vec<SegmentedCommand> =
        segment_commands_with_offset_and_config(scope_text, scope.start, config)
            .into_iter()
            .filter(|command| !command.name().is_empty())
            .collect();
    let selected: Vec<&SegmentedCommand> = commands
        .iter()
        .filter(|command| {
            let (start, end) = command_span_offsets(source, command);
            start >= sel_start && end <= sel_end
        })
        .collect();
    if selected.is_empty() {
        return None;
    }
    let title = "Extract selection into proc".to_string();
    match plan_extraction(
        source, &selected, &commands, scope, analysis, registry, config,
    ) {
        Ok(plan) => Some(Refactoring {
            title,
            edits: plan.edits,
            kind: ActionKind::RefactorExtract,
            data_group: None,
            disabled: None,
        }),
        Err(reason) => Some(Refactoring {
            title,
            edits: Vec::new(),
            kind: ActionKind::RefactorExtract,
            data_group: None,
            disabled: Some(reason),
        }),
    }
}

/// The command-and-rename payload the code-action layer attaches after
/// applying an extraction's edits.
///
/// Exposed separately from [`Refactoring`] because the post-extract rename is
/// an editor *command*, not an edit: the generated name is a placeholder the
/// user is expected to replace immediately.
#[must_use]
pub fn extract_proc_rename_command(
    refactoring: &Refactoring,
    source: &str,
) -> Option<ActionCommand> {
    if refactoring.disabled.is_some() {
        return None;
    }
    let definition = refactoring.edits.iter().min_by_key(|edit| edit.start)?;
    let name_offset = definition.new_text.find("proc ")? + "proc ".len();
    let name_end = definition.new_text[name_offset..]
        .find(char::is_whitespace)
        .map(|len| name_offset + len)?;
    // The definition is inserted at `definition.start`; the name sits on the
    // definition's first line, whose column is the name's offset within the
    // inserted text (the insertion always begins at a line start).
    let line = source[..definition.start as usize]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    Some(ActionCommand {
        command: "tclLsp.renameSymbolAtPosition".to_string(),
        args: vec![
            u32::try_from(line).unwrap_or(0),
            u32::try_from(name_offset).unwrap_or(0),
            u32::try_from(name_end).unwrap_or(0),
        ],
        string_args: Vec::new(),
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
    edits: Vec<RefactorEdit>,
}

/// Build the extraction, or the reason it cannot be built.
fn plan_extraction(
    source: &str,
    selected: &[&SegmentedCommand],
    scope_commands: &[SegmentedCommand],
    scope: Scope,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Result<Plan, String> {
    reject_enclosing_definition_body(source, scope, registry, config)?;
    reject_frame_sensitive_selection(source, selected, registry)?;
    // The document's own `${…}` close rule — a brace-bearing name read by
    // the wrong release's rule produces a proc built for a variable that
    // does not exist (issue #1605).
    let style = super::braced_var_style(analysis);
    let walk = FrameWalk::new(source, analysis);
    let roles = classify_variables(source, selected, registry, &walk, style)?;

    // The selection's own byte range, snapped to the commands it covers.
    let block_start = command_span_offsets(source, selected[0]).0;
    let block_end = command_span_offsets(source, selected[selected.len() - 1]).1;
    let block = source
        .get(block_start as usize..block_end as usize)
        .ok_or_else(|| "the selected range is unreadable".to_string())?;

    // A written variable that is read again by the frame the selection runs
    // in must keep reaching the caller; one that is not becomes a proc local.
    let mut used_after: BTreeSet<String> = BTreeSet::new();
    let mut nested_tail = Vec::new();
    for (start, end) in observing_regions(source, &walk, block_start, block_end) {
        let text = source.get(start as usize..end as usize).unwrap_or_default();
        used_after.extend(variable_references(text, style));
        // Segmented at its real offset so the same-frame walk below can
        // address nested bodies in `source`'s own coordinates.
        for command in segment_commands_with_offset_and_config(text, start, config) {
            if command.name().is_empty() {
                continue;
            }
            used_after.extend(role_named_variables(&command, registry));
            // A read decides `upvar` versus proc local, so a role-named read
            // nested in a control-flow body has to count exactly as a
            // top-level one does: `if {$ok} {incr total}` carries no `$total`
            // for the text scan to find, and missing it would turn the
            // selection's write into a proc local and lose the caller's value.
            nested_tail.clear();
            nested_same_frame_commands(source, &command, &walk, 0, &mut nested_tail);
            for inner in &nested_tail {
                used_after.extend(role_named_variables(inner, registry));
            }
        }
    }

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

    let name = unique_proc_name(analysis, registry);
    let indent = line_indent_at(source, block_start);
    let definition = render_definition(&name, &by_value, &by_name, block, &indent);
    let call = render_call(&name, &by_value, &by_name);

    // Place the definition immediately before the enclosing *top-level*
    // command, so it lands after the file's `package require` / `namespace`
    // prologue rather than at line 0 — and, when the selection is inside a
    // proc, before that proc rather than inside it.
    let insert_at = top_level_command_start(source, scope_commands, scope, block_start, config);
    Ok(Plan {
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
    walk: &FrameWalk,
    block_start: u32,
    block_end: u32,
) -> Vec<(u32, u32)> {
    let mut frame = (0, u32::try_from(source.len()).unwrap_or(u32::MAX));
    let mut enclosing: Vec<(u32, u32)> = Vec::new();
    let mut search = frame;
    let mut depth = 0;
    while !crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        let Some((region, opens_frame)) = containing_region(source, walk, search, block_start)
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
    walk: &FrameWalk,
    search: (u32, u32),
    offset: u32,
) -> Option<((u32, u32), bool)> {
    let text = source.get(search.0 as usize..search.1 as usize)?;
    for command in segment_commands_with_offset_and_config(text, search.0, walk.config) {
        if command.name().is_empty() {
            continue;
        }
        if let Some(region) = region_containing(
            &crate::references::frame_shifted_dispatch_regions(source, walk.dialect, &command),
            offset,
        ) {
            return Some((region, true));
        }
        if let Some(region) =
            region_containing(&same_frame_regions(source, &command, walk), offset)
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

/// The document facts a same-frame statement walk needs, built once per
/// extraction.
///
/// `nesting` is deliberately the **document's own** registry rather than the
/// caller's: which nested words are same-frame scripts is a question
/// [`crate::references::nested_dispatch_regions`] answers from the dialect
/// profile's registry (a `switch` clause list reaches its arm bodies only
/// through that registry's `CaseListSpec`), whereas the argument roles this
/// module classifies stay the caller's to decide.
struct FrameWalk {
    dialect: &'static tcl_dialect::DialectProfile,
    nesting: &'static CommandRegistry,
    identities: tcl_compiler::realm::CommandBindingRealm,
    config: LexerConfig,
    expr_surface: tcl_registry::expr_surface::RuntimeExprSurface,
}

impl FrameWalk {
    fn new(source: &str, analysis: &AnalysisResult) -> Self {
        let dialect = crate::profile_for_dialect(&analysis.dialect);
        let nesting = crate::registry_for_dialect_profile(dialect);
        Self {
            dialect,
            nesting,
            identities: tcl_compiler::realm::document_realm_bindings(source, dialect, nesting),
            config: LexerConfig::from_grammar(dialect.grammar),
            expr_surface: tcl_registry::expr_surface::RuntimeExprSurface::for_profile(dialect),
        }
    }
}

/// The regions of `command` that run in `command`'s own variable frame.
///
/// [`crate::references::nested_dispatch_regions`] owns the script ones. It
/// cannot see inside a *braced* expression argument, which the script lexer
/// treats as one opaque word and `expr` substitutes itself, so `if {[set x 1]}
/// …` would hide a write to the caller's `x`.  Those spans come from
/// [`tcl_syntax::expr::substitution::command_substitution_spans`], the
/// expression owner's own script bridge, gated on the dialect's runtime
/// expression surface: an expression the release would reject at run time
/// substitutes nothing.
fn same_frame_regions(
    source: &str,
    command: &SegmentedCommand,
    walk: &FrameWalk,
) -> Vec<(usize, usize)> {
    let mut regions = crate::references::nested_dispatch_regions_with_identities(
        source,
        walk.dialect,
        walk.nesting,
        &walk.identities,
        command,
    );
    let head = command.name();
    let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
    for index in walk
        .nesting
        .arg_indices_for_role(head, &args, ArgRole::Expr)
    {
        let Some(token) = command.argv.get(index + 1) else {
            continue;
        };
        // An unbraced or quoted expression word is substituted by the script
        // lexer before `expr` ever parses it, so its `[…]` are already among
        // the dispatch regions above; only a braced one needs the expression
        // parser to find them.
        if source.as_bytes().get(token.span.start() as usize) != Some(&b'{')
            || token.content_offset != 1
        {
            continue;
        }
        let start = token.span.start() as usize + token.content_offset as usize;
        let end = token.span.end() as usize;
        let Some(expression) = source.get(start..end) else {
            continue;
        };
        for span in tcl_syntax::expr::substitution::command_substitution_spans(
            expression,
            walk.dialect,
            walk.config,
            |parsed| walk.expr_surface.validate(parsed).is_ok(),
        ) {
            // The span carries the `[` and `]`; the script inside them is what
            // runs.
            let (Some(inner_start), Some(inner_end)) = (
                start.checked_add(span.start() as usize + 1),
                start
                    .checked_add(span.end() as usize)
                    .and_then(|e| e.checked_sub(1)),
            ) else {
                continue;
            };
            if inner_start < inner_end {
                regions.push((inner_start, inner_end));
            }
        }
    }
    regions
}

/// Every command nested inside `command` that still runs in `command`'s own
/// variable frame, appended to `out` innermost-first.
///
/// The nesting itself is [`crate::references::nested_dispatch_regions`]'s
/// answer — the same walker Find-References and the caller-frame scan use for
/// "which nested scripts run in this frame": an [`ArgRole::Body`] argument
/// with a `Plain` body kind (`if`, `while`, `foreach`, `try`, `catch`, …), a
/// `switch`-style clause list flattened through the registry's own
/// `CaseListSpec`, and every `[…]` command substitution.  A `Structural`
/// body — `proc`, `namespace eval`, `uplevel`, `oo::define` — and `apply`'s
/// lambda are deliberately *not* descended: a `set` inside one writes that
/// frame's variable, not the selection's, so classifying it here would put a
/// stranger's name in the generated parameter list or, worse, an `upvar`
/// against a variable the moved code never touches.
fn nested_same_frame_commands(
    source: &str,
    command: &SegmentedCommand,
    walk: &FrameWalk,
    depth: u32,
    out: &mut Vec<SegmentedCommand>,
) {
    if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return;
    }
    for (start, end) in same_frame_regions(source, command, walk) {
        let Some(text) = source.get(start..end) else {
            continue;
        };
        for nested in segment_commands_with_offset_and_config(
            text,
            u32::try_from(start).unwrap_or(0),
            walk.config,
        ) {
            if nested.name().is_empty() {
                continue;
            }
            nested_same_frame_commands(source, &nested, walk, depth + 1, out);
            out.push(nested);
        }
    }
}

/// Every region inside `command` that runs in a variable frame of its own,
/// appended to `out`.
///
/// The frame-opening bodies are [`crate::references::frame_shifted_dispatch_regions`]'s
/// answer — the exact complement of the same-frame set
/// [`nested_same_frame_commands`] walks — asked of `command` and of every
/// same-frame command beneath it, so a `proc` nested in an `if` branch is
/// found as readily as one at the selection's top level.
fn frame_shifted_regions_within(
    source: &str,
    command: &SegmentedCommand,
    walk: &FrameWalk,
    out: &mut Vec<(u32, u32)>,
) {
    let mut push = |regions: Vec<(usize, usize)>| {
        for (start, end) in regions {
            let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
                continue;
            };
            out.push((start, end));
        }
    };
    push(crate::references::frame_shifted_dispatch_regions(
        source,
        walk.dialect,
        command,
    ));
    let mut nested = Vec::new();
    nested_same_frame_commands(source, command, walk, 0, &mut nested);
    for inner in &nested {
        push(crate::references::frame_shifted_dispatch_regions(
            source,
            walk.dialect,
            inner,
        ));
    }
}

/// `start..end` with every range in `holes` cut out of it.
fn same_frame_slices(start: u32, end: u32, holes: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut cuts: Vec<(u32, u32)> = holes
        .iter()
        .copied()
        .filter(|(from, to)| *to > start && *from < end && from < to)
        .collect();
    cuts.sort_unstable();
    let mut slices = Vec::new();
    let mut cursor = start;
    for (from, to) in cuts {
        if from > cursor {
            slices.push((cursor, from.min(end)));
        }
        cursor = cursor.max(to);
        if cursor >= end {
            break;
        }
    }
    if cursor < end {
        slices.push((cursor, end));
    }
    slices
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
/// `total` (issue #1201).  [`nested_same_frame_commands`]
/// decides what "nested" means, so a body that opens its own frame stays out.
fn classify_variables(
    source: &str,
    selected: &[&SegmentedCommand],
    registry: &CommandRegistry,
    walk: &FrameWalk,
    style: BracedVarStyle,
) -> Result<VariableRoles, String> {
    let mut roles = VariableRoles {
        read: BTreeMap::new(),
        written: BTreeMap::new(),
    };
    let mut nested = Vec::new();
    let mut frames = Vec::new();
    for command in selected {
        // One text scan per selected command covers the `$name` reads of its
        // whole subtree, minus the bodies that open a variable frame of their
        // own: a `$name` in a nested proc body or an `apply` lambda names that
        // frame's variable, so making it a parameter would ask the caller for
        // a variable it does not have.
        let (start, end) = command_span_offsets(source, command);
        frames.clear();
        frame_shifted_regions_within(source, command, walk, &mut frames);
        for (from, to) in same_frame_slices(start, end, &frames) {
            let text = source.get(from as usize..to as usize).unwrap_or("");
            for (name, at, _) in super::variable_reference_spans(text, style) {
                let at = from.saturating_add(u32::try_from(at).unwrap_or(0));
                roles
                    .read
                    .entry(name)
                    .and_modify(|first| *first = (*first).min(at))
                    .or_insert(at);
            }
        }
        classify_command(source, command, registry, &mut roles)?;
        nested.clear();
        nested_same_frame_commands(source, command, walk, 0, &mut nested);
        for inner in &nested {
            classify_command(source, inner, registry, &mut roles)?;
        }
    }
    Ok(roles)
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
    registry: &CommandRegistry,
    roles: &mut VariableRoles,
) -> Result<(), String> {
    let head = command.name();
    if head.contains('$') || head.contains('[') || head.contains('{') {
        return Err(format!(
            "the command head `{head}` is computed at run time, so which \
             variables the command reads and writes is unknown"
        ));
    }
    let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
    let (command_start, command_end) = command_span_offsets(source, command);
    for index in registry.arg_indices_for_role(head, &args, ArgRole::VarRead) {
        if let Some(name) = args.get(index) {
            roles
                .read
                .entry((*name).to_string())
                .or_insert(command_start);
        }
    }
    for index in registry.arg_indices_for_role(head, &args, ArgRole::VarWrite) {
        let Some(name) = args.get(index) else {
            continue;
        };
        record_write(roles, carriable_written_name(head, name)?, command_end);
    }
    // A loop variable-binding word is a *list* of names (`foreach {k v} …`),
    // so the list owner splits it rather than this module reading it as one.
    let binds_at = loop_body_start(command, registry, &args).unwrap_or(command_end);
    for index in registry.arg_indices_for_role(head, &args, ArgRole::LoopVarList) {
        let Some(word) = args.get(index) else {
            continue;
        };
        let Ok(names) = tcl_syntax::list::split_list(word) else {
            // An unparseable list is a syntax error in the document, not a
            // binding this transform can name.
            continue;
        };
        for name in names {
            record_write(roles, carriable_written_name(head, &name)?, binds_at);
        }
    }
    Ok(())
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

/// Where `command`'s first body argument starts — the point from which the
/// names it binds hold the loop's values rather than the caller's.
fn loop_body_start(
    command: &SegmentedCommand,
    registry: &CommandRegistry,
    args: &[&str],
) -> Option<u32> {
    let index = registry
        .arg_indices_for_role(command.name(), args, ArgRole::Body)
        .into_iter()
        .next()?;
    Some(command.argv.get(index + 1)?.span.start())
}

/// `name` as a variable this transform can carry back to the caller, or the
/// reason it cannot.
fn carriable_written_name(head: &str, name: &str) -> Result<String, String> {
    if name.contains('$') || name.contains('[') {
        return Err(format!(
            "'{head}' writes to a variable whose name is computed \
             (`{name}`), so the extraction cannot tell which variable \
             leaves the selection"
        ));
    }
    if name.contains('(') {
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
/// Membership comes from [`CommandRegistry::frame_sensitive_commands`] — the
/// registry's union of block terminators, control transfers, scope aliases,
/// and barriers — so this names no command.  Every member changes meaning
/// when a `proc` boundary is introduced between it and its frame: `return`
/// would return from the new proc rather than the caller, `break` would
/// escape a loop that is no longer enclosing it, and `upvar 1` would alias
/// the caller's caller.
fn reject_frame_sensitive_selection(
    source: &str,
    selected: &[&SegmentedCommand],
    registry: &CommandRegistry,
) -> Result<(), String> {
    let unsafe_heads = registry.frame_sensitive_commands();
    for command in selected {
        let (start, end) = command_span_offsets(source, command);
        let text = source.get(start as usize..end as usize).unwrap_or("");
        for word in text.split(|c: char| !(c.is_alphanumeric() || c == ':' || c == '_')) {
            if !word.is_empty() && unsafe_heads.contains(&word) {
                return Err(format!(
                    "the selection uses '{word}', which acts on the call frame — \
                     putting a `proc` boundary between it and that frame would \
                     change what it returns from, breaks out of, or binds against"
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
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Result<(), String> {
    if scope.start == 0 {
        return Ok(());
    }
    // Walk the top-level commands to find the one whose body the scope sits
    // in, then check what kind of body it is.
    for command in segment_commands_with_offset_and_config(source, 0, config) {
        let (start, end) = command_span_offsets(source, &command);
        if scope.start < start || scope.end > end {
            continue;
        }
        let head = command.name();
        let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
        let traits = registry.call_traits(head, &args);
        if traits.contains(Traits::DECLARES_NAMESPACE) {
            return Err(format!(
                "the selection is inside a '{head}' body — an extracted proc \
                 would be created in a different namespace, changing what its \
                 unqualified calls and variables resolve to"
            ));
        }
        if registry
            .get(head)
            .is_some_and(|spec| spec.definition_body.is_some())
        {
            return Err(format!(
                "the selection is inside a '{head}' class/type definition body, \
                 where a top-level `proc` is not the same thing as a member"
            ));
        }
    }
    Ok(())
}

/// The innermost script region containing `offset`, as a byte range.
///
/// Descends through registry-resolved [`ArgRole::Body`] arguments, so a
/// selection inside a `proc` body, an `if` branch, or a `foreach` body is
/// scoped to that body rather than to the whole file — which is what makes
/// "is this variable read after the selection?" a question about the right
/// piece of code.
fn enclosing_scope(
    source: &str,
    offset: u32,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Scope {
    let mut scope = Scope {
        start: 0,
        end: u32::try_from(source.len()).unwrap_or(u32::MAX),
    };
    loop {
        let Some(text) = source.get(scope.start as usize..scope.end as usize) else {
            return scope;
        };
        let mut descended = false;
        for command in segment_commands_with_offset_and_config(text, scope.start, config) {
            let head = command.name();
            if head.is_empty() {
                continue;
            }
            let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
            for index in registry.arg_indices_for_role(head, &args, ArgRole::Body) {
                let Some(token) = command.argv.get(index + 1) else {
                    continue;
                };
                if token.kind != tcl_lexer::TokenType::Str {
                    continue;
                }
                let inner_start = token.span.start() + u32::from(token.content_offset);
                let inner_end = token.span.end();
                if inner_end > inner_start && offset >= inner_start && offset < inner_end {
                    scope = Scope {
                        start: inner_start,
                        end: inner_end,
                    };
                    descended = true;
                    break;
                }
            }
            if descended {
                break;
            }
        }
        if !descended {
            return scope;
        }
    }
}

/// The byte offset of the line start of the outermost top-level command
/// containing `offset` — where the extracted definition is inserted.
fn top_level_command_start(
    source: &str,
    scope_commands: &[SegmentedCommand],
    scope: Scope,
    offset: u32,
    config: LexerConfig,
) -> u32 {
    // A selection already at top level inserts before its own first command.
    let anchor = if scope.start == 0 {
        scope_commands
            .iter()
            .map(|command| command_span_offsets(source, command).0)
            .filter(|start| *start <= offset)
            .max()
            .unwrap_or(offset)
    } else {
        segment_commands_with_offset_and_config(source, 0, config)
            .into_iter()
            .filter(|command| {
                let (start, end) = command_span_offsets(source, command);
                start <= offset && offset < end
            })
            .map(|command| command_span_offsets(source, &command).0)
            .min()
            .unwrap_or(offset)
    };
    line_start_of(source, anchor)
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
) -> String {
    use std::fmt::Write as _;
    let mut params: Vec<String> = by_value.to_vec();
    params.extend(by_name.iter().map(|var| format!("{var}Name")));
    let mut body = String::new();
    for var in by_name {
        // The `$<var>Name` read is a *reference* and needs the brace form for a
        // non-bare name; the `upvar` target is a plain word, so it needs list
        // quoting instead (a name may carry an unbalanced brace — on 8.x
        // `${a{b}` names `a{b`).
        let _ = writeln!(
            body,
            "    upvar 1 {} {}",
            var_ref(&format!("{var}Name")),
            word(var)
        );
    }
    for line in block.lines() {
        let stripped = line.strip_prefix(block_indent).unwrap_or(line);
        if stripped.trim().is_empty() {
            body.push('\n');
        } else {
            let _ = writeln!(body, "    {}", stripped.trim_end());
        }
    }
    // The parameter list is a Tcl *list*, so each name goes in as a properly
    // quoted element rather than raw text.
    let param_list = tcl_syntax::list::join_list(params.iter());
    format!("proc {name} {{{param_list}}} {{\n{body}}}\n\n")
}

/// A `$` reference to `name`, braced unless the name can be written bare.
///
/// `${…}` is the spelling that survives every name this repository's scanners
/// can produce, on both release rules: the 9.x scanner ends the name at the
/// *matching* `}`, so `${a{b}c}` reads `a{b}c`, and the 8.x scanner ends it at
/// the first `}`, so `${a{b}` reads `a{b}`'s 8.x form `a{b`. Emitting the bare
/// `$a{b}c` instead would parse as `$a` followed by literal text on **both**
/// releases — the same mistake the minifier made (issue #1605, fifth site).
fn var_ref(name: &str) -> String {
    if tcl_syntax::naming::is_bare_var_name(name) {
        format!("${name}")
    } else {
        format!("${{{name}}}")
    }
}

/// `name` as a single command word, list-quoted when it cannot stand raw.
fn word(name: &str) -> String {
    tcl_syntax::list::join_list(std::iter::once(name))
}

/// Render the call that replaces the selection.
///
/// A by-value parameter is passed as `$var`; a by-name one is passed as the
/// *name* itself, which is what `upvar` in the proc binds against.
fn render_call(name: &str, by_value: &[String], by_name: &[String]) -> String {
    let mut words = vec![name.to_string()];
    words.extend(by_value.iter().map(|var| var_ref(var)));
    words.extend(by_name.iter().map(|var| word(var)));
    words.join(" ")
}

/// A proc name not already taken by a workspace symbol or a registry command.
///
/// The generated name is a placeholder the user renames straight away, but it
/// must not *shadow* something in the meantime: defining `proc lsort {…}` by
/// accident would silently replace the builtin for the rest of the file.
///
/// The comparison is against each proc's **qualified** name, not its simple
/// one.  The generated definition is written at the top level, so the name it
/// would occupy is `::<candidate>` — a `::app::extracted_proc` in some other
/// namespace is a different command and is no reason to pick a different
/// placeholder.  (A namespace-blind `proc_def.name == candidate` scan is also
/// the M1 drift class `cargo xtask resolution-drift` flags.)
fn unique_proc_name(analysis: &AnalysisResult, registry: &CommandRegistry) -> String {
    let taken = |candidate: &str| {
        let global = format!("::{candidate}");
        registry.get(candidate).is_some()
            || analysis
                .all_procs
                .values()
                .any(|proc_def| proc_def.qualified_name == global)
    };
    if !taken(BASE_NAME) {
        return BASE_NAME.to_string();
    }
    // Bounded rather than open-ended: the suffix search is over a placeholder
    // the user renames immediately, so a file already holding this many
    // `extracted_proc_N` procs needs a person to choose a name, not a longer
    // search.  Falling back to the base name keeps the refactoring available;
    // the rename that follows resolves the collision.
    (2..=MAX_NAME_SUFFIX)
        .map(|suffix| format!("{BASE_NAME}_{suffix}"))
        .find(|candidate| !taken(candidate))
        .unwrap_or_else(|| BASE_NAME.to_string())
}

/// Distinct `$name` / `${name}` references in `text`, array elements
/// reduced to the array's own name.
///
/// `style` is the document's `${…}` close rule: the captured-variable set
/// decides the generated proc's parameter list and its call-site arguments,
/// so reading a brace-bearing name by the wrong release's rule emits a proc
/// built for a variable that does not exist (issue #1605).
fn variable_references(text: &str, style: BracedVarStyle) -> BTreeSet<String> {
    super::variable_reference_spans(text, style)
        .into_iter()
        .map(|(name, _, _)| name)
        .collect()
}

/// The variable names `command` names through a registry `VarRead` /
/// `VarWrite` role — a use that carries no `$`, so the `$name` scan misses it.
fn role_named_variables(command: &SegmentedCommand, registry: &CommandRegistry) -> Vec<String> {
    let head = command.name();
    let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for role in [ArgRole::VarRead, ArgRole::VarWrite] {
        for index in registry.arg_indices_for_role(head, &args, role) {
            if let Some(name) = args.get(index) {
                out.push((*name).to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    /// Run the transform over the byte range of `needle` in `src`.
    fn at(src: &str, needle: &str) -> Option<Refactoring> {
        at_dialect(src, needle, "tcl9.0")
    }

    /// [`at`] against a document analysed under a named release — the
    /// `${…}` close rule is release-dependent (issue #1605).
    fn at_dialect(src: &str, needle: &str, dialect: &str) -> Option<Refactoring> {
        let registry = super::super::test_registry();
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(src, dialect).clone();
        let start = u32::try_from(src.find(needle).expect("needle in source")).unwrap();
        let end = start + u32::try_from(needle.len()).unwrap();
        extract_proc(src, (start, end), &analysis, &registry)
    }

    /// The rewritten document, or the refusal reason.
    fn outcome(src: &str, needle: &str) -> Result<String, String> {
        let refactoring = at(src, needle).expect("a selection to extract");
        match &refactoring.disabled {
            Some(reason) => Err(reason.clone()),
            None => Ok(refactoring.apply(src)),
        }
    }

    // -- TP: a pure selection with live-ins and no live-outs ---------------

    #[test]
    fn tp_extracts_a_read_only_selection_as_value_parameters() {
        let src = "set x 5\nputs $x\nputs done\n";
        let result = outcome(src, "puts $x").unwrap();
        assert!(
            result.contains("proc extracted_proc {x} {\n    puts $x\n}"),
            "{result}"
        );
        assert!(result.contains("extracted_proc $x"), "{result}");
    }

    /// Issue #1605 — the captured-variable set decides the generated proc's
    /// parameters and its call-site arguments, so a brace-bearing `${…}`
    /// name must be read by the **document's** release rule, not always the
    /// 8.x first-`}` one.
    ///
    /// Oracle: with `a{b}c` set to `NINE` and `a{b` set to `EIGHT`,
    /// `puts ${a{b}c}` prints `NINE` on tclsh 9.0.4 (one variable named
    /// `a{b}c`) and `EIGHTc}` on 8.6.16 (the name ends at the first `}` and
    /// `c}` is ordinary word text).
    ///
    /// Asserted on `variable_references` rather than through a whole
    /// extraction because the **segmenter**'s own `${…}` word span still
    /// truncates at the first `}` on a 9.x document (issue #1568, the
    /// compiled-word path — explicitly out of #1605's scope), so the block
    /// boundary an end-to-end extraction computes is wrong for a reason this
    /// change does not touch.
    #[test]
    fn variable_references_read_braced_names_by_the_documents_release() {
        let text = "puts ${a{b}c}";
        // 9.x: brace depth, so the whole `a{b}c` is the name.
        let nine = variable_references(text, BracedVarStyle::Tcl9Nesting);
        assert_eq!(
            nine.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["a{b}c"]
        );
        // 8.x: the name ends at the first `}`.
        let eight = variable_references(text, BracedVarStyle::FirstClose);
        assert_eq!(
            eight.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["a{b"]
        );
    }

    /// An unterminated `${` names nothing — inventing a name for it would
    /// put that name into the generated proc's parameter list.
    #[test]
    fn variable_references_reject_an_unterminated_braced_name() {
        assert!(variable_references("puts ${abc", BracedVarStyle::Tcl9Nesting).is_empty());
        assert!(variable_references("puts ${abc", BracedVarStyle::FirstClose).is_empty());
    }

    /// The **plumb**: the style must come from the document's own dialect,
    /// not a constant. The reference sits inside a braced body so the whole
    /// command's span is balanced and the segmenter hands over the complete
    /// `${a{b}c}` (issue #1568 only truncates a bare `${…}` word).
    ///
    /// Oracle: `puts ${a{b}c}` reads the variable `a{b}c` on tclsh 9.0.4 and
    /// `a{b` on 8.6.16, so the two releases capture different names — and
    /// the generated proc's parameter list differs accordingly.
    #[test]
    fn the_documents_dialect_decides_the_captured_braced_name() {
        let src = "if {1} {puts ${a{b}c}}\nputs done\n";
        let nine = at_dialect(src, "if {1} {puts ${a{b}c}}", "tcl9.0")
            .expect("a selection")
            .apply(src);
        assert!(
            nine.contains("proc extracted_proc {a{b}c} {"),
            "9.x must capture the whole name: {nine}"
        );
        let eight = at_dialect(src, "if {1} {puts ${a{b}c}}", "tcl8.6")
            .expect("a selection")
            .apply(src);
        // `a{b` carries an unbalanced brace, so it reaches the parameter list
        // as the quoted element tclsh itself writes (`list a{b` → `a\{b`).
        // Emitting it raw made `proc extracted_proc {a{b} {…}` — an unbalanced
        // brace that breaks the whole generated command.
        assert!(
            eight.contains(r"proc extracted_proc {a\{b} {"),
            "8.x must capture only `a{{b`, quoted: {eight}"
        );
    }

    #[test]
    fn render_call_braces_a_non_bare_captured_name() {
        // A bare-writable name keeps the bare form…
        assert_eq!(
            render_call("p", &["plain".to_string()], &[]),
            "p $plain",
            "a bare name needs no braces"
        );
        // …and one that cannot be written bare gets the `${…}` spelling.
        // `$a{b}c` would parse as `$a` followed by the literal `{b}c` on both
        // 8.6.16 and 9.0.4, so the call passed the wrong value (issue #1636
        // review).
        assert_eq!(
            render_call("p", &["a{b}c".to_string()], &[]),
            "p ${a{b}c}",
            "a brace-bearing name must be passed braced"
        );
        // A by-name argument is a word, not a reference, so it is list-quoted
        // rather than braced.
        assert_eq!(
            render_call("p", &[], &["a{b".to_string()]),
            r"p a\{b",
            "an unbalanced-brace name must reach the call as one word"
        );
        assert_eq!(
            render_call("p", &[], &["plain".to_string()]),
            "p plain",
            "a bare by-name argument stays raw"
        );
    }

    #[test]
    fn extracted_call_passes_a_brace_bearing_name_braced() {
        // End-to-end, with the reference inside a braced body so the command
        // span is balanced and the segmenter hands over the complete word
        // (the #1568 cap the PR documents).
        let src = "if {1} {puts ${a{b}c}}\nputs done\n";
        let out = at_dialect(src, "if {1} {puts ${a{b}c}}", "tcl9.0")
            .expect("a selection")
            .apply(src);
        assert!(
            out.contains("extracted_proc ${a{b}c}"),
            "the generated call must brace the name: {out}"
        );
        assert!(
            !out.contains("extracted_proc $a{b}c"),
            "the bare form parses as `$a` plus literal text: {out}"
        );
    }

    #[test]
    fn tp_a_write_never_read_again_becomes_a_proc_local() {
        // `tmp` is not used after the selection, so it is a genuine local:
        // it becomes a proc local and stops leaking into the caller at all.
        let src = "set tmp 1\nputs done\n";
        let result = outcome(src, "set tmp 1").unwrap();
        assert!(
            result.contains("proc extracted_proc {} {\n    set tmp 1\n}"),
            "{result}"
        );
        assert!(!result.contains("upvar"), "no upvar needed: {result}");
    }

    // -- TP: caller-frame writes survive via upvar -------------------------
    /// A name the selection reads *before* it writes is still an input. The
    /// list word of `foreach x $x` is evaluated before the loop binds `x`, so
    /// the caller has to hand that list over.
    ///
    /// Oracle (tclsh 8.6.18 and 9.0.4 alike): the original prints `a` then
    /// `b`; so does the extraction.  A proc with no `x` parameter dies with
    /// `can't read "x": no such variable`.
    #[test]
    fn tp_a_read_before_a_loop_binding_stays_a_value_parameter() {
        let src = "set x {a b}\nforeach x $x {\n    puts $x\n}\nputs done\n";
        let result = outcome(src, "foreach x $x {\n    puts $x\n}").unwrap();
        assert!(
            result.contains("proc extracted_proc {x} {"),
            "the list the loop iterates comes from the caller: {result}"
        );
        assert!(result.contains("extracted_proc $x\n"), "{result}");
    }

    /// The same rule without a loop: a command evaluates its arguments before
    /// its own write lands, so `$y` reads what the caller set.
    ///
    /// Oracle: the original leaves `y` at 2; so does the extraction.
    #[test]
    fn tp_a_read_in_the_writing_command_stays_a_value_parameter() {
        let src = "set y 1\nset y [expr {$y + 1}]\nputs done\n";
        let result = outcome(src, "set y [expr {$y + 1}]").unwrap();
        assert!(
            result.contains("proc extracted_proc {y} {"),
            "the operand is the caller's value: {result}"
        );
        assert!(result.contains("extracted_proc $y\n"), "{result}");
    }

    /// A `[…]` inside a *braced* expression argument is script the script
    /// lexer cannot see: `expr` substitutes it, in the caller's own frame. The
    /// write it makes therefore has to leave by name like any other.
    ///
    /// Oracle: the original prints `1`; so does the extraction.  Without the
    /// `upvar` the caller's `x` stays `0`.
    #[test]
    fn tp_a_write_inside_a_braced_expression_is_carried_by_upvar() {
        let src = "set x 0\nif {[set x 1]} {\n    puts hi\n}\nputs $x\n";
        let result = outcome(src, "if {[set x 1]} {\n    puts hi\n}").unwrap();
        assert!(
            result.contains("upvar 1 $xName x"),
            "the expression's substitution writes the caller's variable: {result}"
        );
        assert!(result.contains("extracted_proc x\n"), "{result}");
    }

    /// A selection made *inside* a loop body still writes the caller's
    /// variable: a control-flow body shares the frame it sits in, so the
    /// "read after the selection?" question runs past the body's closing
    /// brace to the frame that owns the variable.
    ///
    /// Oracle (tclsh 8.6.18 and 9.0.4 alike): original and extraction both
    /// print `6`.  Classifying `total` as a proc local prints `0`.
    #[test]
    fn tp_a_write_inside_a_loop_body_reaches_the_caller() {
        let src = "set total 0\nforeach n {1 2 3} {\n    incr total $n\n}\nputs $total\n";
        let result = outcome(src, "incr total $n").unwrap();
        assert!(
            result.contains("proc extracted_proc {n totalName} {\n    upvar 1 $totalName total\n"),
            "the loop body's write is the caller's write: {result}"
        );
        assert!(result.contains("extracted_proc $n total\n"), "{result}");
    }

    /// The enclosing body counts in full, not only the part after the
    /// selection: a loop runs again, so a read *above* the selection reads
    /// what the previous iteration assigned.
    ///
    /// Oracle: the original prints `0`, `1`, `3`; so does the extraction.
    #[test]
    fn tp_a_loop_carried_read_above_the_selection_keeps_the_upvar() {
        let src = "set total 0\nforeach n {1 2 3} {\n    puts $total\n    incr total $n\n}\n";
        let result = outcome(src, "incr total $n").unwrap();
        assert!(
            result.contains("upvar 1 $totalName total"),
            "the next iteration reads what this one wrote: {result}"
        );
        assert!(result.contains("extracted_proc $n total\n"), "{result}");
    }

    /// A `$name` inside a body that opens its own variable frame names *that*
    /// frame's variable, so it is neither a parameter of the extracted proc
    /// nor an argument at the call site — the caller has no such variable to
    /// pass.
    ///
    /// Oracle: the original defines `helper` and prints `done`; so does the
    /// extraction.  Asking the caller for `$name` dies with
    /// `can't read "name": no such variable`.
    #[test]
    fn tp_a_read_inside_a_nested_procs_body_is_not_a_parameter() {
        let src = "proc helper {name} {\n    puts \"hello $name\"\n}\nputs done\n";
        let result = outcome(src, "proc helper {name} {\n    puts \"hello $name\"\n}").unwrap();
        assert!(
            result.contains("proc extracted_proc {} {"),
            "the nested proc's parameter is nobody else's: {result}"
        );
        assert!(
            !result.contains("extracted_proc $name"),
            "the caller has no `name` to pass: {result}"
        );
    }

    /// A write nested in a control-flow body is still the caller's write.
    ///
    /// Classifying only the selection's *top-level* commands hides the `set
    /// total` inside the `foreach` body: `total` reads as never written and
    /// leaves as an ordinary value parameter whose assignment the caller
    /// never sees.  The loop variable is the mirror case — a `LoopVarList`
    /// binding is not a `VarWrite` — and would leave as a second value
    /// parameter, making the generated call read a `$n` the caller does not
    /// have.
    ///
    /// Oracle (tclsh 8.6.18 and 9.0.4 alike): the original prints `6` and so
    /// does the extraction below, while `proc extracted_proc {n total}` /
    /// `extracted_proc $n $total` dies with `can't read "n": no such
    /// variable`.
    #[test]
    fn tp_a_write_nested_in_a_loop_body_is_carried_by_upvar() {
        let src = "set total 0\nforeach n {1 2 3} {\n    set total [expr {$total + $n}]\n}\nputs $total\n";
        let result = outcome(
            src,
            "foreach n {1 2 3} {\n    set total [expr {$total + $n}]\n}",
        )
        .unwrap();
        assert!(
            result.contains("proc extracted_proc {totalName} {\n    upvar 1 $totalName total\n"),
            "the nested write must reach the caller's frame: {result}"
        );
        assert!(
            result.contains("extracted_proc total\n"),
            "the call passes the variable's *name*: {result}"
        );
        assert!(
            !result.contains("extracted_proc $n"),
            "the loop variable is bound by the loop, not supplied by the caller: {result}"
        );
    }

    /// The same nesting through a `switch` clause body, which is not an
    /// `ArgRole::Body` argument at all: its arms are reached through the
    /// registry's own `CaseListSpec`, so the walk covers them only because it
    /// asks the shared same-frame walker rather than descending braces itself.
    #[test]
    fn tp_a_write_nested_in_a_switch_arm_is_carried_by_upvar() {
        let src = "set total 0\nswitch x {\n    x {\n        set total 5\n    }\n}\nputs $total\n";
        let result = outcome(src, "switch x {\n    x {\n        set total 5\n    }\n}").unwrap();
        assert!(
            result.contains("upvar 1 $totalName total"),
            "a switch arm's write must reach the caller's frame: {result}"
        );
        assert!(result.contains("extracted_proc total\n"), "{result}");
    }

    /// A `dict for` binds its key and value names on every iteration, so they
    /// are the loop's own locals rather than parameters the caller supplies —
    /// while the `lappend` target nested in the body still leaves by name.
    ///
    /// Oracle: the extraction below prints `a b` on tclsh 8.6.18 and 9.0.4,
    /// as the original does.
    #[test]
    fn tp_a_loops_pair_bindings_are_locals_not_parameters() {
        let src =
            "set d {a 1 b 2}\nset out {}\ndict for {k v} $d {\n    lappend out $k\n}\nputs $out\n";
        let result = outcome(src, "dict for {k v} $d {\n    lappend out $k\n}").unwrap();
        assert!(
            result.contains("proc extracted_proc {d outName} {\n    upvar 1 $outName out\n"),
            "only the dictionary is a value parameter: {result}"
        );
        assert!(result.contains("extracted_proc $d out"), "{result}");
    }

    /// The nesting stops at a fresh variable frame.  A `proc` body inside the
    /// selection writes *that* proc's local, so classifying it as the
    /// selection's write would emit an `upvar` for a variable the moved code
    /// never touches.
    #[test]
    fn tp_a_nested_procs_own_writes_are_not_the_selections() {
        let src = "set inner 0\nproc helper {} {\n    set inner 1\n}\nputs $inner\n";
        let result = outcome(src, "proc helper {} {\n    set inner 1\n}").unwrap();
        assert!(
            result.contains("proc extracted_proc {} {"),
            "the nested proc's local is nobody's parameter: {result}"
        );
        assert!(
            !result.contains("upvar"),
            "no caller-frame write happened here: {result}"
        );
    }

    /// The mirror question — "is this variable read *after* the selection?" —
    /// is asked over the tail's statement tree too.  `incr total` carries no
    /// `$total` for the text scan to find, so a top-level-only tail scan
    /// would classify the write as a proc local and lose the caller's
    /// value.
    #[test]
    fn tp_a_role_named_read_nested_after_the_selection_keeps_the_upvar() {
        let src = "set total 0\nset total 5\nif {1} {\n    incr total\n}\n";
        let result = outcome(src, "set total 5").unwrap();
        assert!(
            result.contains("upvar 1 $totalName total"),
            "the later nested read makes `total` a live-out: {result}"
        );
        assert!(result.contains("extracted_proc total\n"), "{result}");
    }

    #[test]
    fn tp_a_write_read_after_the_selection_is_carried_by_upvar() {
        // The issue's reproducer.  With a value parameter the original
        // printed `after=1` and the refactored one printed `after=0`.
        let src = "set x 0\nset x 1\nputs $x\nputs \"after=$x\"\n";
        let result = outcome(src, "set x 1\nputs $x").unwrap();
        assert!(
            result.contains("upvar 1 $xName x"),
            "the write must reach the caller's frame: {result}"
        );
        assert!(
            result.contains("extracted_proc x\n"),
            "the call passes the variable's *name*: {result}"
        );
        assert!(result.contains("puts \"after=$x\""), "{result}");
    }

    #[test]
    fn tp_the_definition_lands_after_the_file_prologue() {
        // Not at line 0: a definition inserted above `package require` would
        // sit before the dialect/package setup it may depend on.
        let src = "package require Tcl\nset x 5\nputs $x\n";
        let result = outcome(src, "puts $x").unwrap();
        let package_line = result.find("package require Tcl").unwrap();
        let proc_line = result.find("proc extracted_proc").unwrap();
        assert!(package_line < proc_line, "{result}");
    }

    #[test]
    fn tp_the_generated_name_avoids_a_collision() {
        let src = "proc extracted_proc {} {}\nset x 5\nputs $x\n";
        let result = outcome(src, "puts $x").unwrap();
        assert!(result.contains("proc extracted_proc_2 {x}"), "{result}");
    }

    // -- FP/TN: refusals that keep behaviour -------------------------------

    #[test]
    fn fp_refuses_a_selection_containing_return() {
        let src = "proc f {} {\n    set x 1\n    return $x\n}\n";
        let reason = outcome(src, "return $x").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_selection_containing_break() {
        let src = "foreach i {1 2 3} {\n    break\n}\n";
        let reason = outcome(src, "break").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_selection_containing_continue() {
        let src = "foreach i {1 2 3} {\n    continue\n}\n";
        let reason = outcome(src, "continue").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_selection_containing_global() {
        let src = "proc f {} {\n    global config\n    puts $config\n}\n";
        let reason = outcome(src, "global config").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_selection_containing_upvar() {
        let src = "proc f {n} {\n    upvar 1 $n local\n}\n";
        let reason = outcome(src, "upvar 1 $n local").unwrap_err();
        assert!(reason.contains("call frame"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_computed_variable_write() {
        let src = "set n counter\nset $n 1\nputs $counter\n";
        let reason = outcome(src, "set $n 1").unwrap_err();
        assert!(reason.contains("computed"), "{reason}");
    }

    #[test]
    fn fp_refuses_an_array_element_write() {
        let src = "set a(x) 1\nputs $a(x)\n";
        let reason = outcome(src, "set a(x) 1").unwrap_err();
        assert!(reason.contains("array element"), "{reason}");
    }

    #[test]
    fn fp_refuses_a_selection_inside_a_namespace_eval() {
        let src = "namespace eval app {\n    set x 1\n    puts $x\n}\n";
        let reason = outcome(src, "puts $x").unwrap_err();
        assert!(reason.contains("namespace"), "{reason}");
    }

    #[test]
    fn tn_no_action_for_an_empty_selection() {
        let registry = super::super::test_registry();
        let mut analyser = Analyser::new();
        let src = "set x 1\n";
        let analysis = analyser.analyse(src, "tcl9.0").clone();
        assert!(extract_proc(src, (0, 0), &analysis, &registry).is_none());
    }

    #[test]
    fn tn_no_action_for_a_selection_covering_no_whole_command() {
        // Half a word is not a command: nothing is offered rather than
        // something wrong being offered.
        let src = "set x 1\n";
        assert!(at(src, "et x").is_none());
    }

    #[test]
    fn tn_no_action_for_a_whitespace_only_selection() {
        let src = "set x 1\n\n\nputs $x\n";
        assert!(at(src, "\n\n\n").is_none());
    }

    // -- Dialect-drift regression -------------------------------------------

    /// iRules' `}{` ghost word separator (no space between an `if`'s
    /// condition and its body — the idiom every real iRule uses) must be
    /// read under the document's own grammar, never the segmenter's
    /// default.  Under the default grammar `{$x}{ … }` reads as one
    /// *composite* word (`{cond}` welded to `{body}`), so `if` sees a single
    /// argument instead of the required condition + body pair and
    /// `enclosing_scope`'s `ArgRole::Body` descent cannot find a body to
    /// descend into at all; under `f5-irules` it splits at the `}{`
    /// boundary into the ordinary two-word `if {cond} {body}` shape.  If
    /// `extract_proc` ever again resolved its scope through a dialect-blind
    /// segmenter call, this selection would find no enclosing `if` body and
    /// the extraction would run against the whole `proc`'s scope instead —
    /// wrong, though not `None` (the top-level `proc` body still exists),
    /// so the two grammars must be told apart to catch the drift.
    #[test]
    fn extracts_an_irule_selection_across_the_brace_separator_idiom() {
        let snippet = "if {$x}{ set y 1 }";
        // Document the drift this test guards: the same text really does
        // segment differently under the two grammars.
        let default_cmds = tcl_compiler::segmenter::segment_commands(snippet);
        assert_eq!(
            default_cmds[0].argv.len(),
            2, // `if` + one composite `{$x}{ set y 1 }` argument
            "default grammar must weld `}}{{` into one word: {:?}",
            default_cmds[0].texts
        );
        let irules_cmds = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            snippet,
            0,
            tcl_lexer::LexerConfig::for_dialect("f5-irules"),
        );
        assert_eq!(
            irules_cmds[0].argv.len(),
            3, // `if` + condition + body, split at `}{`
            "iRules grammar must split `}}{{` into two words: {:?}",
            irules_cmds[0].texts
        );

        // The end-to-end refactor: a selection inside the if-body only
        // resolves to that body's scope when `extract_proc` threads the
        // document's own (iRules) grammar into its own segmentation.
        let src = "proc handler {} {\n    if {$x}{\n        set y 1\n        set y 2\n    }\n    puts done\n}\n";
        let result = at_dialect(src, "set y 1\n        set y 2", "f5-irules").expect("a selection");
        assert!(
            result.disabled.is_none(),
            "must not decline: {:?}",
            result.disabled
        );
        let applied = result.apply(src);
        assert!(
            applied.contains("proc extracted_proc {} {\n    set y 1\n    set y 2\n}"),
            "the extracted proc must be exactly the if-body's two commands: {applied}"
        );
        assert!(
            applied.contains("if {$x}{\n        extracted_proc\n    }"),
            "the call must replace only the if-body's contents, in place: {applied}"
        );
    }

    // -- Unit-level helpers ------------------------------------------------

    #[test]
    fn variable_references_collects_bare_and_braced_names() {
        let found = variable_references("puts $a ${b} $c_1", BracedVarStyle::Tcl9Nesting);
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            vec!["a".to_string(), "b".to_string(), "c_1".to_string()]
        );
    }
}
