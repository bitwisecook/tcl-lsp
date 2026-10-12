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

//! Is a byte offset in text Tcl never substitutes?
//!
//! Variable cursor labels come from selected original lexical components.
//! A syntactic component alone cannot decide whether its position is a
//! substituting source role: comments and brace-quoted data retain literal
//! dollar bytes. Editor providers therefore pair the shared source-reference
//! geometry with these actual-analysis applicability queries before using
//! their independently authorised lexical or original naming resolution.
//!
//! Comment geometry and braced-word source applicability are separate queries.
//! The data query uses the caller's actual complete analysis, original word
//! geometry and selected Registry source schema. `None` means unavailable
//! ownership or layout; it must not be treated as a substituting occurrence.
//!
//! This source classification grants no runtime variable cell, entered frame,
//! command dispatch or complete reference inventory. Native providers require
//! their independent original naming receipts. Explicit lexical declaration
//! advice uses this query to reject literal text consistently across providers.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{NativeWord, Span, WordKind};
use tcl_registry::ArgRole;

/// Current-physical-line compatibility scan for a `#` comment.
/// It does not retain quoting that starts on an earlier line. Analysis
/// consumers use `offset_in_physical_comment_in_analysis` and the independent
/// original word/data query; this approximation supplies no liveness proof.
///
/// A `#` begins a comment only in **command position** — where `tclParse.c`'s
/// `Tcl_ParseCommand` is about to read a command's first word: at the start of
/// a command, after a `;`, or just inside a `{` that **opens a word**.
///
/// That last qualifier is the whole subtlety. A `{` is structural only at
/// word-start position; **inside** a bare word it is an ordinary character, so
/// the text after it is still mid-word and a `#` there is data, not a comment.
/// Verified on tclsh 9.0.4 and 8.6.16:
///
/// ```tcl
/// set v OK
/// puts "1: [list a{# $v]"          ;# 1: a\{# OK   <- one word `a{#`, $v substituted
/// puts "2: [string cat a{# $v]"    ;# 2: a{#OK
/// puts {3: # not a comment $v}     ;# 3: # not a comment $v
/// if {1} {
///     # this IS a comment $v
///     puts "4: reached"            ;# 4: reached
/// }
/// puts "5: before" ;# comment $v   ;# 5: before
/// ```
///
/// Line 1 is the delicate case: `a{#` is a single bare word (the list quoting
/// shows the brace is literal) and `$v` really is substituted, so treating the
/// mid-word `{` as structural would classify the `#` as a comment and suppress
/// a genuine variable read.
///
/// A word-start `{` does keep command position, and that is sound whichever
/// kind of braced word it opens: if the braces hold a script, a `#` at its
/// start is a real comment; if they hold data, the `$var` inside is literal
/// text anyway (line 3). Either way the position is inert.
///
/// The single-line compatibility rules also limit these cases:
///
/// * The scan covers only the offset's own line, and starts out of command
///   position when the previous line ends in a backslash continuation.
/// * A `#` on a line with an odd number of preceding unescaped `"` is literal
///   text inside a quoted word.
/// * A `[` is not treated as opening a script at all, so a `#` inside a
///   bracket substitution is never claimed — which costs nothing, because a
///   comment there would swallow the `]` to end of line and the script would
///   not parse (`puts [list [# c] $v]` → `missing close-bracket`).
#[must_use]
pub fn offset_in_comment(source: &str, off: u32) -> bool {
    let off = off as usize;
    if off > source.len() {
        return false;
    }
    let line_start = source[..off].rfind('\n').map_or(0, |nl| nl + 1);
    let line = &source[line_start..off];
    let mut quotes = 0u32;
    // A backslash-continued previous line means this one resumes mid-command,
    // so its first word is not a command name.
    let mut command_position = !continues_previous_line(&source[..line_start]);
    // Whether the next non-blank character would start a fresh word — which is
    // the only position where `{` is structural.
    let mut at_word_start = true;
    let mut it = line.chars();
    while let Some(c) = it.next() {
        match c {
            // A backslash escapes the next character wholesale.
            '\\' => {
                let _ = it.next();
                command_position = false;
                at_word_start = false;
            }
            '"' => {
                quotes += 1;
                command_position = false;
                at_word_start = false;
            }
            '#' if command_position && quotes.is_multiple_of(2) => return true,
            // Whitespace ends a word but does **not** restore command
            // position: a `#` as a command's second word is data (`list # x`
            // yields `# x`).
            ' ' | '\t' | '\r' => at_word_start = true,
            ';' => {
                command_position = true;
                at_word_start = true;
            }
            // Structural only at word start; mid-word it is a literal brace.
            '{' if at_word_start => {
                command_position = true;
                at_word_start = true;
            }
            _ => {
                command_position = false;
                at_word_start = false;
            }
        }
    }
    false
}

