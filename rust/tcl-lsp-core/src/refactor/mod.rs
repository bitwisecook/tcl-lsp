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

//! Mechanical refactoring transforms.
//!
//! Each transform accepts source text plus a cursor (or selection)
//! and returns a [`Refactoring`] describing a titled set of text
//! edits, or `None` when the transform does not apply.  The
//! [`code_actions`](crate::code_actions) provider lifts each
//! [`Refactoring`] into a `CodeAction`.
//!
//! Transforms:
//!
//! * [`extract_proc`] — move a run of commands into a new proc and call
//!   it, carrying caller-frame writes through with `upvar`.
//! * [`extract_variable`] — replace a selected expression with a named
//!   `set` assignment.
//! * [`inline_variable`] — inline a single-use `set` variable.
//! * [`inline_proc`] — replace a call with the proc's body, with its
//!   parameters bound to the call's argument *values*.
//! * [`if_to_switch`] — convert an `if`/`elseif` equality chain to a
//!   `switch`.
//! * [`switch_to_dict`] — convert a constant-mapping `switch` to a
//!   `dict` lookup.
//! * [`extract_to_datagroup`] — convert inline if/switch membership /
//!   mapping patterns to iRules `class match` / `class lookup` against
//!   a generated data-group.
//!
//! ## Coordinate convention
//!
//! Cursors and edit ranges use LSP `(line, character)` coordinates with
//! UTF-16 character columns, matching the existing `extract_proc` /
//! `inline_proc` transforms in [`crate::code_actions`].  Internally a
//! transform works in **byte offsets** (so it can slice `&str` directly)
//! and converts to UTF-16 [`LspRange`] only when building the final
//! edits, via the document's [`LineIndex`].

mod brace_expr;
mod datagroup;
mod extract_proc;
mod extract_variable;
mod if_to_switch;
mod inline_proc;
mod inline_variable;
mod switch_to_dict;

pub use brace_expr::brace_expr;
pub use datagroup::{
    DataGroupDefinition, data_group_tcl, extract_to_datagroup, extract_to_datagroup_from_if,
    extract_to_datagroup_from_switch,
};
pub use extract_proc::{extract_proc, extract_proc_rename_command};
pub use extract_variable::extract_variable;
pub use if_to_switch::if_to_switch;
pub use inline_proc::{inline_proc, inline_proc_in_program};
pub use inline_variable::inline_variable;
pub use switch_to_dict::switch_to_dict;

use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset};
use tcl_dialect::BracedVarStyle;
use tcl_lexer::{LineIndex, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};
use tcl_syntax::switch_body::parse_braced_pairs;

use crate::definition::LspRange;

/// The `${…}` close rule the document's dialect uses.
///
/// A refactor rewrites *text*, so reading a variable name by one release's
/// rule while the document is another's changes what the edit means: on a
/// Tcl 9 document `${a{b}c}` is one variable named `a{b}c`, but the 8.x
/// first-`}` rule reads `a{b` and leaves `c}` looking like ordinary word
/// text (issue #1605).
pub(crate) fn braced_var_style(
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> BracedVarStyle {
    crate::profile_for_dialect(&analysis.dialect)
        .grammar
        .braced_var
}

/// Every `$name` / `${name}` reference in `text`, as
/// `(name, start, end)` byte offsets covering the whole reference
/// (`$` through the closing `}`).
///
/// The braced form is delimited by [`tcl_lexer::braced_var_name_end`] — the
/// one release-aware `Tcl_ParseVarName` port — rather than a `find('}')`,
/// so a brace-bearing name is read the way the document's own release reads
/// it. An unterminated `${` is not a reference: it is a parse error in the
/// document, and inventing a name for it would put that name into a
/// generated parameter list.
///
/// Array elements are **not** reduced here; a caller that wants the array's
/// own name (extract-proc's captured set) trims at the `(`.
pub(crate) fn variable_reference_spans(
    text: &str,
    style: BracedVarStyle,
) -> Vec<(String, usize, usize)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        if bytes.get(index + 1) == Some(&b'{') {
            let name_start = index + 2;
            if let tcl_lexer::BracedVarEnd::Closed(close) =
                tcl_lexer::braced_var_name_end(bytes, name_start, style)
            {
                out.push((text[name_start..close].to_string(), index, close + 1));
                index = close + 1;
                continue;
            }
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b':')
        {
            end += 1;
        }
        if end > index + 1 {
            out.push((text[index + 1..end].to_string(), index, end));
            index = end;
        } else {
            index += 1;
        }
    }
    out
}

