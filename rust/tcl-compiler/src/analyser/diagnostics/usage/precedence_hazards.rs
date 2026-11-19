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

//! W154/W155: an unparenthesised operand whose other grouping gives a different result.

use crate::analyser::state::Analyser;
use crate::analyser::types::{CodeFix, Diagnostic, Severity};
use crate::irules_checks::FixSafety;
use tcl_core_types::DiagCode;
use tcl_syntax::expr::precedence::HazardTier;

impl Analyser {
    /// W154/W155 on one expression operand. A braced word reaches the
    /// evaluator as written; a quoted or bare word does too when it has no
    /// substitution or backslash, since its value is then its source text.
    pub(in crate::analyser) fn emit_w154_precedence_hazards(
        &mut self,
        word: &tcl_lexer::NativeWord,
    ) {
        if word.image().bytes() != self.source.as_bytes() {
            return;
        }
        let Ok(body) = word.content_span() else {
            return;
        };
        let Some(text) = self.source.get(body.as_range()).map(str::to_owned) else {
            return;
        };
        if word.group().kind != tcl_lexer::WordKind::Braced && text.contains(['$', '[', '\\']) {
            return;
        }
        self.emit_precedence_hazards_in(&text, word.config(), Some(body.start()), body);
    }

    /// W154/W155 on an expression `expr` concatenates from several literal
    /// words (`expr 1 & 1 == 1`). The joined text is not source, so the
    /// whole argument span carries the diagnostic and no fix is offered.
    pub(in crate::analyser) fn emit_joined_precedence_hazards(
        &mut self,
        joined: &str,
        config: tcl_lexer::LexerConfig,
        span: tcl_lexer::Span,
    ) {
        self.emit_precedence_hazards_in(joined, config, None, span);
    }