/// Whether `before` (everything up to the start of the line under
/// examination) ends in a backslash-newline continuation, so that line resumes
/// an unfinished command rather than starting a new one.
fn continues_previous_line(before: &str) -> bool {
    let Some(prev) = before.strip_suffix('\n') else {
        return false;
    };
    let prev = prev.strip_suffix('\r').unwrap_or(prev);
    // An odd run of trailing backslashes means the last one escapes the
    // newline; an even run is escaped backslashes and the command ended.
    !prev
        .chars()
        .rev()
        .take_while(|&c| c == '\\')
        .count()
        .is_multiple_of(2)
}

/// Whether a retained physical comment fact covers this original cursor.
/// This inventory includes command-position physical comments; absence does
/// not classify other inert text or an unavailable Body as live code. The
/// caller must use the independent original word/data query for that purpose.
/// Missing or stale analysis/source ownership remains unavailable.
#[must_use]
pub(crate) fn offset_in_physical_comment_in_analysis(
    source: &str,
    analysis: &AnalysisResult,
    off: u32,
) -> Option<bool> {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    source.get(..usize::try_from(off).ok()?)?;
    let facts =
        tcl_compiler::analyser::utils::script_comment_facts_from_analysis(source, analysis)?;
    Some(facts.iter().any(|fact| contains(fact.span, off)))
}

/// Classify original braced data using the complete retained analysis.
///
/// `Some(true)` denotes literal data under the retained source schema;
/// `Some(false)` denotes an original non-braced word or authentic script/expr
/// position. `None` preserves unknown, stale, partial and cooked geometry.
/// The result grants source applicability only, independently of execution,
/// variable substitution receipts and editable reference coverage.
#[must_use]
pub(crate) fn offset_in_data_brace_in_analysis(
    source: &str,
    analysis: &AnalysisResult,
    off: u32,
) -> Option<bool> {
    // Implementation contract: naming.core.original-data-brace-classification
    // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
    let config = analysis.body_lexer_config?;
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    let command = structure
        .commands
        .iter()
        .filter(|command| contains(command.execution_span(source), off))
        .min_by_key(|command| command.execution_span(source).len())?;
    if command.is_partial {
        return None;
    }
    let plan = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        command.execution_span(source),
        config,
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let (written, word) = plan
        .commands
        .first()?
        .words
        .iter()
        .enumerate()
        .find(|(_, word)| contains(word.span(), off))?;
    if word.group().expand {
        return None;
    }
    if word.group().kind != WordKind::Braced {
        return Some(false);
    }
    if written == 0 {
        return Some(true);
    }
    let words = crate::original_invocation::source_registry_words(source, analysis, command)?;
    let argument = usize::try_from(words.active_argument_at(written, off)?).ok()?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let bodies = words.source_script_bodies(&context);
    if bodies.iter().any(|body| contains(body.content_span(), off)) {
        return Some(false);
    }
    let (list_argument, expressions) = words.with_source_schema(&context, |schema| {
        (
            schema
                .authored_source_case_invocation()
                .and_then(|(_, layout)| layout.clause_list_index),
            schema.authored_source_expression_arguments(),
        )
    })?;
    if list_argument == Some(argument) {
        return None;
    }
    let expressions = expressions?;
    if expressions.concatenates && words.arguments.len() != 1 {
        // A separately parsed contributor cannot own the joined expression.
        return None;
    }
    if expressions.arguments.contains(&argument) {
        return expression_data_at(word, analysis, off);
    }
    let mut result = true;
    for &(_, role) in words
        .roles
        .as_deref()?
        .iter()
        .filter(|(index, _)| *index == argument)
    {
        result &= match role {
            ArgRole::Body => return None,
            ArgRole::Expr => return None,
            ArgRole::LambdaLiteral => lambda_data_at(word, off)?,
            _ => true,
        };
    }
    Some(result)
}