/// A single text replacement, in byte-offset space.
///
/// The transform layer composes edits in byte offsets and only converts
/// to LSP [`LspRange`] coordinates at the boundary, via
/// [`RefactorEdit::to_lsp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefactorEdit {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
    /// Replacement text.
    pub new_text: String,
}

impl RefactorEdit {
    /// Convert this byte-offset edit to an LSP `(line, character)` edit
    /// with UTF-16 character columns.
    #[must_use]
    pub fn to_lsp(&self, source: &str, line_index: &LineIndex) -> crate::rename::TextEdit {
        let start = line_index.position_at_utf16(self.start, source);
        let end = line_index.position_at_utf16(self.end, source);
        crate::rename::TextEdit {
            range: LspRange {
                start_line: start.line,
                start_character: start.character.get(),
                end_line: end.line,
                end_character: end.character.get(),
            },
            new_text: self.new_text.clone(),
        }
    }
}

/// The outcome of a refactoring transform — a titled set of edits plus
/// the LSP code-action kind.
///
/// `data_group` carries the rendered tmsh definition for the
/// extract-to-datagroup transform and is `None` for every other
/// transform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refactoring {
    /// Title shown in the editor.
    pub title: String,
    /// Edits, in byte-offset space.  Empty when [`Self::disabled`] is set —
    /// a refused refactoring has nothing to apply.
    pub edits: Vec<RefactorEdit>,
    /// LSP code-action kind dotted string (`refactor.extract`, …).
    pub kind: crate::code_actions::ActionKind,
    /// Generated data-group definition, when this is an
    /// extract-to-datagroup result.
    pub data_group: Option<DataGroupDefinition>,
    /// Why this refactoring cannot be applied here, when it cannot.
    ///
    /// A transform that finds its subject but *cannot preserve behaviour* says
    /// so rather than vanishing: the reason is surfaced as the code action's
    /// LSP `disabled.reason`, so the editor greys the entry out and explains
    /// itself.  Silently offering nothing leaves a user unable to tell "this
    /// refactoring does not apply here" from "this refactoring is broken".
    pub disabled: Option<String>,
}

impl Refactoring {
    /// Apply the edits to `source` bottom-to-top, returning the
    /// rewritten text.  Edits are sorted by descending start offset so
    /// an earlier edit never shifts a later one's coordinates.
    ///
    /// The sort key is `(start, end)`, not `start` alone: an insertion and a
    /// replacement can legitimately share a start offset — extract-proc puts
    /// the new definition at the line the selection begins on and replaces
    /// that selection with a call — and ordering by start alone leaves which
    /// of the two lands first up to the sort's stability, interleaving the two
    /// texts.  Applying the wider edit first is the only order that composes.
    #[must_use]
    pub fn apply(&self, source: &str) -> String {
        let mut sorted: Vec<&RefactorEdit> = self.edits.iter().collect();
        sorted.sort_by_key(|e| std::cmp::Reverse((e.start, e.end)));
        let mut result = source.to_owned();
        for edit in sorted {
            let start = (edit.start as usize).min(result.len());
            let end = (edit.end as usize).min(result.len()).max(start);
            result.replace_range(start..end, &edit.new_text);
        }
        result
    }
}

