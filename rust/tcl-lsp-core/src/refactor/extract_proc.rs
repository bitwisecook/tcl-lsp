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
//! own [`ArgRole::VarWrite`] / [`ArgRole::VarRead`] roles plus the `$name`
//! references in the text — never a keyword list:
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

use std::collections::BTreeSet;

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
    let roles = classify_variables(source, selected, registry, style)?;

    // The selection's own byte range, snapped to the commands it covers.
    let block_start = command_span_offsets(source, selected[0]).0;
    let block_end = command_span_offsets(source, selected[selected.len() - 1]).1;
    let block = source
        .get(block_start as usize..block_end as usize)
        .ok_or_else(|| "the selected range is unreadable".to_string())?;

    // A written variable that is read again after the selection must keep
    // reaching the caller's frame; one that is not becomes a proc local.
    let tail = source
        .get(block_end as usize..scope.end as usize)
        .unwrap_or("");
    let tail_commands: Vec<SegmentedCommand> =
        segment_commands_with_offset_and_config(tail, 0, config)
            .into_iter()
            .filter(|command| !command.name().is_empty())
            .collect();
    let mut used_after: BTreeSet<String> = variable_references(tail, style);
    for command in &tail_commands {
        used_after.extend(role_named_variables(command, registry));
    }

    let by_name: Vec<String> = roles
        .written
        .iter()
        .filter(|name| used_after.contains(*name))
        .cloned()
        .collect();
    let by_value: Vec<String> = roles
        .read
        .iter()
        .filter(|name| !roles.written.contains(*name))
        .cloned()
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

/// The variables the selection reads and writes.
struct VariableRoles {
    read: BTreeSet<String>,
    written: BTreeSet<String>,
}

/// Classify the selection's variable use from the registry's argument roles
/// plus the `$name` references in its text.
///
/// The write set comes from [`ArgRole::VarWrite`], so `set`, `incr`,
/// `lappend`, `append`, `dict set`, `lassign`, and anything else a spec
/// declares are all covered without this module naming any of them.
fn classify_variables(
    source: &str,
    selected: &[&SegmentedCommand],
    registry: &CommandRegistry,
    style: BracedVarStyle,
) -> Result<VariableRoles, String> {
    let mut read = BTreeSet::new();
    let mut written = BTreeSet::new();
    for command in selected {
        let (start, end) = command_span_offsets(source, command);
        read.extend(variable_references(
            source.get(start as usize..end as usize).unwrap_or(""),
            style,
        ));
        let head = command.name();
        if head.contains('$') || head.contains('[') || head.contains('{') {
            return Err(format!(
                "the command head `{head}` is computed at run time, so which \
                 variables the command reads and writes is unknown"
            ));
        }
        let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
        for index in registry.arg_indices_for_role(head, &args, ArgRole::VarRead) {
            if let Some(name) = args.get(index) {
                read.insert((*name).to_string());
            }
        }
        for index in registry.arg_indices_for_role(head, &args, ArgRole::VarWrite) {
            let Some(name) = args.get(index) else {
                continue;
            };
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
            written.insert((*name).to_string());
        }
    }
    Ok(VariableRoles { read, written })
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
