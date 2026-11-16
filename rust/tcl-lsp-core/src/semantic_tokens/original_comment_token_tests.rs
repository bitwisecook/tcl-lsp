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

use super::*;

fn analysis(source: &str) -> AnalysisResult {
    let mut registry = CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "held-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let profile = crate::profile_for_dialect("tcl8.6");
    crate::source_structure::analyse_document(
        source,
        profile,
        &registry,
        tcl_lexer::LexerConfig {
            strict_quoting: true,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        },
    )
}

fn comments(source: &str, analysis: &AnalysisResult) -> Vec<String> {
    let index = LineIndex::new(source);
    let mut entries = Vec::new();
    push_comment_tokens(source, &index, Some(analysis), &mut entries);
    entries
        .into_iter()
        .map(|(line, column, length, kind, _)| {
            assert_eq!(kind, TokenKind::Comment);
            let start = index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(column), source);
            let end =
                index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(column + length), source);
            source[start as usize..end as usize].to_owned()
        })
        .collect()
}

#[test]
fn original_comment_tokens_share_real_phase_and_preserve_inline_comments() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "set data {\n# inert brace\n}\nset data \"quoted\n# inert quote\"\n# physical\nputs ok ;# tail\nheld-script {\n# selected body\nputs é ;# selected tail\n}\nunknown-script {\n# unavailable body\n}\n";
    let analysis = analysis(source);
    assert_eq!(
        comments(source, &analysis),
        vec!["# physical", "# tail", "# selected body", "# selected tail"]
    );
    let physical =
        tcl_compiler::analyser::utils::script_comment_facts_from_analysis(source, &analysis)
            .unwrap();
    assert_eq!(
        physical
            .iter()
            .map(|fact| fact.text.as_str())
            .collect::<Vec<_>>(),
        vec!["# physical", "# selected body"]
    );
    let shadowed =
        "proc held-script args {}\nheld-script {\n# no selected body\n}\nputs ok ;# still a tail\n";
    let shadowed_analysis = self::analysis(shadowed);
    assert_eq!(
        comments(shadowed, &shadowed_analysis),
        vec!["# still a tail"]
    );
}

#[test]
fn original_comment_tokens_refuse_foreign_source_or_missing_full_context() {
    // naming.core.original-comment-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-comment-source-context.md
    let source = "# retained\nheld-script {\n# owned\n}\n";
    let analysis = analysis(source);
    assert_eq!(comments(source, &analysis), vec!["# retained", "# owned"]);
    assert_eq!(
        comments(&format!("{source} "), &analysis),
        [] as [std::string::String; 0]
    );
    let mut changed = analysis.clone();
    changed.resolved_input = None;
    assert_eq!(comments(source, &changed), [] as [std::string::String; 0]);
    let mut changed = analysis.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting = false;
    assert_eq!(comments(source, &changed), [] as [std::string::String; 0]);
    let mut changed = analysis.clone();
    let profile = crate::profile_for_dialect("tcl8.6");
    changed.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::context_for_profile(profile),
        analysis.body_lexer_config.unwrap(),
    ));
    assert_eq!(comments(source, &changed), [] as [std::string::String; 0]);
}