/// Exclusive end byte offset for `tok`, including its closing delimiter.
///
/// Lexer token spans follow the
/// inner-end convention — a non-empty `{…}` / `[…]` / `"…"` word's
/// `span.end()` is the exclusive offset of the closer, so the closer
/// sits one byte past the end and a whole-word span must widen by one.
/// An empty `{}` / `[]` / `""` already has its span extended over the
/// closer, so widening would overshoot; the discriminator is whether the
/// token text is empty (the closer is already covered), exactly as the
/// segmenter's `widen_word_end`.
#[must_use]
pub fn token_end_offset(source: &str, tok: Token) -> u32 {
    let bytes = source.as_bytes();
    let start = tok.span.start() as usize;
    let end = tok.span.end();
    // The closer is derived from the *opening* delimiter the token's
    // source slice begins with — a braced (`STR`) word opens with `{`, a
    // command substitution (`CMD`) with `[`, and a quoted word (an `ESC`
    // word whose first source byte is `"`) with `"`.  `in_quote` marks an
    // *interpolated* fragment inside quotes, not a whole quoted word, so
    // the opening byte is the reliable discriminator.
    let closer = match tok.kind {
        TokenType::Str => b'}',
        TokenType::Cmd => b']',
        TokenType::Esc if bytes.get(start) == Some(&b'"') => b'"',
        _ => return end.min(byte_len(source)),
    };
    // Detect the degenerate empty word (`{}` / `[]` / `""`), whose span
    // the lexer already extended over its closer — widening again would
    // overshoot (`a {}}`).  Mirror `SourceMap::token_text`'s
    // emptiness rule: strip the opener via `content_offset`, then treat a
    // remainder of `""`, `"}"`, or `"]"` as the empty case.
    let raw = source.get(start..end as usize).unwrap_or("");
    let stripped = raw.get(tok.content_offset as usize..).unwrap_or("");
    if stripped.is_empty() || stripped == "}" || stripped == "]" {
        return end.min(byte_len(source));
    }
    if bytes.get(end as usize) == Some(&closer) {
        (end + 1).min(byte_len(source))
    } else {
        end.min(byte_len(source))
    }
}

fn byte_len(source: &str) -> u32 {
    u32::try_from(source.len()).unwrap_or(u32::MAX)
}

/// `(start, end)` byte offsets covering the full command text.
///
/// The segmenter's `span` already widens for a
/// trailing brace / bracket but deliberately **not** for a trailing
/// quote (the segmenter keeps the inner-end for `"…"` words), so widen
/// the end here to the final argv word's [`token_end_offset`] — this
/// covers the closing `"` of a command like `set name "hello"`.
#[must_use]
pub fn command_span_offsets(source: &str, cmd: &SegmentedCommand) -> (u32, u32) {
    let len = byte_len(source);
    let mut end = cmd.span.end();
    if let Some(&last) = cmd.argv.last() {
        end = end.max(token_end_offset(source, last));
    }
    (cmd.span.start().min(len), end.min(len))
}

/// Recursively find the innermost command at byte offset `cursor`.
///
/// Walks into registry-resolved
/// `ArgRole::Body` arguments (proc / when / if / while / foreach
/// bodies, …) so transforms apply inside nested bodies; the innermost
/// match wins.  When `predicate` is `Some(name)`, only a command whose
/// first word equals `name` is returned.
///
/// Body words are re-segmented at their absolute offset
/// ([`segment_commands_with_offset`]) so the returned command's spans
/// are always in the outer source buffer's offset space — the caller can
/// slice `source` with them directly.
#[must_use]
pub fn find_command_at(
    source: &str,
    cursor: u32,
    predicate: Option<&str>,
    registry: &CommandRegistry,
) -> Option<SegmentedCommand> {
    find_command_at_inner(source, cursor, predicate, registry, 0)
}

/// Recursively walk every command in `source`, including those nested inside
/// registry-resolved body arguments (proc / when / if / while / foreach
/// bodies, …). Returns `(texts, line, character)` per command — `texts` is the
/// command's words (name first), `(line, character)` is the 0-based
/// (UTF-16-counted) position of its start.
#[must_use]
pub fn walk_commands(source: &str, registry: &CommandRegistry) -> Vec<(Vec<String>, u32, u32)> {
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    walk_commands_inner(source, source, 0, registry, &line_index, 0, &mut out);
    out
}

