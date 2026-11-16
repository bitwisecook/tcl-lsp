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

fn parse(source: &str, config: tcl_lexer::LexerConfig) -> Option<(String, String)> {
    let script = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        config,
    )
    .ok()?;
    let [command] = script.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    parse_namespaced_ensemble(word)
}

#[test]
fn ensemble_heads_use_original_whole_scalar_and_literal_component_geometry() {
    for (dialect, nested) in [("tcl8.6", false), ("tcl9.1", true), ("jim", false)] {
        let config = tcl_lexer::LexerConfig::for_profile(Some(
            tcl_registry::model::ingress::resolve_known_environment(dialect)
                .expect("selected original reference grammar environment")
                .unit_profile(),
        ));
        for (source, name) in [
            ("${prefix}::tail", "prefix"),
            ("${ with space }::tail", " with space "),
            ("${$literal}::tail", "$literal"),
            ("${literal(open}::tail", "literal(open"),
            ("${}::tail", ""),
        ] {
            assert_eq!(
                parse(source, config),
                Some((name.to_owned(), "tail".to_owned())),
                "{dialect} {source}"
            );
        }
        assert_eq!(
            parse("${a{b}c}::tail", config),
            nested.then(|| ("a{b}c".to_owned(), "tail".to_owned())),
            "{dialect}"
        );
        for source in [
            "$prefix::tail",
            "$café::tail",
            "${array(key)}::tail",
            "$array(key)::tail",
            "${prefix}::$tail",
            "${prefix}::[tail]",
            "${prefix}::",
            "{${prefix}::tail}",
            "\"${prefix}::tail\"",
            "{*}${prefix}::tail",
            "${prefix}::ta\\il",
        ] {
            assert!(parse(source, config).is_none(), "{dialect} {source}");
        }
    }
}

#[test]
fn command_ensemble_heads_use_original_command_and_literal_suffix_components() {
    let config = tcl_lexer::LexerConfig::default();
    for (source, expected) in [
        ("[set prefix]::tail", true),
        ("[list ::N]::tail", true),
        (r#"[format %s " ]::inert"]::tail"#, true),
        ("[list ::N]", false),
        ("[set prefix]::$tail", false),
        ("[set prefix]::[tail]", false),
        ("[set prefix]::ta\\il", false),
        ("{[set prefix]::tail}", false),
        (r#""[set prefix]::tail""#, false),
        ("{*}[set prefix]::tail", false),
    ] {
        let script = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        assert_eq!(
            command_namespaced_ensemble(&script.commands[0].words[0]),
            expected,
            "{source}"
        );
    }
}

#[test]
fn ensemble_head_source_projection_refuses_stale_source_and_original_offsets() {
    let source = "namespace eval N {proc tail {} {}}; set prefix ::N; ${prefix}::tail";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut analyser = Analyser::new().with_resolved_input(input);
    analyser.analyse(source, profile.name);
    let site = analyser
        .var_command_sites
        .iter()
        .find(|site| site.cmd_span.start() as usize == source.find("${prefix}").unwrap())
        .expect("actual original dynamic command head")
        .clone();
    assert_eq!(
        analyser.original_namespaced_ensemble(&site),
        Some(("prefix".to_owned(), "tail".to_owned()))
    );
    let mut shifted = site.clone();
    shifted.cmd_span = tcl_lexer::Span::new(site.cmd_span.start() + 1, site.cmd_span.end());
    assert!(analyser.original_namespaced_ensemble(&shifted).is_none());
    analyser.source = format!("{}{}", analyser.source, ' ').into();
    assert!(analyser.original_namespaced_ensemble(&site).is_none());
}
