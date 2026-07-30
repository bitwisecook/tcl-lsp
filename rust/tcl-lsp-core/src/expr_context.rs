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

//! "Is the cursor inside an `expr` expression, and if so on a math-function
//! call?" — the one place every provider (hover, go-to-definition,
//! completion, and through [`crate::definition::resolve_proc_target_at`] also
//! references / rename / call-hierarchy / linked-editing) asks that question.
//!
//! Two distinct mechanisms, because the two callers have different inputs:
//!
//! * [`mathfunc_call_at`] — for a cursor in **complete** source. It reads the
//!   analyser's own record of the expression: `record_expr_function_invocations`
//!   already walks every [`ArgRole::Expr`] argument, parses the expression
//!   grammar, and pushes one [`SignatureCommandInvocation`] per `NAME(` call
//!   with `is_mathfunc_call = true`, the bare word's exact span, and a
//!   `resolved_qualified_name` already settled by
//!   `finalise_invocation_resolutions` against C Tcl's two-candidate mathfunc
//!   rule (the caller's own `tcl::mathfunc`, honouring its `namespace path`,
//!   else the global one). Nothing here re-sniffs text or re-derives a
//!   namespace: the analyser is the authority, this is a span lookup over its
//!   output.
//!
//! * [`expr_arg_context_at`] — for **completion**, where the source is
//!   mid-typing (`set a [expr {si`) and therefore unbalanced, so no
//!   invocation record exists yet and the segmenter has nothing to segment.
//!   It walks the byte prefix before the cursor to find the innermost
//!   enclosing command's word list, then asks the **registry** whether the
//!   cursor's word carries [`ArgRole::Expr`]. No command name appears here —
//!   `expr` / `if` / `while` / `for` / an iRules `when` condition are all just
//!   whatever `arg_indices_for_role` says.
//!
//! The bare ⇄ `::tcl::mathfunc::NAME` mapping, and both availability axes,
//! live in [`tcl_registry::mathfunc`] — see its module docs for the C Tcl
//! oracle behind the resolution order.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::signature_scan::types::SignatureCommandInvocation;
use tcl_registry::{ArgRole, CommandRegistry};

/// The analyser-recorded `expr` math-function call whose **name word** covers
/// `cursor_off`, or `None` when the cursor is not on one.
///
/// This *is* the expr-context test for a cursor in complete source: an
/// invocation with `is_mathfunc_call` exists only because the analyser
/// recognised an `ArgRole::Expr` argument and parsed a function-call
/// production inside it. A `[…]` substitution nested in an expression
/// re-enters script context and produces ordinary (non-mathfunc)
/// invocations, so `expr {[sin 1]}` correctly does not match here.
pub(crate) fn mathfunc_call_at(
    analysis: &AnalysisResult,
    cursor_off: u32,
) -> Option<&SignatureCommandInvocation> {
    analysis.command_invocations.iter().find(|inv| {
        inv.is_mathfunc_call && inv.range.start() <= cursor_off && cursor_off < inv.range.end()
    })
}

/// What an `expr` math-function call at the cursor resolves to.
#[derive(Debug, Clone, Copy)]
pub(crate) enum MathFuncTarget<'a> {
    /// A user-defined override — `proc ::tcl::mathfunc::f` (global) or a
    /// namespace-local `namespace eval ns { namespace eval tcl::mathfunc {
    /// proc f … } }`. Always wins: it is a real command, installed in the
    /// table C Tcl dispatches through.
    UserProc(&'a tcl_compiler::analyser::ProcDef),
    /// The built-in function, rendered from its registry spec.
    Builtin {
        /// The bare function word as written (`sin`).
        bare: &'a str,
        /// The qualified command name it dispatches to.
        qualified: &'a str,
        /// The registry spec carrying the hover / synopsis data.
        spec: &'a tcl_registry::CommandSpec,
    },
}

/// Resolve the math-function call at `cursor_off` to its target, applying C
/// Tcl's own precedence: the resolved command-table entry first (a user
/// `proc` in the winning `tcl::mathfunc` namespace), else the built-in the
/// registry has data for under `profile`.
///
/// `None` when the cursor is not on a math-function call, or when it is but
/// the function does not exist in this dialect *and* no user proc provides it
/// — the deliberate abstention that keeps `expr {gamma(1)}` under 9.0 from
/// rendering a 9.1-only function.
pub(crate) fn math_function_target_at<'a>(
    analysis: &'a AnalysisResult,
    registry: Option<&'a CommandRegistry>,
    profile: &'static tcl_dialect::DialectProfile,
    cursor_off: u32,
) -> Option<MathFuncTarget<'a>> {
    let inv = mathfunc_call_at(analysis, cursor_off)?;
    let resolved = inv.resolved_qualified_name.as_deref()?;
    // A user-defined override is a real command in the resolved namespace —
    // it wins over the built-in, exactly as it does at runtime.
    if let Some(proc_def) = analysis.all_procs.get(resolved) {
        return Some(MathFuncTarget::UserProc(proc_def));
    }
    let spec = registry?.math_function_spec(&inv.name, profile)?;
    Some(MathFuncTarget::Builtin {
        bare: &inv.name,
        qualified: resolved,
        spec,
    })
}