fn walk_commands_inner(
    full: &str,
    slice: &str,
    offset: u32,
    registry: &CommandRegistry,
    line_index: &LineIndex,
    depth: u32,
    out: &mut Vec<(Vec<String>, u32, u32)>,
) {
    if MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
        return;
    }
    for cmd in segment_commands_with_offset(slice, offset) {
        let pos = line_index.position_at_utf16(cmd.span.start(), full);
        out.push((cmd.texts.clone(), pos.line, pos.character.get()));
        // Spans here are absolute into `full` (the segmenter was given
        // `offset`), so the lambda splitter reads from `full` too.
        for body in body_words(full, &cmd, registry) {
            let Some(body_slice) = full.get(body.inner_start as usize..body.inner_end as usize)
            else {
                continue;
            };
            walk_commands_inner(
                full,
                body_slice,
                body.inner_start,
                registry,
                line_index,
                depth + 1,
                out,
            );
        }
    }
}

/// Defensive recursion bound for the nested-command search, set to match the
/// compiler analyser's `MAX_BODY_DEPTH` so deeply (but validly) nested code
/// still resolves the command under the cursor. Real source never nests
/// anywhere near this.
const MAX_COMMAND_SEARCH_DEPTH: tcl_core_types::RecursionLimit =
    tcl_core_types::RecursionLimit(256);

fn find_command_at_inner(
    source: &str,
    cursor: u32,
    predicate: Option<&str>,
    registry: &CommandRegistry,
    depth: u32,
) -> Option<SegmentedCommand> {
    if MAX_COMMAND_SEARCH_DEPTH.exceeded(depth) {
        return None;
    }
    for cmd in segment_commands_with_offset(source, 0) {
        let (cmd_start, cmd_end) = command_span_offsets(source, &cmd);
        if cursor < cmd_start || cursor > cmd_end {
            continue;
        }
        // Recurse into body arguments first — the innermost match wins.
        for body in body_words(source, &cmd, registry) {
            if let Some(inner) = find_command_at_inner(
                &source[body.inner_start as usize..body.inner_end as usize],
                cursor.saturating_sub(body.inner_start),
                predicate,
                registry,
                depth + 1,
            ) {
                // Relocate the inner command's spans back into `source`.
                return Some(inner.shifted_by(body.inner_start));
            }
        }
        if predicate.is_none_or(|name| cmd.name() == name) {
            return Some(cmd);
        }
    }
    None
}

/// A registry-resolved body word and the byte range of its interior
/// (delimiters excluded) in the outer source buffer.
struct BodyWord {
    inner_start: u32,
    inner_end: u32,
}

