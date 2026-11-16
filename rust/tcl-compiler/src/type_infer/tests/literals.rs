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

//! Literal classification: original literal contents, numerals per release and the integer tower.

use super::*;

#[test]
fn original_literal_contents_keep_point_grammar_and_withdraw_stale_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig {
        escapes: tcl_dialect::EscapeSyntax::Tcl84,
        ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
    };
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
        .with_resolved_analysis_input(input);
    let source = r"set value \U00000041";
    lowerer.lower(source);
    let module = lowerer.finish_module(source);
    let tokens = module
        .top_level
        .statements
        .iter()
        .find_map(|statement| {
            module
                .top_level
                .retained_source_tokens_for_statement(statement)
        })
        .expect("actual original setter source");
    let literal = original_literal_argument_contents(context.commands(), &module, tokens, 1)
        .expect("original lexical value has complete source ownership");
    assert_eq!(literal.value(), "U00000041");
    assert_eq!(literal.contents_type().tcl_type(), Some(TclType::String));
    let mut changed = module.clone();
    changed.source_metadata_input = None;
    assert!(original_literal_argument_contents(context.commands(), &changed, tokens, 1).is_none());
    let mut changed = module.clone();
    changed.lexer_config.escapes = tcl_dialect::EscapeSyntax::Tcl86;
    assert!(original_literal_argument_contents(context.commands(), &changed, tokens, 1).is_none());
    let mut changed = module.clone();
    changed.source = tcl_lexer::SourceImage::document("set value OTHER");
    assert!(original_literal_argument_contents(context.commands(), &changed, tokens, 1).is_none());
    let foreign = tcl_registry::CommandRegistry::build_default();
    assert!(original_literal_argument_contents(&foreign, &module, tokens, 1).is_none());
    let mut changed = tokens.clone();
    changed.argv_texts[2] = "OTHER".to_owned();
    assert!(original_literal_argument_contents(context.commands(), &module, &changed, 1).is_none());
}

#[test]
fn original_literal_contents_do_not_borrow_numeric_grammar_or_substituted_values() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    // Independently authored source numerals remain Tcl85 while command
    // availability is C9. This selects no Native parser or execution.
    let mut authored = tcl_dialect::DialectProfile::plain_tcl().clone();
    authored.grammar.numbers = NumberSyntax::Tcl85;
    let profile: &'static tcl_dialect::DialectProfile = Box::leak(Box::new(authored));
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    for (source, expected) in [
        ("set value 1_000", Some(TclType::String)),
        ("set value 2", Some(TclType::Int)),
        ("set value $external", None),
        ("set value [list x]", None),
        (r"set value \x00", None),
    ] {
        let mut lowerer = crate::lowering::Lowerer::with_config(context.commands(), config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let tokens = module
            .top_level
            .statements
            .iter()
            .find_map(|statement| {
                module
                    .top_level
                    .retained_source_tokens_for_statement(statement)
            })
            .expect("whole original source vector");
        let contents = original_literal_argument_contents(context.commands(), &module, tokens, 1);
        assert_eq!(
            contents
                .as_ref()
                .and_then(|value| value.contents_type().tcl_type()),
            expected,
            "{source}"
        );
        if let Some(contents) = contents {
            assert_eq!(contents.value(), tokens.argv_texts[2]);
        }
    }
}

#[test]
fn float_literal_infers_double() {
    let t = literal_type("3.14", NumberSyntax::Tcl90);
    assert_eq!(t, TypeLattice::of(TclType::Double));
}

#[test]
fn exponent_and_infinite_literals_infer_double_2500() {
    for text in ["1e3", "Inf", "-Inf", "+inf"] {
        assert_eq!(
            literal_type(text, NumberSyntax::Tcl90),
            TypeLattice::of(TclType::Double),
            "{text}"
        );
    }
}

#[test]
fn bool_literal_infers_boolean() {
    let t = literal_type("true", NumberSyntax::Tcl90);
    assert_eq!(t, TypeLattice::of(TclType::Boolean));
    let t2 = literal_type("false", NumberSyntax::Tcl90);
    assert_eq!(t2, TypeLattice::of(TclType::Boolean));
}

#[test]
fn string_literal_infers_string() {
    let t = literal_type("hello", NumberSyntax::Tcl90);
    assert_eq!(t, TypeLattice::of(TclType::String));
}

#[test]
fn hex_and_binary_literals_infer_int() {
    // Hex / binary literals store an INT intrep (`set n 0x80; incr n` is
    // one clean parse, not per-iteration shimmer).
    for lit in ["0x80", "0X1f", "-0xFF", "0b1010", "0B1", "+0x0"] {
        assert_eq!(
            literal_type(lit, NumberSyntax::Tcl90),
            TypeLattice::of(TclType::Int),
            "{lit} should be INT"
        );
    }
    // Octal `0o…` and non-integer hex stay STRING (set-statement classifier).
    assert_eq!(
        literal_type("0o17", NumberSyntax::Tcl90),
        TypeLattice::of(TclType::String)
    );
    assert_eq!(
        literal_type("0xZZ", NumberSyntax::Tcl90),
        TypeLattice::of(TclType::String)
    );
}

