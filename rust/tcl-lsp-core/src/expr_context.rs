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
//! Complete-source Logical math targets consume the Compiler's checked
//! expression occurrence and original declaration cards. Analyser invocation
//! rows remain reporting/span metadata and cannot recover target identity.
//! Native math functions retain their separate fixed-table/command-table owner.
//!
//! Actual completion uses [`source_expr_arg_context_at`]: it retains the current
//! original Expr operand and asks the configured shared expression lexer about
//! its real source prefix. Literal/comment/script positions cannot borrow
//! expression-space completion. No missing closer is added.
//!
//! The nominal [`expr_arg_context_at`] helper supports unpositioned compatibility
//! callers. Neither source geometry route grants evaluation, Native dispatch or
//! edit authority.

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

/// Conditional metadata from the original Logical expression owner. Every
/// declaration remains a distinct source allocation; no runtime winner is chosen.
pub(crate) struct MathFuncTarget<'a> {
    pub(crate) occurrence: tcl_compiler::command_binding::OriginalLogicalMathFunctionOccurrence,
    pub(crate) declarations:
        Option<Vec<&'a tcl_compiler::analyser::types::LogicalSourceProcedureDeclaration>>,
    pub(crate) builtin: Option<&'a tcl_registry::CommandSpec>,
}