/// Yield the script-bearing words of `cmd` that [`find_command_at_inner`]
/// may descend into, as interior byte ranges (delimiters excluded) in the
/// outer source buffer.
///
/// Two registry roles qualify, both resolved from the spec — never from a
/// command name:
///
/// * [`ArgRole::Body`] — a plain braced script; the whole interior is code.
/// * [`ArgRole::LambdaLiteral`] — `apply`'s `{argList body ?ns?}` two- or
///   three-element list, where only element 1 is code.  It is split with
///   [`tcl_compiler::lambda_literal::split_lambda_literal`], the same
///   splitter every other consumer uses (issue #1000).  Treating
///   the whole literal as one body instead reads `argList` as a command
///   name and swallows the real body, which is why no refactor code action
///   fired inside an `apply` lambda.  Only a `{braced}` body element is
///   descended
///   ([`LambdaLiteralElements::braced_body`](tcl_compiler::lambda_literal::LambdaLiteralElements::braced_body)):
///   a bare / double-quoted one is backslash-decoded before `apply`
///   evaluates it, so its source slice is not the script that runs and the
///   spans a code action derived from it would edit the wrong bytes (Codex
///   review on #1047).
fn body_words(source: &str, cmd: &SegmentedCommand, registry: &CommandRegistry) -> Vec<BodyWord> {
    let name = cmd.name();
    if name.is_empty() {
        return Vec::new();
    }
    let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for idx in registry.arg_indices_for_role(name, &args, ArgRole::Body) {
        // `arg_indices_for_role` is 0-based over the post-name args; the
        // representative argv token sits at `idx + 1`.
        let Some(&tok) = cmd.argv.get(idx + 1) else {
            continue;
        };
        if tok.kind != TokenType::Str {
            continue;
        }
        // The STR token's span starts on the opening `{` and ends before
        // the closing `}` (inner-end convention).  The descended interior
        // is one byte past the `{` up to the span end.
        let inner_start = tok.span.start() + u32::from(tok.content_offset);
        let inner_end = tok.span.end();
        push_body_word(&mut out, inner_start, inner_end);
    }
    for idx in registry.arg_indices_for_role(name, &args, ArgRole::LambdaLiteral) {
        let Some(&tok) = cmd.argv.get(idx + 1) else {
            continue;
        };
        // A `$var` / `[cmd]`-computed lambda can't be split statically —
        // the guard every `LambdaLiteral` consumer applies.
        if tok.kind != TokenType::Str {
            continue;
        }
        let Some(body) = tcl_compiler::lambda_literal::split_lambda_literal(source, tok)
            .and_then(|elems| elems.braced_body())
        else {
            continue;
        };
        push_body_word(&mut out, body.start(), body.end());
    }
    out
}

/// Record a descendable interior range, skipping an empty one (nothing to
/// descend into).
fn push_body_word(out: &mut Vec<BodyWord>, inner_start: u32, inner_end: u32) {
    if inner_end > inner_start {
        out.push(BodyWord {
            inner_start,
            inner_end,
        });
    }
}

/// Parse a `switch` command's flags + subject + pattern/body pairs.
///
/// Shared by [`switch_to_dict`] and the data-group switch transform.
/// Returns `(subject, pairs)` only for an `-exact` switch (the only mode
/// either transform handles); any other mode — `-glob` / `-regexp` —
/// yields `None`.  The remaining args are the pattern-body pairs, given
/// either as a single braced list (`{pat1 {body1} …}`) or as separate
/// words.  Parses the flag / subject / pairs prologue shared by both
/// transforms.
#[must_use]
pub fn parse_exact_switch(texts: &[String]) -> Option<(String, Vec<(String, String)>)> {
    let mut i = 1;
    let mut mode = "exact";
    while i < texts.len() && texts[i].starts_with('-') {
        match texts[i].as_str() {
            "-exact" => mode = "exact",
            "-glob" => mode = "glob",
            "-regexp" => mode = "regexp",
            "--" => {
                i += 1;
                break;
            }
            _ => {}
        }
        i += 1;
    }
    if mode != "exact" || i >= texts.len() {
        return None;
    }
    let subject = texts[i].clone();
    i += 1;

    let pairs: Vec<(String, String)> = if i + 1 == texts.len() {
        parse_braced_pairs(&texts[i])
    } else {
        let mut p = Vec::new();
        while i + 1 < texts.len() {
            p.push((texts[i].clone(), texts[i + 1].clone()));
            i += 2;
        }
        p
    };
    Some((subject, pairs))
}