/// The innermost unclosed opener (`{` or `[`) before `probe`, with its byte
/// offset — the delimiter of the word / substitution the position sits inside.
///
/// A quoted word is deliberately *not* tracked: an unbalanced `"` in
/// mid-typed source is indistinguishable from a literal one, and treating it
/// as an opener would swallow the rest of the file.
fn innermost_unclosed_opener(source: &str, probe: usize) -> Option<(usize, char)> {
    let prefix = source.get(..probe)?;
    let mut open: Vec<(usize, char)> = Vec::new();
    let mut it = prefix.char_indices();
    while let Some((idx, c)) = it.next() {
        match c {
            // A backslash escapes the next character wholesale, newline
            // continuations included.
            '\\' => {
                let _ = it.next();
            }
            '{' | '[' => open.push((idx, c)),
            '}' if open.last().map(|(_, o)| *o) == Some('{') => {
                open.pop();
            }
            ']' if open.last().map(|(_, o)| *o) == Some('[') => {
                open.pop();
            }
            _ => {}
        }
    }
    open.pop()
}

/// The complete words of the command enclosing `probe` within its own script
/// region, and the 0-based index of the word `probe` sits at.
///
/// The script region starts just after the innermost unclosed opener (a `[…]`
/// substitution's script, a braced body / expression) or at offset 0; within
/// it, the command starts after the last `\n` / `;` at depth 0.  Braced,
/// bracketed, and quoted words are consumed opaquely, so a nested body counts
/// as exactly one word.
fn command_words_at(source: &str, probe: usize) -> Option<(Vec<String>, usize)> {
    let region_start = innermost_unclosed_opener(source, probe).map_or(0, |(pos, _)| pos + 1);
    let text = source.get(region_start..probe)?;
    let bytes = source.as_bytes();
    let mut pos = region_start + command_start_in_region(text);
    let mut words: Vec<String> = Vec::new();
    while pos < probe {
        while pos < probe && matches!(bytes[pos], b' ' | b'\t' | b'\r' | b'\n') {
            pos += 1;
        }
        if pos >= probe {
            break;
        }
        let end = word_end(source, pos, probe);
        if end >= probe {
            // The word reaches `probe`: it *is* the word at the cursor (the
            // cursor sits inside it, or immediately after its last typed
            // character).  A word that ends *before* `probe` is complete and
            // counts towards the argument index.
            break;
        }
        words.push(source.get(pos..end).unwrap_or_default().to_owned());
        pos = end;
    }
    let index = words.len();
    Some((words, index))
}

/// Byte offset just after the word starting at `start`, capped at `limit`.
/// A braced / bracketed / quoted word is consumed whole (nesting counted); an
/// unclosed one returns `limit`, which is how the caller learns the cursor is
/// inside it.
///
/// The one place a single Tcl word is measured off raw source, shared with
/// [`crate::definition::offset_is_in_parameter_list`] so both agree on where a
/// braced word ends.
pub(crate) fn word_end(source: &str, start: usize, limit: usize) -> usize {
    let bytes = source.as_bytes();
    let (open, close) = match bytes[start] {
        b'{' => (b'{', b'}'),
        b'[' => (b'[', b']'),
        b'"' => (b'"', b'"'),
        _ => (0u8, 0u8),
    };
    let mut pos = start;
    if close != 0 {
        let mut depth = 0i32;
        while pos < limit {
            let c = bytes[pos];
            if c == b'\\' {
                pos += 2;
                continue;
            }
            if c == open && open != close {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth <= 0 || open == close {
                    return pos + 1;
                }
            }
            pos += 1;
        }
        return limit;
    }
    while pos < limit && !matches!(bytes[pos], b' ' | b'\t' | b'\r' | b'\n' | b';') {
        pos += 1;
    }
    pos
}

/// Byte offset (relative to `text`) where the last command in `text` begins —
/// just after the final depth-0 `\n` or `;`.
fn command_start_in_region(text: &str) -> usize {
    let mut depth = 0i32;
    let mut start = 0usize;
    let bytes = text.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' => idx += 1,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'\n' | b';' if depth <= 0 => start = idx + 1,
            _ => {}
        }
        idx += 1;
    }
    start
}