fn contains(span: Span, off: u32) -> bool {
    span.start() <= off && off < span.end()
}

/// Expression terms retain their own original string-versus-substitution
/// purpose. An expression's braced literal is data even inside a script role.
fn expression_data_at(word: &NativeWord, analysis: &AnalysisResult, off: u32) -> Option<bool> {
    let content = word.content_span().ok()?;
    let expression = word.image().bytes().get(content.as_range())?;
    let profile = analysis.resolved_profile()?;
    let config = word.config();
    let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
        expression,
        &config.grammar_over(profile.grammar),
        profile.expr_grammar_base,
        profile.f5_core_expr_grammar(),
    );
    if unknown {
        return None;
    }
    let local = off.checked_sub(content.start())?;
    let terms = tcl_lexer::expression_terms(expression, &tokens, config)?;
    let term = terms.iter().find(|term| term.source.contains(&local))?;
    Some(term.kind == tcl_lexer::ExprTermKind::String)
}

fn lambda_data_at(word: &NativeWord, off: u32) -> Option<bool> {
    let elements = tcl_compiler::lambda_literal::split_original_lambda_literal(word)?;
    if elements
        .braced_body()
        .is_some_and(|body| contains(body, off))
    {
        Some(false)
    } else if elements.body.is_some_and(|body| contains(body, off)) {
        None
    } else if contains(elements.params, off)
        || elements
            .namespace
            .is_some_and(|namespace| contains(namespace, off))
    {
        Some(true)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(src: &str, needle: &str) -> Option<bool> {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(src, "tcl8.6");
        offset_in_data_brace_in_analysis(src, &analysis, offset_of(src, needle))
    }

    fn offset_of(src: &str, needle: &str) -> u32 {
        u32::try_from(src.find(needle).expect("needle present")).expect("fits u32")
    }

    /// TP: a `$level` inside an inert comment.
    #[test]
    fn a_dollar_ref_inside_a_comment_is_inert() {
        let src = "proc p {} {\n    set level 42\n    # so $level ...\n}\n";
        assert!(offset_in_comment(src, offset_of(src, "$level")));
    }

    /// TN: a `#` that is not in command position is ordinary text, and a `#`
    /// inside a quoted word is not a comment either.
    #[test]
    fn a_non_command_position_hash_is_not_a_comment() {
        let src = "puts a#b $v\n";
        assert!(!offset_in_comment(src, offset_of(src, "$v")));
        let quoted = "puts \"a # b $v\"\n";
        assert!(!offset_in_comment(quoted, offset_of(quoted, "$v")));
        // A trailing comment after `;` *is* one.
        let trailing = "set x 1 ;# note $v\n";
        assert!(offset_in_comment(trailing, offset_of(trailing, "$v")));
    }

    /// TN: a `{` **inside** a bare word
    /// is an ordinary character, so the `#` after it is not in command
    /// position and `$v` is a genuine, substituted read.
    ///
    /// Oracle (tclsh 9.0.4 and 8.6.16 agree):
    ///
    /// ```text
    /// set v OK
    /// puts "1: [list a{# $v]"        -> 1: a\{# OK
    /// puts "2: [string cat a{# $v]"  -> 2: a{#OK
    /// ```
    ///
    /// The list quoting (`a\{#`) shows `a{#` is one bare word with a literal
    /// brace, and `OK` shows `$v` was substituted.
    #[test]
    fn a_mid_word_brace_does_not_start_command_position() {
        for src in [
            "set v OK\nputs a{# $v\n",
            "set v OK\nset w [string cat a{# $v]\n",
            "set v OK\nputs abc{def# $v\n",
        ] {
            assert!(
                !offset_in_comment(src, offset_of(src, "$v")),
                "mid-word brace must not make the `#` a comment: {src:?}"
            );
        }
    }

    /// TP: a `{` at **word start** is structural, so it does keep command
    /// position — and that is sound either way, because a braced word holds
    /// either a script (whose leading `#` is a real comment) or data (whose
    /// `$var` is literal text). Oracle: `puts {3: # not a comment $v}` prints
    /// `3: # not a comment $v` verbatim on both 9.0.4 and 8.6.16.
    #[test]
    fn a_word_start_brace_keeps_command_position() {
        // A `#` immediately inside a word-start brace.
        let src = "puts {# not a comment $v}\n";
        assert!(offset_in_comment(src, offset_of(src, "$v")));
        // But a non-leading `#` inside that same braced word is not: the
        // brace's first word is `3:`, so the `#` is its second word.
        let later = "puts {3: # not a comment $v}\n";
        assert!(!offset_in_comment(later, offset_of(later, "$v")));
    }

    /// TN: a `#` as a command's *second* word is data, not a comment —
    /// whitespace ends a word but does not restore command position
    /// (`list # x` yields `# x`).
    #[test]
    fn a_hash_in_an_argument_position_is_not_a_comment() {
        let src = "list # $v\n";
        assert!(!offset_in_comment(src, offset_of(src, "$v")));
    }

    /// TN: a line resumed by a backslash continuation is mid-command, so its
    /// first word is not a command name and a `#` there is data.
    #[test]
    fn a_continued_line_does_not_start_in_command_position() {
        let src = "list a \\\n# $v\n";
        assert!(!offset_in_comment(src, offset_of(src, "$v")));
        // An *escaped* backslash at end of line does end the command, so the
        // next line starts fresh and its `#` is a comment.
        let escaped = "list a \\\\\n# $v\n";
        assert!(offset_in_comment(escaped, offset_of(escaped, "$v")));
    }

    /// Original data and nested script words use the same retained geometry.
    #[test]
    fn a_dollar_ref_inside_a_data_brace_is_inert() {
        // Implementation contract: naming.core.original-data-brace-classification
        // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
        for src in [
            "set t {plain $level here}\n",
            "proc p {} {\n    set t {plain $level here}\n}\n",
            "set t {é $level}\n",
        ] {
            assert_eq!(classify(src, "$level"), Some(true), "{src:?}");
        }
    }

    #[test]
    fn a_dollar_ref_inside_a_script_brace_keeps_source_applicability() {
        // Implementation contract: naming.core.original-data-brace-classification
        // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
        for src in [
            "proc p {} {\n    puts $level\n}\n",
            "if {$level > 1} { puts hi }\n",
            "while {$level} { break }\n",
            "foreach i {1 2} { puts $level }\n",
            "set x [expr {$level * 2}]\n",
            "apply {{a} {puts $level}} 1\n",
            "eval {puts $level}\n",
            "switch $v {a {puts $level}}\n",
        ] {
            assert_eq!(classify(src, "$level"), Some(false), "{src:?}");
        }
    }

    #[test]
    fn original_case_lambda_and_expression_literals_keep_data_separate() {
        // Implementation contract: naming.core.original-data-brace-classification
        // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
        assert_eq!(classify("switch $v {{$level} {puts ok}}\n", "$level"), None);
        assert_eq!(
            classify("apply {{$level} {puts ok}} 1\n", "$level"),
            Some(true)
        );
        assert_eq!(classify(r"apply {{} puts\ $level}", "$level"), None);
        assert_eq!(classify("expr {{$level}}\n", "$level"), Some(true));
        assert_eq!(classify("expr {\"$level\"}\n", "$level"), Some(false));
    }

    #[test]
    fn original_expression_contributors_do_not_borrow_independent_data_roles() {
        // Implementation contract: naming.core.original-data-brace-classification
        // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
        for src in [
            "expr {1 +} {$level}\n",
            "expr {$level} {+ 1}\n",
            "interp alias {} calculate {} expr 1 +\ncalculate {$level}\n",
        ] {
            assert_eq!(classify(src, "$level"), None, "{src:?}");
        }
        assert_eq!(classify("expr {$level + 1}\n", "$level"), Some(false));
        assert_eq!(classify("expr {{$level}}\n", "$level"), Some(true));
    }

    #[test]
    fn original_data_classification_keeps_alias_prefix_and_shadow_boundaries() {
        // Implementation contract: naming.core.original-data-brace-classification
        // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
        assert_eq!(
            classify(
                "interp alias {} write {} set\nwrite t {plain $level}\n",
                "$level"
            ),
            Some(true)
        );
        assert_eq!(
            classify("interp alias {} act {} if 1\nact {puts $level}\n", "$level"),
            Some(false)
        );
        assert_eq!(
            classify("proc set {name value} {}\nset t {plain $level}\n", "$level"),
            None
        );
        assert_eq!(classify("zzznotacommand {plain $level}\n", "$level"), None);
        assert_eq!(classify("set t \"plain $level\"\n", "$level"), Some(false));
    }

    #[test]
    fn original_data_classification_declines_stale_or_unowned_inputs() {
        // Implementation contract: naming.core.original-data-brace-classification
        // docs/design/analysis/name-resolution-proofs/core-original-data-brace-classification.md
        let src = "set t {plain $level}\n";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(src, "tcl8.6");
        let off = offset_of(src, "$level");
        assert_eq!(
            offset_in_data_brace_in_analysis(src, &analysis, off),
            Some(true)
        );
        assert_eq!(
            offset_in_data_brace_in_analysis("set t {other $level}\n", &analysis, off),
            None
        );
        let mut wrong_config = analysis.clone();
        wrong_config
            .body_lexer_config
            .as_mut()
            .expect("config retained")
            .strict_quoting ^= true;
        assert_eq!(
            offset_in_data_brace_in_analysis(src, &wrong_config, off),
            None
        );
        assert_eq!(
            offset_in_data_brace_in_analysis(src, &AnalysisResult::default(), off),
            None
        );
        let partial = "set t {plain $level";
        assert_eq!(classify(partial, "$level"), None);
    }
    #[test]
    fn original_comment_cursor_keeps_multiline_quoted_substitutions_live() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        // These are source-applicability controls, not entered/native frames.
        let source = "set v value\nset text \"é\n# $v\"\n# inert $v\n";
        let live = offset_of(source, "$v");
        let inert = u32::try_from(source.rfind("$v").unwrap()).unwrap();
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            assert_eq!(
                offset_in_physical_comment_in_analysis(source, &analysis, live),
                Some(false),
                "{dialect}"
            );
            assert_eq!(
                crate::definition::offset_is_inert(source, &analysis, live),
                Some(false),
                "{dialect}"
            );
            assert_eq!(
                offset_in_physical_comment_in_analysis(source, &analysis, inert),
                Some(true),
                "{dialect}"
            );
            assert_eq!(
                crate::definition::offset_is_inert(source, &analysis, inert),
                Some(true),
                "{dialect}"
            );
        }
    }

    #[test]
    fn original_comment_cursor_declines_stale_or_unowned_analysis() {
        // naming.core.original-comment-source-context
        // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
        let source = "# inert $v\n";
        let offset = offset_of(source, "$v");
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            offset_in_physical_comment_in_analysis(source, &analysis, offset),
            Some(true)
        );
        assert_eq!(
            crate::definition::offset_is_inert("# other $v\n", &analysis, offset),
            None
        );
        let mut foreign = analysis.clone();
        foreign.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        assert_eq!(
            crate::definition::offset_is_inert(source, &foreign, offset),
            None
        );
        foreign = analysis.clone();
        foreign.resolved_input = None;
        assert_eq!(
            crate::definition::offset_is_inert(source, &foreign, offset),
            None
        );
        assert_eq!(
            crate::definition::offset_is_inert(source, &AnalysisResult::default(), offset),
            None
        );
    }
}
