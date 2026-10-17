// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configured completeness over an original image, independent of evaluation.
//!
//! Callers supply the complete selected configuration and original channel.
//! The C branch consumes structured issues issued by the lexical/component
//! producers and retained through the ordered cut. Their messages are display
//! data. Missing component kind or original geometry remains unavailable.
//! The Jim branch consumes the existing Script roster's missing marker.
//! Neither branch issues a value, command handler, entered source or edit owner.

use crate::{
    DocumentLineEndingProjection, JimScriptTokensUnavailable, LexError, LexerConfig, ParseCut,
    ParseCutCompleteness, ParseCutUnavailable, SourceChannel, SourceImage,
};

/// Missing selected grammar, original geometry or parse-kind correspondence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptCompletenessUnavailable {
    /// The C cut owner could not inspect the original lexical/components stream.
    ParseCut(ParseCutUnavailable),
    /// The existing Jim roster declined the supplied grammar or original input.
    JimScript(JimScriptTokensUnavailable),
    /// Original Document bytes have no checked Unicode newline presentation.
    Document,
}

impl std::fmt::Display for ScriptCompletenessUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseCut(error) => error.fmt(formatter),
            Self::JimScript(error) => {
                write!(formatter, "Jim script completeness unavailable: {error:?}")
            }
            Self::Document => formatter.write_str("original Document completeness unavailable"),
        }
    }
}

impl std::error::Error for ScriptCompletenessUnavailable {}