    fn emit_precedence_hazards_in(
        &mut self,
        text: &str,
        config: tcl_lexer::LexerConfig,
        base: Option<u32>,
        fallback: tcl_lexer::Span,
    ) {
        let Some(context) = self.original_expression_parser_context() else {
            return;
        };
        let Some(expression) =
            tcl_syntax::expr::OriginalSourceExpression::capture(text, &context, config)
        else {
            return;
        };
        for hazard in expression.precedence_hazards() {
            let (code, severity) = match hazard.tier {
                HazardTier::Misreading => (DiagCode::W154, Severity::Warning),
                HazardTier::NonAssociative => (DiagCode::W155, Severity::Hint),
            };
            let message = format!(
                "{}, so this parses as `{}`; if you meant `{}`, add parentheses.",
                capitalise(&hazard.reason),
                hazard.as_parsed,
                hazard.likely_intended
            );
            let Some(base) = base else {
                self.result
                    .diagnostics
                    .push(Diagnostic::new(code, fallback, message, severity));
                continue;
            };
            let shift = |span: tcl_lexer::Span| {
                tcl_lexer::Span::new(span.start() + base, span.end() + base)
            };
            let extent = shift(hazard.extent);
            let fixes = vec![
                CodeFix {
                    span: extent,
                    new_text: hazard.as_parsed.clone(),
                    description: format!("Parenthesise as parsed: {}", hazard.as_parsed),
                    safety: FixSafety::SemanticsEquivalent,
                },
                CodeFix {
                    span: extent,
                    new_text: hazard.likely_intended.clone(),
                    description: format!("Regroup as {}", hazard.likely_intended),
                    safety: FixSafety::RequiresReview,
                },
            ];
            self.result.diagnostics.push(
                Diagnostic::new(code, shift(hazard.operator), message, severity).with_fixes(fixes),
            );
        }
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

#[cfg(test)]
mod tests {
    use crate::analyser::types::Diagnostic;
    use tcl_core_types::DiagCode;

    fn w154(dialect: &str, source: &str) -> Vec<Diagnostic> {
        crate::analyser::state::Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .into_iter()
            .filter(|d| d.code == DiagCode::W154)
            .collect()
    }

    #[test]
    fn bitwise_and_over_equality_is_reported_on_the_operator() {
        let source = "if {$c & 0xff == 0x81} {puts hit}";
        let found = w154("tcl8.6", source);
        let [diagnostic] = found.as_slice() else {
            panic!("{found:?}");
        };
        assert_eq!(&source[diagnostic.span.as_range()], "&");
        assert!(diagnostic.message.contains("`$c & (0xff == 0x81)`"));
        let intended = diagnostic
            .fixes
            .iter()
            .find(|fix| fix.new_text == "($c & 0xff) == 0x81")
            .unwrap();
        assert_eq!(&source[intended.span.as_range()], "$c & 0xff == 0x81");
    }

    #[test]
    fn explicit_or_unbraced_groupings_are_quiet() {
        assert!(w154("tcl8.6", "if {($c & 0xff) == 0x81} {puts hit}").is_empty());
        assert!(w154("tcl8.6", "set x [expr {$a + $b == 3}]").is_empty());
    }

    #[test]
    fn irules_word_operators_and_nested_expressions_are_checked() {
        let found = w154(
            "f5-irules",
            "when HTTP_REQUEST { set f [expr {$flags | 2 == 2}]\n if { not $a equals \"x\" } { log local0. x } }",
        );
        assert_eq!(found.len(), 2, "{found:?}");
    }
}

#[cfg(test)]
mod coverage {
    use tcl_core_types::DiagCode;

    fn codes(dialect: &str, source: &str) -> Vec<(DiagCode, String)> {
        crate::analyser::state::Analyser::new()
            .analyse(source, dialect)
            .diagnostics
            .into_iter()
            .filter(|d| matches!(d.code, DiagCode::W154 | DiagCode::W155))
            .map(|d| (d.code, source[d.span.as_range()].to_owned()))
            .collect()
    }

    /// Every reachable expression position reports, in each dialect.
    #[test]
    fn every_expression_position_reports() {
        let cases: &[(&str, &str, &str)] = &[
            ("expr", "tcl8.6", "expr {$a & 1 == 1}"),
            ("set-expr", "tcl8.6", "set x [expr {$a & 1 == 1}]"),
            ("if", "tcl8.6", "if {$a & 1 == 1} {puts x}"),
            (
                "elseif",
                "tcl8.6",
                "if {$b} {puts y} elseif {$a & 1 == 1} {puts x}",
            ),
            ("if-then", "tcl8.6", "if {$a & 1 == 1} then {puts x}"),
            ("while", "tcl8.6", "while {$a & 1 == 1} {break}"),
            (
                "for",
                "tcl8.6",
                "for {set i 0} {$i & 1 == 1} {incr i} {break}",
            ),
            ("proc", "tcl8.6", "proc p {a} {expr {$a & 1 == 1}}"),
            (
                "namespace-proc",
                "tcl8.6",
                "namespace eval n {proc p {a} {if {$a & 1 == 1} {return 1}}}",
            ),
            ("quoted-word", "tcl8.6", "puts \"v=[expr {$a & 1 == 1}]\""),
            (
                "index-word",
                "tcl8.6",
                "set l {1 2}; lindex $l [expr {$a & 1 == 1}]",
            ),
            ("nested-expr", "tcl8.6", "expr {[expr {$a & 1 == 1}] + 1}"),
            ("math-function", "tcl8.6", "expr {int($a & 1 == 1)}"),
            ("apply", "tcl8.6", "apply {{a} {expr {$a & 1 == 1}}} 3"),
            ("catch", "tcl8.6", "catch {expr {$a & 1 == 1}}"),
            (
                "foreach",
                "tcl8.6",
                "foreach a {1 2} {if {$a & 1 == 1} {puts x}}",
            ),
            ("switch", "tcl8.6", "switch $x {a {expr {$a & 1 == 1}}}"),
            ("after", "tcl8.6", "after 10 {expr {$a & 1 == 1}}"),
            ("uplevel", "tcl8.6", "uplevel 1 {expr {$a & 1 == 1}}"),
            ("eval", "tcl8.6", "eval {expr {$a & 1 == 1}}"),
            ("try", "tcl8.6", "try {expr {$a & 1 == 1}} on error {} {}"),
            (
                "oo-method",
                "tcl8.6",
                "oo::class create C {method m {a} {expr {$a & 1 == 1}}}",
            ),
            ("lmap", "tcl8.6", "lmap a {1 2} {expr {$a & 1 == 1}}"),
            (
                "dict-for",
                "tcl8.6",
                "set d {k 1}; dict for {k a} $d {if {$a & 1 == 1} {puts x}}",
            ),
            ("joined-literals", "tcl8.6", "expr 1 & 1 == 1"),
            ("quoted-literal", "tcl8.6", "if \"1 & 1 == 1\" {puts x}"),
            ("tcl8.4", "tcl8.4", "if {$a & 1 == 1} {puts x}"),
            ("jim", "jim", "if {$a & 1 == 1} {puts x}"),
            (
                "irules-when",
                "f5-irules",
                "when HTTP_REQUEST { if {$a & 1 == 1} { log local0. x } }",
            ),
            (
                "irules-proc",
                "f5-irules",
                "proc p {a} { return [expr {$a & 1 == 1}] }",
            ),
            (
                "irules-word",
                "f5-irules",
                "when HTTP_REQUEST { if {not $a contains \"x\"} { log local0. x } }",
            ),
            (
                "irules-elseif",
                "f5-irules",
                "when HTTP_REQUEST { if {$b} { log local0. y } elseif {$a & 1 == 1} { log local0. x } }",
            ),
            (
                "irules-switch",
                "f5-irules",
                "when HTTP_REQUEST { switch [HTTP::host] { a { if {$a & 1 == 1} { log local0. x } } } }",
            ),
        ];
        let missing: Vec<_> = cases
            .iter()
            .filter(|(_, dialect, source)| {
                !codes(dialect, source)
                    .iter()
                    .any(|(code, _)| *code == DiagCode::W154)
            })
            .map(|(name, ..)| *name)
            .collect();
        assert!(missing.is_empty(), "no W154 in {missing:?}");
    }

    #[test]
    fn tiers_split_between_w154_and_w155() {
        assert_eq!(
            codes("tcl8.6", "expr {$a + $b * $c == 1 || $d && $e}"),
            vec![
                (DiagCode::W155, "+".to_owned()),
                (DiagCode::W155, "==".to_owned()),
                (DiagCode::W154, "||".to_owned()),
                (DiagCode::W155, "||".to_owned()),
            ]
        );
        assert_eq!(
            codes("tcl8.6", "expr {-2 ** 2 + $x}"),
            vec![
                (DiagCode::W154, "**".to_owned()),
                (DiagCode::W155, "+".to_owned()),
            ]
        );
    }

    #[test]
    fn joined_literals_have_no_fix_and_substituted_quotes_are_skipped() {
        let joined = crate::analyser::state::Analyser::new().analyse("expr 1 & 1 == 1", "tcl8.6");
        let diagnostic = joined
            .diagnostics
            .iter()
            .find(|d| d.code == DiagCode::W154)
            .unwrap();
        assert_eq!(&"expr 1 & 1 == 1"[diagnostic.span.as_range()], "1 & 1 == 1");
        assert!(diagnostic.fixes.is_empty());
        assert!(codes("tcl8.6", "if \"$a & 1 == 1\" {puts x}").is_empty());
    }
}