/// Original Logical math-function source candidates. The actual expression,
/// parent schema and original header cards supply identity; analyser report
/// names and command-table displays never select a declaration.
pub(crate) fn math_function_target_at<'a>(
    source: &str,
    analysis: &'a AnalysisResult,
    cursor_off: u32,
) -> Option<MathFuncTarget<'a>> {
    let occurrence = analysis.logical_math_function_at(source, cursor_off)?;
    let declarations = occurrence.source_declarations(analysis);
    let builtin = occurrence.builtin_metadata(analysis);
    Some(MathFuncTarget {
        occurrence,
        declarations,
        builtin,
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
/// Legacy nominal authoring geometry only. Current expression completion uses
/// the configured expression lexer; parameter-list extents use the original
/// token grouper and `NativeWord` geometry rather than this compatibility scan.
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
    // segmentation-drift-ok: a bare-word end scan inside the `expr`-context
    // probe, which runs on half-typed source at a cursor where a full
    // tokenise is not affordable and often not possible.
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
            // segmentation-drift-ok: `command_start_in_region` is a private
            // last-command splitter for the same half-typed-source probe; it
            // is depth-tracked but dialect-blind.
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

/// Nominal authoring probe for an [`ArgRole::Expr`] argument in source without
/// retained analysis. Production completion uses the original source selector
/// below; this compatibility helper has no selected handler or shadow proof.
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
///
/// Takes a pre-built `line_index` so a caller resolving several positions per
/// request (`completions`) shares one index instead of each probe rebuilding
/// its own.
#[must_use]
pub fn expr_arg_context_at(
    source: &str,
    line: u32,
    character: u32,
    line_index: &tcl_lexer::LineIndex,
    registry: &CommandRegistry,
) -> bool {
    let cursor_off =
        crate::definition::byte_offset_at(line_index, source, line, character) as usize;
    match innermost_unclosed_opener(source, cursor_off) {
        // Inside a braced word: that word is the candidate argument.
        Some((pos, '{')) => word_at_is_expr_argument(source, pos, registry),
        // Command position inside a `[…]` substitution, or plain script
        // context: the cursor's own word is the candidate argument.
        _ => word_at_is_expr_argument(source, cursor_off, registry),
    }
}

/// Original source word selected by an independently retained expression role.
/// This is readonly source geometry, without expression evaluation or rewrite
/// equivalence. Hosted candidates retain their conditional applicability.
#[derive(Debug, Clone)]
pub(crate) struct SourceExpressionOperand {
    word: tcl_lexer::NativeWord,
}

impl SourceExpressionOperand {
    pub(crate) fn content_span(&self) -> Option<tcl_lexer::Span> {
        self.word.content_span().ok()
    }
}

/// The original expression operand covering this cursor under the complete
/// current source/configuration and actual Registry generation. Reporting
/// heads cannot select a role after an original command becomes unavailable.
pub(crate) fn source_expression_operand_at(
    source: &str,
    analysis: &AnalysisResult,
    offset: u32,
) -> Option<SourceExpressionOperand> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    if analysis.has_original_vendor_source_names() {
        let (metadata, _) = crate::original_invocation::selected_vendor_registry_words_at(
            source, analysis, offset,
        )?;
        let shape = metadata.shape();
        if !shape.roles_complete() {
            return None;
        }
        let mut selected = None;
        for &(ordinal, role) in shape.roles() {
            if role != ArgRole::Expr {
                continue;
            }
            let index = shape
                .argument_offset()
                .checked_add(usize::from(ordinal))?
                .checked_add(1)?;
            let word = shape.original_words().get(index)?;
            let span = word.content_span().ok()?;
            if span.start() <= offset
                && offset <= span.end()
                && selected
                    .replace(SourceExpressionOperand { word: word.clone() })
                    .is_some()
            {
                return None;
            }
        }
        return selected;
    }
    let selector = ExpressionOperandSelector {
        source,
        analysis,
        registry,
        walk: crate::refactor::FrameWalk::new(source, analysis)?,
    };
    selector.in_region(source, 0, offset, 0)
}

struct ExpressionOperandSelector<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    registry: &'a CommandRegistry,
    walk: crate::refactor::FrameWalk<'a>,
}
impl ExpressionOperandSelector<'_> {
    fn in_region(
        &self,
        text: &str,
        base: u32,
        offset: u32,
        depth: u32,
    ) -> Option<SourceExpressionOperand> {
        if crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
            return None;
        }
        for command in if depth == 0 {
            self.walk.segment_document(text)
        } else {
            self.walk.segment_nested(text, base)
        } {
            if command.span.start() > offset || offset > command.span.end() {
                continue;
            }
            let regions = self
                .walk
                .same_frame_regions(self.source, &command)
                .into_iter()
                .chain(self.walk.frame_shifted_regions(self.source, &command));
            for (start, end) in regions {
                let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
                    continue;
                };
                if start <= offset
                    && offset <= end
                    && let Some(text) = self.source.get(start as usize..end as usize)
                    && let Some(operand) = self.in_region(text, start, offset, depth + 1)
                {
                    return Some(operand);
                }
            }
            let selected = crate::original_invocation::selected_registry_words(
                self.source,
                self.analysis,
                &command,
                self.registry,
            )?;
            let words = self.walk.native_words(self.source, &command)?;
            for &(index, role) in selected.roles.as_ref()? {
                if role != ArgRole::Expr {
                    continue;
                }
                let origin = selected.origins.get(index.checked_add(1)?)?;
                let tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) =
                    origin
                else {
                    continue;
                };
                let word = words.get(*written)?;
                let span = word.content_span().ok()?;
                if span.start() <= offset && offset <= span.end() {
                    return Some(SourceExpressionOperand { word: word.clone() });
                }
            }
        }
        None
    }
}

