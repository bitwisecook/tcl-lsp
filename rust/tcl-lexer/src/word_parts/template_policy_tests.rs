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

#[test]
fn c_template_retains_disabled_sigil_and_escape_token_boundaries() {
    // Source proof: naming.substitution.template-token-and-index-source
    // docs/design/analysis/name-resolution-proofs/substitution-template-token-and-index-source.md
    // Native proof: naming.substitution.counted-template-completions
    // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
    // v6 source0/flags5 has four native pushes before strcat, so C9
    // returns a String rather than one unchanged literal with no primary.
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let config = LexerConfig::for_dialect(dialect);
        let flags = SubstFlags {
            vars: false,
            ..SubstFlags::default()
        };
        let source = b"${x}${y}";
        let parts = decompose_template_spanned(source, flags, config, TemplateVariableSyntax::CTcl)
            .unwrap();
        assert_eq!(
            parts
                .iter()
                .map(|part| (part.start, part.end))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 4), (4, 5), (5, 8)]
        );
        let values: Vec<_> = parts
            .iter()
            .map(|part| match &part.part {
                WordPart::Text(value) => value.as_ref(),
                _ => panic!("disabled variable is a native text token"),
            })
            .collect();
        assert_eq!(values, [b"$".as_slice(), b"{x}", b"$", b"{y}"]);
        assert_eq!(
            decompose_template_spanned(source, flags, config, TemplateVariableSyntax::WrittenWord)
                .unwrap()
                .len(),
            1
        );
        for (source, expected) in [
            (br"a\nb".as_slice(), [b"a".as_slice(), b"\n", b"b"]),
            (b"a\0b".as_slice(), [b"a".as_slice(), b"\0", b"b"]),
        ] {
            let parts = decompose_template_spanned(
                source,
                SubstFlags::default(),
                config,
                TemplateVariableSyntax::CTcl,
            )
            .unwrap();
            let values: Vec<_> = parts
                .iter()
                .map(|part| match &part.part {
                    WordPart::Text(value) => value.as_ref(),
                    _ => panic!("retained literal/escape token"),
                })
                .collect();
            assert_eq!(values, expected);
        }
        let disabled = SubstFlags {
            vars: false,
            cmds: false,
            backslashes: false,
            ..SubstFlags::default()
        };
        let parts =
            decompose_template_spanned(b"$[\\", disabled, config, TemplateVariableSyntax::CTcl)
                .unwrap();
        assert_eq!(parts.len(), 3);
        assert!(
            parts
                .iter()
                .all(|part| matches!(part.part, WordPart::Text(_)))
        );
    }
}

#[test]
fn jim_expression_components_retain_parentheses_and_native_token_sites() {
    // Native proof: naming.variable.empty-array-root-lexical-reference
    // docs/design/analysis/name-resolution-proofs/empty-array-root-lexical-reference.md
    let source = b"pre$(1+(2))post";
    let jim = LexerConfig::for_dialect("jim");
    let parts = decompose_spanned(source, SubstFlags::default(), jim);
    assert_eq!(parts[1].part, WordPart::Expression(b"(1+(2))"));
    assert_eq!((parts[1].start, parts[1].end), (3, 11));
    assert_eq!(
        parts[1].source_span(source, 0, 100),
        Some(crate::Span::new(103, 110))
    );
    assert_eq!(
        scan_expression_sugar(b"$()", 0, jim),
        Ok(Some((b"()".as_slice(), 3)))
    );
    assert_eq!(
        scan_expression_sugar(b"$(1", 0, jim),
        Err("missing close-paren for expression substitution")
    );
    let disabled = SubstFlags {
        vars: false,
        ..SubstFlags::default()
    };
    assert_eq!(decompose(source, disabled, jim), WordBody::Literal(source));
    assert!(
        matches!(decompose(b"$(k)", SubstFlags::default(), LexerConfig::default()), WordBody::Parts(parts) if matches!(&parts[0], WordPart::Variable(reference) if reference.name.is_empty()))
    );
}

#[test]
fn jim_template_acceptance_does_not_relax_written_words() {
    let source = b"prefix ${x y";
    let config = LexerConfig::default();
    let parts = decompose_template_spanned(
        source,
        SubstFlags::default(),
        config,
        TemplateVariableSyntax::Jim084,
    )
    .unwrap();
    let WordPart::Variable(reference) = &parts[1].part else {
        panic!("native scalar reference");
    };
    assert_eq!(reference.name, b"x y");
    assert_eq!(
        parts[1].template_source_span(source, 10, TemplateVariableSyntax::Jim084),
        Some(crate::Span::new(17, 22))
    );
    assert!(matches!(
        decompose_spanned(source, SubstFlags::default(), config)
            .last()
            .unwrap()
            .part,
        WordPart::ParseError(_)
    ));
    let parts = decompose_template_spanned(
        b"${a{b}c}",
        SubstFlags::default(),
        config,
        TemplateVariableSyntax::Jim084,
    )
    .unwrap();
    assert!(matches!(&parts[0].part, WordPart::Variable(reference) if reference.name == b"a{b"));
    assert!(matches!(&parts[1].part, WordPart::Text(bytes) if bytes.as_ref() == b"c}"));
    let flags = SubstFlags {
        vars: false,
        ..SubstFlags::default()
    };
    assert!(
        matches!(&decompose_template_spanned(source, flags, config, TemplateVariableSyntax::Jim084).unwrap()[0].part, WordPart::Text(bytes) if bytes.as_ref() == source)
    );
    let flags = SubstFlags {
        backslashes: false,
        cmds: false,
        ..SubstFlags::default()
    };
    assert!(
        matches!(&decompose_template_spanned(b"\\${x", flags, config, TemplateVariableSyntax::Jim084).unwrap()[0].part, WordPart::Text(bytes) if bytes.as_ref() == b"\\${x")
    );
    assert_eq!(
        decompose_template_spanned(b"$[1+2]", flags, config, TemplateVariableSyntax::Jim084),
        Err(UnsupportedTemplateSyntax::ExpressionSugar)
    );
}