/// Leading-whitespace indent of `line`.
#[must_use]
pub fn line_indent(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Re-indent a (possibly braced) body to `target_indent`.
///
/// Used by `if_to_switch` and `extract_datagroup`: strips one layer
/// of surrounding braces, computes
/// the body's minimum indent, and re-emits each non-blank line at
/// `target_indent` (preserving interior blank lines).  An empty body
/// becomes `"{target_indent}# empty"`.
#[must_use]
pub fn reindent_body(body: &str, target_indent: &str) -> String {
    let mut text = body.trim();
    if text.starts_with('{') && text.ends_with('}') && text.len() >= 2 {
        text = text[1..text.len() - 1].trim();
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min();
    let Some(min_indent) = min_indent else {
        return format!("{target_indent}# empty");
    };
    let mut result: Vec<String> = Vec::new();
    for ln in &lines {
        if ln.trim().is_empty() {
            if !result.is_empty() {
                result.push(String::new());
            }
        } else {
            result.push(format!("{target_indent}{}", &ln[min_indent..]));
        }
    }
    result.join("\n")
}

/// Strip one layer of surrounding double quotes, if present.
#[must_use]
pub fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

#[cfg(test)]
pub(crate) fn test_registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_is_bottom_to_top() {
        let r = Refactoring {
            title: "t".into(),
            edits: vec![
                RefactorEdit {
                    start: 6,
                    end: 8,
                    new_text: "99".into(),
                },
                RefactorEdit {
                    start: 0,
                    end: 3,
                    new_text: "let".into(),
                },
            ],
            kind: crate::code_actions::ActionKind::RefactorRewrite,
            data_group: None,
            disabled: None,
        };
        assert_eq!(r.apply("set x 42"), "let x 99");
    }

    #[test]
    fn find_command_descends_into_proc_body() {
        let reg = test_registry();
        let src = "proc handler {m} {\n    if {$m eq \"GET\"} { go }\n}";
        // Cursor on the inner `if` (line 1).
        let cursor = u32::try_from(src.find("if {").unwrap()).unwrap() + 1;
        let cmd = find_command_at(src, cursor, Some("if"), &reg).expect("inner if");
        assert_eq!(cmd.name(), "if");
        // The returned span is in the outer buffer's offsets.
        let (s, _e) = command_span_offsets(src, &cmd);
        assert!(src[s as usize..].starts_with("if {$m"));
    }

    /// Parity with [`find_command_descends_into_proc_body`] for `apply`'s
    /// lambda literal (issue #1000).  `apply`'s first argument is
    /// `ArgRole::LambdaLiteral`, not `Body`: the whole `{argList body}`
    /// blob is not a script, so descending into it verbatim reads
    /// `argList` as a command name and swallows the real body.  Splitting
    /// it reaches the body element, and the `if` inside is found exactly
    /// as it is inside a `proc`.  tclsh8.6/9.0-verified that this lambda
    /// really does run that `if` (`get` / `post` / `other`).
    #[test]
    fn find_command_descends_into_apply_lambda_body() {
        let reg = test_registry();
        let src = "proc handler {} {\n    apply {{m} {\n        if {$m eq \"GET\"} {\n            puts get\n        } elseif {$m eq \"POST\"} {\n            puts post\n        } else {\n            puts other\n        }\n    }} $x\n}\n";
        // Cursor on the `if` inside the lambda body.
        let cursor = u32::try_from(src.find("if {$m").unwrap()).unwrap() + 1;
        let cmd = find_command_at(src, cursor, Some("if"), &reg).expect("if inside apply lambda");
        assert_eq!(cmd.name(), "if");
        // The returned span is in the outer buffer's offsets.
        let (start, _end) = command_span_offsets(src, &cmd);
        assert!(
            src[start as usize..].starts_with("if {$m"),
            "span must relocate back into the outer buffer: {:?}",
            &src[start as usize..]
        );
    }

    /// TN: the lambda's *argument-list* element is a plain word list, not
    /// code.  A cursor on it must not produce a command match — descending
    /// into it is exactly the mis-read the split avoids.
    #[test]
    fn find_command_does_not_match_inside_the_lambda_arg_list() {
        let reg = test_registry();
        let src = "apply {{if} {\n    puts $if\n}} 1\n";
        // Cursor on the `if` *parameter name* in the argument list.
        let cursor = u32::try_from(src.find("{if}").unwrap()).unwrap() + 1;
        assert!(
            find_command_at(src, cursor, Some("if"), &reg).is_none(),
            "a parameter word named `if` is not an `if` command"
        );
    }

    #[test]
    fn reindent_strips_braces_and_normalises() {
        let out = reindent_body("{\n        puts hi\n    }", "    ");
        assert_eq!(out, "    puts hi");
    }
}