/// Query the supplied image under its complete original lexer configuration.
///
/// Native values retain counted bytes; Documents retain their independent
/// newline channel. C uses the first ordered cut's incomplete flag, while Jim
/// uses its existing Script missing marker. This creates no native value,
/// command selection, execution entry or source-edit receipt. The legacy
/// `script_is_complete` utility has a separate configless C9.0.3 contract.
///
/// # Errors
/// Returns unavailable selected grammar or original lexical geometry.
pub fn script_image_is_complete(
    image: &SourceImage,
    config: LexerConfig,
) -> Result<bool, ScriptCompletenessUnavailable> {
    // Validate a Document view without replacing the original parse image.
    let document = if image.channel() == SourceChannel::Document {
        Some(
            DocumentLineEndingProjection::new(image.clone())
                .ok_or(ScriptCompletenessUnavailable::Document)?,
        )
    } else {
        None
    };
    if config.var_syntax == tcl_dialect::VarSyntax::Jim {
        return crate::jim_script_tokens::jim_script_is_complete(image, config)
            .map_err(ScriptCompletenessUnavailable::JimScript);
    }
    let cut = match crate::first_parse_cut_image_checked(image, config) {
        Ok(cut) => cut.map(ParseCut::completeness),
        // Strict mode retains the first error from the same selected lexer.
        // Its known owner-issued kinds have the same readonly incomplete flag.
        Err(ParseCutUnavailable::LexicalStream(LexError::SyntaxError { kind, .. })) => {
            Some(if kind.is_incomplete() {
                ParseCutCompleteness::Incomplete
            } else {
                ParseCutCompleteness::Terminal
            })
        }
        Err(error) => return Err(ScriptCompletenessUnavailable::ParseCut(error)),
    };
    match cut {
        Some(ParseCutCompleteness::Incomplete) => Ok(false),
        Some(ParseCutCompleteness::Terminal) => Ok(true),
        None => {
            // Pending EOF continuation is a pure presentation projection.
            // Terminal cuts above take precedence over later incomplete text.
            let bytes = document
                .as_ref()
                .map_or_else(|| image.bytes(), |view| view.text().as_bytes());
            Ok(!crate::structural_index::ends_with_line_continuation(bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{Family, Release, grammar};

    #[test]
    fn original_parse_issue_kinds_keep_nested_order_and_diagnostic_presentations() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Typed producer/retention software control; this does not add an
        // external provider row or an execution/error publication contract.
        let config = LexerConfig::default();
        let image = SourceImage::native(b"puts $a([set q \"x)".as_slice());
        let cut = crate::first_parse_cut_image_checked(&image, config)
            .unwrap()
            .unwrap();
        assert_eq!(cut.kind, crate::ParseIssueKind::MissingQuote);
        assert_eq!(cut.message, crate::MISSING_QUOTE);
        assert_eq!(cut.completeness(), ParseCutCompleteness::Incomplete);
        assert_eq!(script_image_is_complete(&image, config), Ok(false));

        let image = SourceImage::native(b"set y {a}b {".as_slice());
        let cut = crate::first_parse_cut_image_checked(&image, config)
            .unwrap()
            .unwrap();
        assert_eq!(cut.kind, crate::ParseIssueKind::ExtraAfterBrace);
        assert_eq!(cut.message, crate::EXTRA_AFTER_CLOSE_BRACE);
        assert_eq!(cut.completeness(), ParseCutCompleteness::Terminal);
        assert_eq!(script_image_is_complete(&image, config), Ok(true));

        let image = SourceImage::native(b"set x \"".as_slice());
        let (_, warnings) = crate::Lexer::with_source_image(&image, config)
            .tokenise_all_with_warnings()
            .unwrap();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].kind, crate::ParseIssueKind::MissingQuote);
        assert_eq!(warnings[0].message, crate::MISSING_QUOTE);
        let strict = LexerConfig {
            strict_quoting: true,
            ..config
        };
        assert_eq!(
            crate::Lexer::with_source_image(&image, strict).tokenise_all(),
            Err(LexError::SyntaxError {
                kind: crate::ParseIssueKind::MissingQuote,
                message: crate::MISSING_QUOTE.to_owned(),
            })
        );
        assert_eq!(script_image_is_complete(&image, strict), Ok(false));
    }

    const ORIGINAL_CASES: [(&str, &str); 41] = [
        ("empty", ""),
        ("ordinary", "set x 1"),
        ("open_brace", "set x {"),
        ("brace_literal_bracket", "set x {[}"),
        ("brace_literal_quote", "set x {\"}"),
        ("open_quote", "set x \""),
        ("quoted_open_substitution", "set x \"[set y"),
        ("closed_quote_tail", "set x \"a\"b {"),
        ("closed_brace_tail", "set x {a}b {"),
        ("open_command", "set x [set y"),
        ("closed_command", "set x [set y]"),
        ("var_open", "set x ${a"),
        ("var_inner_open_first_close", "set x ${a{}"),
        ("var_inner_balanced", "set x ${a{b}c}"),
        ("var_escaped_close", "set x ${a\\}"),
        ("var_unicode_first_close", "set x ${é{}"),
        ("quoted_var_first_close", "set x \"${a{}\""),
        ("array_open", "set x $a("),
        ("array_closed", "set x $a(foo)"),
        ("array_open_command", "set x $a(foo[set y"),
        ("array_quote_literal", "set x $a(\"foo)"),
        ("array_tail_open_command", "set x $a(foo)tail[set y"),
        ("expansion_open_brace", "set x {*}{"),
        ("expansion_open_quote", "set x {*}\""),
        ("expansion_closed", "set x {*}{a b}"),
        ("trailing_backslash", "set x \\"),
        ("pending_continuation", "set x \\\n"),
        ("comment_pending_continuation", "# c\\\n"),
        ("comment_literal_delimiters", "# { [ \""),
        ("comment_crlf_continuation", "# c\\\r\n"),
        ("lone_cr", "set x {a\rb}"),
        ("crlf_brace", "set x {a\r\nb}"),
        ("nul_before_open", "set x 1\u{0000}{"),
        ("nul_inside_brace", "set x {a\u{0000}"),
        ("nul_after_open_command", "set x [\u{0000}"),
        ("nul_literal", "set x a\u{0000}b"),
        ("literal_variable_dollar", "set x $"),
        ("stray_close_brace", "set x }"),
        ("unicode_non_ascii", "set x é"),
        ("jim_sugar_open", "set x $(1+2"),
        ("jim_sugar_closed", "set x $(1+2)"),
    ];

    #[test]
    fn original_string_completeness_rows_keep_selected_c_and_jim_parser_purposes() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Original public String-value observations only. The Rust bytes below
        // carry the fixed request's scalar text; no native header, raw C source
        // encoding, execution or compilation equivalence is inferred.
        let providers = [
            (
                "8.4.20",
                Family::Tcl,
                Release::TCL_8_4,
                include_str!(
                    "../../tcl-registry/tests/data/native_configured_script_completeness_original/capture/8.4.20/execute.stdout"
                ),
            ),
            (
                "8.5.19",
                Family::Tcl,
                Release::TCL_8_5,
                include_str!(
                    "../../tcl-registry/tests/data/native_configured_script_completeness_original/capture/8.5.19/execute.stdout"
                ),
            ),
            (
                "8.6.18",
                Family::Tcl,
                Release::TCL_8_6,
                include_str!(
                    "../../tcl-registry/tests/data/native_configured_script_completeness_original/capture/8.6.18/execute.stdout"
                ),
            ),
            (
                "9.0.4",
                Family::Tcl,
                Release::TCL_9_0,
                include_str!(
                    "../../tcl-registry/tests/data/native_configured_script_completeness_original/capture/9.0.4/execute.stdout"
                ),
            ),
            (
                "9.1.0",
                Family::Tcl,
                Release::TCL_9_1,
                include_str!(
                    "../../tcl-registry/tests/data/native_configured_script_completeness_original/capture/9.1.0/execute.stdout"
                ),
            ),
            (
                "jim",
                Family::Jim,
                Release::JIM_0_84,
                include_str!(
                    "../../tcl-registry/tests/data/native_configured_script_completeness_original/capture/jim/execute.stdout"
                ),
            ),
        ];
        for (provider, family, release, stdout) in providers {
            let rows: Vec<_> = stdout
                .lines()
                .filter(|line| line.starts_with("ROW\t"))
                .collect();
            assert_eq!(rows.len(), ORIGINAL_CASES.len(), "{provider}");
            let config = LexerConfig::from_grammar(grammar(family, release));
            for (row, &(label, source)) in rows.iter().zip(&ORIGINAL_CASES) {
                let columns: Vec<_> = row.split('\t').collect();
                assert_eq!(columns.len(), 4, "{provider}: {row}");
                assert_eq!(columns[1], label, "{provider}: {row}");
                assert_eq!(columns[2], "0", "{provider}: {row}");
                let expected = match columns[3] {
                    "0" => false,
                    "1" => true,
                    _ => panic!("unexpected original result: {provider}: {row}"),
                };
                let image = SourceImage::native(source.as_bytes());
                assert_eq!(
                    script_image_is_complete(&image, config),
                    Ok(expected),
                    "{provider}/{label}"
                );
                if family == Family::Tcl {
                    assert_eq!(
                        script_image_is_complete(
                            &image,
                            LexerConfig {
                                strict_quoting: true,
                                ..config
                            }
                        ),
                        Ok(expected),
                        "strict {provider}/{label}"
                    );
                }
            }
        }
    }

    #[test]
    fn original_document_completeness_keeps_channel_and_complete_configuration() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Source-channel software contract; these are not public value rows.
        let source = "# c\\\r\n";
        let native = SourceImage::native(source.as_bytes());
        let document = SourceImage::document(source);
        let config = LexerConfig::default();
        assert_eq!(script_image_is_complete(&native, config), Ok(true));
        assert_eq!(script_image_is_complete(&document, config), Ok(false));
        assert_eq!(document.bytes(), source.as_bytes());
        for source in ["# c\\\n", "# c\\\r", "set x $a(", "set x [set y"] {
            assert_eq!(
                script_image_is_complete(&SourceImage::document(source), config),
                Ok(false),
                "{source:?}"
            );
        }
        let source = SourceImage::document("set x ${a{}");
        let first_close = LexerConfig {
            braced_var: crate::BracedVarStyle::FirstClose,
            ..config
        };
        assert_eq!(script_image_is_complete(&source, first_close), Ok(true));
        assert_eq!(script_image_is_complete(&source, config), Ok(false));
        let source = SourceImage::document("set x {*}{");
        assert_eq!(script_image_is_complete(&source, config), Ok(false));
        assert_eq!(
            script_image_is_complete(
                &source,
                LexerConfig {
                    expand_syntax: false,
                    ..config
                }
            ),
            Ok(true)
        );
    }

    #[test]
    fn original_completeness_refuses_foreign_jim_grammar_and_opaque_document_input() {
        // naming.source.original-configured-script-completeness
        // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
        // Missing grammar/Document correspondence cannot fall back to C9.
        let config = LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84));
        let source = SourceImage::native(b"set x ${a".as_slice());
        assert_eq!(script_image_is_complete(&source, config), Ok(true));
        assert_eq!(
            script_image_is_complete(
                &source,
                LexerConfig {
                    braced_var: crate::BracedVarStyle::Tcl9Nesting,
                    ..config
                }
            ),
            Err(ScriptCompletenessUnavailable::JimScript(
                JimScriptTokensUnavailable::Grammar
            ))
        );
        let opaque = SourceImage::from_bytes(b"\xff".as_slice(), SourceChannel::Document);
        assert_eq!(
            script_image_is_complete(&opaque, LexerConfig::default()),
            Err(ScriptCompletenessUnavailable::Document)
        );
        let opaque = SourceImage::native(b"\xff".as_slice());
        assert_eq!(
            script_image_is_complete(&opaque, LexerConfig::default()),
            Ok(true)
        );
    }
}