/// Completion geometry from the same retained source-role selector used by
/// expression actions. It grants no native math-function target or evaluation.
pub(crate) fn source_expr_arg_context_at(
    source: &str,
    analysis: &AnalysisResult,
    line: u32,
    character: u32,
    line_index: &tcl_lexer::LineIndex,
) -> bool {
    let Some(position) =
        crate::original_context::CurrentSourcePosition::capture(source, analysis, line, character)
    else {
        return false;
    };
    let offset = position.offset();
    // The actual source-position owner validates UTF-16 rather than clamping
    // a foreign/out-of-range coordinate through a supplied display index.
    let _ = line_index;
    let Some(operand) = source_expression_operand_at(source, analysis, offset) else {
        return false;
    };
    let Some(span) = operand.content_span() else {
        return false;
    };
    let current = position.context();
    if operand.word.image().bytes() != source.as_bytes()
        || operand.word.config() != current.config()
    {
        return false;
    }
    let profile = analysis
        .resolved_input
        .as_ref()
        .map(tcl_compiler::analyser::ResolvedAnalysisInput::unit_profile);
    let Some(profile) = profile else {
        return false;
    };
    let grammar = tcl_lexer::ExpressionCursorGrammar {
        config: current.config(),
        grammar: profile.grammar,
        expr_grammar_base: profile.expr_grammar_base,
        f5_word_grammar: profile.f5_core_expr_grammar(),
    };
    tcl_lexer::original_expression_cursor_in(operand.word.image(), span, offset, grammar)
        .is_some_and(|cursor| {
            cursor.matches_source(operand.word.image(), grammar) && cursor.is_expression_position()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> &'static CommandRegistry {
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands()
    }

    /// Builds the `line_index` [`expr_arg_context_at`] now expects, so the
    /// tests keep passing a bare source string.
    fn ctx(source: &str, line: u32, character: u32) -> bool {
        let line_index = tcl_lexer::LineIndex::new(source);
        expr_arg_context_at(source, line, character, &line_index, reg())
    }

    /// TP: the canonical incomplete-source shape.
    #[test]
    fn expr_context_detected_in_a_partially_typed_expr() {
        let src = "set a [expr {si";
        assert!(ctx(src, 0, 15));
    }

    /// TP: `if` / `while` / `for` conditions are EXPR-role arguments too — the
    /// registry says so, nothing here names a command.
    #[test]
    fn expr_context_detected_in_if_and_while_conditions() {
        assert!(ctx("if {si", 0, 6));
        assert!(ctx("while {ma", 0, 9));
        // `for`'s third word is the condition; its second is a body.
        assert!(ctx("for {set i 0} {si", 0, 17));
    }

    /// TN: command position, an ordinary argument, a body argument, and a
    /// `[…]` substitution nested inside an expression (script context again).
    #[test]
    fn expr_context_not_detected_outside_expression_arguments() {
        assert!(!ctx("si", 0, 2));
        assert!(!ctx("puts si", 0, 7));
        assert!(!ctx("set si", 0, 6));
        assert!(!ctx("if {1} {si", 0, 10));
        assert!(!ctx("set a [expr {[si", 0, 16));
    }

    /// The unbraced `expr` form is an EXPR argument as well.
    #[test]
    fn expr_context_detected_for_the_unbraced_expr_form() {
        assert!(ctx("set a [expr si", 0, 14));
    }

    /// A completed earlier command on a previous line must not leak its role
    /// map into the line being typed.
    #[test]
    fn expr_context_is_scoped_to_the_command_being_typed() {
        let src = "if {$a > 1} { puts hi }\nputs si";
        assert!(!ctx(src, 1, 7));
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

#[cfg(test)]
mod original_expression_position_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    // Implementation contract: naming.core.original-expression-source-selection

    fn original_expression_positions_keep_handler_roles_and_source_currency() {
        let source = "expr {sin(1)}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let offset = u32::try_from(source.find("sin").unwrap()).unwrap();
        let operand = source_expression_operand_at(source, &analysis, offset)
            .expect("original expression role");
        assert_eq!(operand.content_span().unwrap(), tcl_lexer::Span::new(6, 12));
        analysis.command_invocations.clear();
        analysis.all_procs.clear();
        analysis.dialect = "f5-irules".into();
        assert!(source_expression_operand_at(source, &analysis, offset).is_some());
        assert!(source_expression_operand_at(&format!("#{source}"), &analysis, offset).is_none());
        for source in [
            "set data {sin(1)}",
            "# expr {sin(1)}",
            "proc expr args {}; expr {sin(1)}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("sin").unwrap()).unwrap();
            assert!(
                source_expression_operand_at(source, &analysis, offset).is_none(),
                "{source}"
            );
        }
    }

    #[test]
    // Implementation contract: naming.core.original-expression-source-selection

    fn original_expression_positions_retain_nested_hosted_source_candidates() {
        let source = "when HTTP_REQUEST {if {1 == 1} {HTTP::respond 200}}";
        let analysis = Analyser::new().analyse(source, "f5-irules");
        let offset = u32::try_from(source.find("1 ==").unwrap()).unwrap();
        let operand = source_expression_operand_at(source, &analysis, offset)
            .expect("hosted original expression role");
        let span = operand.content_span().unwrap();
        assert_eq!(
            &source[span.start() as usize..span.end() as usize],
            "1 == 1"
        );
        assert!(source_expression_operand_at(&format!("#{source}"), &analysis, offset).is_none());
    }
}

#[cfg(test)]
mod configured_expression_cursor_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn at(source: &str, analysis: &AnalysisResult, needle: &str) -> bool {
        let offset = u32::try_from(source.find(needle).unwrap() + needle.len()).unwrap();
        let index = tcl_lexer::LineIndex::new_lsp(source);
        let position = index.position_at_utf16(offset, source);
        source_expr_arg_context_at(
            source,
            analysis,
            position.line,
            position.character.get(),
            &index,
        )
    }

    #[test]
    fn original_expression_cursor_uses_configured_literal_and_comment_grammar() {
        for source in [
            "expr {{[} eq {[} ? sin(1) : 0}",
            "expr {1 # [ inert\n + sin(1)}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            assert!(at(source, &analysis, "sin"), "{source}");
        }
        for (source, needle) in [
            ("expr {[set value sin]}", "sin"),
            ("expr {\"literal sin\"}", "sin"),
            ("expr {1 # sin\n+2}", "sin"),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl9.0");
            assert!(!at(source, &analysis, needle), "{source}");
        }
    }

    #[test]
    fn original_expression_cursor_keeps_whole_source_input_and_utf16_currency() {
        let source = "expr {\"😀\" eq \"😀\" ? sin(1) : 0}";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0");
        assert!(at(source, &analysis, "sin"));
        let index = tcl_lexer::LineIndex::new_lsp(source);
        assert!(!source_expr_arg_context_at(
            source, &analysis, 0, 500, &index
        ));
        assert!(!source_expr_arg_context_at(source, &analysis, 0, 8, &index));
        assert!(!at(&source.replace("sin", "cos"), &analysis, "cos"));
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert!(!at(source, &analysis, "sin"));
    }
}

#[cfg(test)]
mod logical_math_source_target_tests {
    use super::*;

    #[test]
    fn logical_math_targets_keep_authentic_expression_and_header_allocations() {
        let source = "namespace eval ::tcl::mathfunc {proc special {root} {return $root}}; namespace eval N {namespace eval tcl::mathfunc {proc special {local} {return $local}}; proc caller {n} {expr {special($n)}}}";
        let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
        let offset = u32::try_from(source.rfind("special(").unwrap()).unwrap();
        let selected = math_function_target_at(source, &analysis, offset)
            .expect("typed original Logical math occurrence");
        let names: Vec<_> = selected
            .declarations
            .unwrap()
            .into_iter()
            .map(|row| row.metadata().params[0].name.as_str())
            .collect();
        assert_eq!(names, ["local", "root"]);
        assert!(selected.occurrence.has_opaque_or_substitution_operands());
        analysis.all_procs.clear();
        assert_eq!(
            math_function_target_at(source, &analysis, offset)
                .unwrap()
                .declarations
                .unwrap()
                .len(),
            2
        );
        assert!(math_function_target_at(&format!("{source} "), &analysis, offset).is_none());
        assert!(math_function_target_at(source, &analysis, offset + 7).is_none());
    }
}
