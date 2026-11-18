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

use crate::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_core_types::DiagCode;

fn input() -> ResolvedAnalysisInput {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    config.strict_quoting = !config.strict_quoting;
    ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        config,
    )
}

#[test]
fn actual_formal_and_lambda_repairs_are_complete_configured_words() {
    for source in ["proc target {{a b c}} {}", "apply {{{a b c}} {return 1}}"] {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.strict_quoting = !config.strict_quoting;
        let actual = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            config,
        );
        let result = Analyser::new()
            .with_resolved_input(actual.clone())
            .analyse(source, "tcl");
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagCode::E006)
            .unwrap();
        let [fix] = diagnostic.fixes.as_slice() else {
            panic!("original repair: {source}")
        };
        let expected = if source.starts_with("proc") {
            "a b c".to_owned()
        } else {
            tcl_syntax::list::join_list(["a b c", "return 1"])
        };
        assert!(tcl_syntax::word_rules::lexical_unicode_source_word_matches(
            &fix.new_text,
            &expected,
            actual.lexer_config(),
        ));
        assert_eq!(fix.span, diagnostic.span);
    }
}

#[test]
fn selector_and_option_proposals_keep_written_origin_separate_from_capture() {
    for (written, captured, code) in [
        (
            "string indez abc 0",
            "interp alias {} selected {} string indez; selected abc 0",
            DiagCode::W001,
        ),
        (
            "lsort -d {b a}",
            "interp alias {} selected {} lsort -d; selected {b a}",
            DiagCode::W145,
        ),
    ] {
        let actual = input();
        let direct = Analyser::new()
            .with_resolved_input(actual.clone())
            .analyse(written, "tcl");
        let diagnostic = direct
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap();
        assert!(!diagnostic.fixes.is_empty(), "{written}");
        for fix in &diagnostic.fixes {
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(&fix.new_text),
                tcl_lexer::Span::new(0, u32::try_from(fix.new_text.len()).unwrap()),
                actual.lexer_config(),
            )
            .unwrap();
            assert_eq!(plan.commands.len(), 1);
            assert_eq!(plan.commands[0].words.len(), 1);
            assert!(plan.fatal_tail.is_none());
        }
        let indirect = Analyser::new()
            .with_resolved_input(actual)
            .analyse(captured, "tcl");
        let diagnostic = indirect
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code);
        if code == DiagCode::W001 {
            // A captured selector cannot borrow a selector anchor
            // (diagnostic-registry-source-ownership.md); tclsh still raises
            // unknown or ambiguous subcommand "indez" at the call.
            assert!(diagnostic.is_none(), "{captured}: {diagnostic:?}");
            continue;
        }
        let diagnostic = diagnostic.unwrap();
        assert!(diagnostic.fixes.is_empty(), "{captured}");
    }
}