/// Whether the command whose word list contains `probe` treats that word as
/// an [`ArgRole::Expr`] argument, per the registry's own role map.
fn word_at_is_expr_argument(source: &str, probe: usize, registry: &CommandRegistry) -> bool {
    let Some((words, word_idx)) = command_words_at(source, probe) else {
        return false;
    };
    // Word 0 is the command head; an argument index only exists from word 1.
    let Some(arg_idx) = word_idx.checked_sub(1) else {
        return false;
    };
    let Some((head, args)) = words.split_first() else {
        return false;
    };
    // The cursor's own (incomplete) word is not in `args` yet; pad so the
    // registry's dynamic `arg_role_resolver` sees an argument at `arg_idx`.
    let mut arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
    while arg_strs.len() <= arg_idx {
        arg_strs.push("");
    }
    registry
        .arg_indices_for_role(head, &arg_strs, ArgRole::Expr)
        .contains(&arg_idx)
}

/// Whether the cursor sits inside an [`ArgRole::Expr`] argument of the
/// innermost enclosing command, per the **registry**'s own role map.
///
/// Built for completion, where the source is unbalanced — see the module docs
/// for why this cannot reuse [`mathfunc_call_at`]'s analyser records.
///
/// Two probes, mirroring the two ways an expression argument is written: the
/// *unbraced* form (`expr $a + 1`), where the cursor's own word is the
/// argument, and the *braced* form (`expr {$a + 1}`, `if {…}`), where the
/// argument is the braced word and the cursor is inside it.  A `[…]`
/// substitution re-enters script context, so a cursor inside one is never in
/// an expression argument of the command outside it.
#[must_use]
pub fn expr_arg_context_at(
    source: &str,
    line: u32,
    character: u32,
    registry: &CommandRegistry,
) -> bool {
    let line_index = tcl_lexer::LineIndex::new(source);
    let cursor_off =
        crate::definition::byte_offset_at(&line_index, source, line, character) as usize;
    match innermost_unclosed_opener(source, cursor_off) {
        // Inside a braced word: that word is the candidate argument.
        Some((pos, '{')) => word_at_is_expr_argument(source, pos, registry),
        // Command position inside a `[…]` substitution, or plain script
        // context: the cursor's own word is the candidate argument.
        _ => word_at_is_expr_argument(source, cursor_off, registry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> &'static CommandRegistry {
        tcl_registry::registry_for_dialect("tcl8.6")
    }

    /// TP: the canonical incomplete-source shape from issue #974 defect 2.
    #[test]
    fn expr_context_detected_in_a_partially_typed_expr() {
        let src = "set a [expr {si";
        assert!(expr_arg_context_at(src, 0, 15, reg()));
    }

    /// TP: `if` / `while` / `for` conditions are EXPR-role arguments too — the
    /// registry says so, nothing here names a command.
    #[test]
    fn expr_context_detected_in_if_and_while_conditions() {
        assert!(expr_arg_context_at("if {si", 0, 6, reg()));
        assert!(expr_arg_context_at("while {ma", 0, 9, reg()));
        // `for`'s third word is the condition; its second is a body.
        assert!(expr_arg_context_at("for {set i 0} {si", 0, 17, reg()));
    }

    /// TN: command position, an ordinary argument, a body argument, and a
    /// `[…]` substitution nested inside an expression (script context again).
    #[test]
    fn expr_context_not_detected_outside_expression_arguments() {
        assert!(!expr_arg_context_at("si", 0, 2, reg()));
        assert!(!expr_arg_context_at("puts si", 0, 7, reg()));
        assert!(!expr_arg_context_at("set si", 0, 6, reg()));
        assert!(!expr_arg_context_at("if {1} {si", 0, 10, reg()));
        assert!(!expr_arg_context_at("set a [expr {[si", 0, 16, reg()));
    }

    /// The unbraced `expr` form is an EXPR argument as well.
    #[test]
    fn expr_context_detected_for_the_unbraced_expr_form() {
        assert!(expr_arg_context_at("set a [expr si", 0, 14, reg()));
    }

    /// A completed earlier command on a previous line must not leak its role
    /// map into the line being typed.
    #[test]
    fn expr_context_is_scoped_to_the_command_being_typed() {
        let src = "if {$a > 1} { puts hi }\nputs si";
        assert!(!expr_arg_context_at(src, 1, 7, reg()));
    }

    #[test]
    fn command_words_reports_head_and_word_index() {
        let (words, idx) = command_words_at("puts hello wor", 14).expect("in a command");
        assert_eq!(words, vec!["puts".to_owned(), "hello".to_owned()]);
        assert_eq!(idx, 2);
        // Cursor between words: the fresh word still gets its own index.
        let (words, idx) = command_words_at("puts hello ", 11).expect("in a command");
        assert_eq!(words, vec!["puts".to_owned(), "hello".to_owned()]);
        assert_eq!(idx, 2);
    }
}