/// The literal classifiers read numerals under the *target release's*
/// grammar, and the set/expr divergence on `0o…` survives it.
///
/// The matrix (from the C release sources): `0x` in every release; `0o`/`0b`
/// from 8.5; `0d` and `_` separators from 9.0; a bare leading zero is octal
/// up to 8.6 and decimal from 9.0. A spelling the release does not have is
/// not a numeral — a bareword in expr context (`Numeric`, since an `expr`
/// still yields a number) and a plain `String` after `set`.
#[test]
fn literal_classifiers_follow_the_release_numeral_matrix() {
    use NumberSyntax::{Tcl84, Tcl85, Tcl90};
    let set_type = |lit: &str, n| literal_type(lit, n).tcl_type();
    let expr_type = |lit: &str, n| expr_literal_type(lit, n).tcl_type();

    // `0o17`: the deliberate divergence. From 8.5 it is an `Int` in expr
    // context but stays `String` after `set` (its canonical intrep, `15`,
    // differs from the source text). Under 8.4 there is no `0o` prefix at
    // all, so it is a bareword either way.
    for syntax in [Tcl85, Tcl90] {
        assert_eq!(
            set_type("0o17", syntax),
            Some(TclType::String),
            "{syntax:?}"
        );
        assert_eq!(expr_type("0o17", syntax), Some(TclType::Int), "{syntax:?}");
    }
    assert_eq!(set_type("0o17", Tcl84), Some(TclType::String));
    assert_eq!(expr_type("0o17", Tcl84), Some(TclType::Numeric));

    // `0d99` — the 9.0-only decimal prefix.
    assert_eq!(set_type("0d99", Tcl90), Some(TclType::Int));
    assert_eq!(expr_type("0d99", Tcl90), Some(TclType::Int));
    for syntax in [Tcl84, Tcl85] {
        assert_eq!(
            set_type("0d99", syntax),
            Some(TclType::String),
            "{syntax:?}"
        );
        assert_eq!(
            expr_type("0d99", syntax),
            Some(TclType::Numeric),
            "{syntax:?}"
        );
    }

    // `1_000` — 9.0-only digit separators.
    assert_eq!(set_type("1_000", Tcl90), Some(TclType::Int));
    assert_eq!(expr_type("1_000", Tcl90), Some(TclType::Int));
    for syntax in [Tcl84, Tcl85] {
        assert_eq!(
            set_type("1_000", syntax),
            Some(TclType::String),
            "{syntax:?}"
        );
        assert_eq!(
            expr_type("1_000", syntax),
            Some(TclType::Numeric),
            "{syntax:?}"
        );
    }

    // `0b101` — 8.5 onward.
    assert_eq!(set_type("0b101", Tcl84), Some(TclType::String));
    assert_eq!(expr_type("0b101", Tcl84), Some(TclType::Numeric));
    for syntax in [Tcl85, Tcl90] {
        assert_eq!(set_type("0b101", syntax), Some(TclType::Int), "{syntax:?}");
        assert_eq!(expr_type("0b101", syntax), Some(TclType::Int), "{syntax:?}");
    }

    // A radix-invalid digit is never a numeral, in any release.
    for syntax in [Tcl84, Tcl85, Tcl90] {
        for bad in ["0o8", "0xZZ"] {
            assert_eq!(
                set_type(bad, syntax),
                Some(TclType::String),
                "{bad} under {syntax:?}"
            );
            assert_eq!(
                expr_type(bad, syntax),
                Some(TclType::Numeric),
                "{bad} under {syntax:?}"
            );
        }
    }

    // A bare leading zero is an integer in every release (only its *value*
    // differs: 493 up to 8.6, 755 from 9.0 — see `intervals`), while `08`
    // is a broken octal up to 8.6 and plain decimal 8 from 9.0.
    for syntax in [Tcl84, Tcl85, Tcl90] {
        assert_eq!(set_type("0755", syntax), Some(TclType::Int), "{syntax:?}");
        assert_eq!(expr_type("0755", syntax), Some(TclType::Int), "{syntax:?}");
    }
    for syntax in [Tcl84, Tcl85] {
        assert_eq!(set_type("08", syntax), Some(TclType::String), "{syntax:?}");
        assert_eq!(
            expr_type("08", syntax),
            Some(TclType::Numeric),
            "{syntax:?}"
        );
    }
    assert_eq!(set_type("08", Tcl90), Some(TclType::Int));
    assert_eq!(expr_type("08", Tcl90), Some(TclType::Int));
}

/// Bignum literals classify on the tower: a beyond-wide decimal is
/// `Bignum` (coarse `Int`), never wrapped and never `String`.
#[test]
fn bignum_literal_classifies_on_the_tower() {
    assert_eq!(
        literal_type("18446744073709551616", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Bignum)
    );
    assert_eq!(
        literal_type("0xFFFFFFFFFFFFFFFF", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Bignum)
    );
    assert_eq!(
        literal_type("5", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Int)
    );
    assert_eq!(
        literal_type("-9223372036854775808", NumberSyntax::Tcl90).single_shape(),
        Some(&TypeShape::Int),
        "i64::MIN still fits a wide"
    );
}
