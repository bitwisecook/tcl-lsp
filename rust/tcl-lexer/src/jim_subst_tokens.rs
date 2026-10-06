// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim substitution tokens, before backend object construction.

use crate::{JimScriptToken, JimScriptTokensUnavailable, Lexer, LexerConfig, SourceImage};

/// Flat native substitution roster. Its token objects have no Source cache;
/// filename ownership belongs to the concrete substitution backing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimSubstTokens {
    /// Original counted native input.
    pub image: SourceImage,
    /// Real tokens only; native substitution omits its final EOL token.
    pub tokens: Vec<JimScriptToken>,
    /// Native NOVAR/NOCMD/NOESC and real-substitution bits, independently
    /// retained by the backing. The latter changes interpolation control flow.
    pub flags: u8,
}

/// Project pinned Jim's substitution parser through the existing nested
/// command and variable scanners. Ordinary text retains braces, quotes,
/// separators and raw newlines; it is not a Script command layout.
///
/// # Errors
/// Rejects foreign grammar, document input, unsupported flags or unavailable
/// original geometry. Backend objects and lookup authority are not issued.
pub fn jim_subst_tokens(
    image: &SourceImage,
    config: LexerConfig,
    flags: u8,
) -> Result<JimSubstTokens, JimScriptTokensUnavailable> {
    if image.channel() != crate::SourceChannel::NativeValue {
        return Err(JimScriptTokensUnavailable::SourceChannel);
    }
    Lexer::with_source_image(image, config).jim_subst_roster(image.clone(), flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JimScriptTokenKind;
    use tcl_dialect::model::{Family, Release, grammar};

    #[test]
    fn original_substitution_roster_matches_native_flag_and_line_windows() {
        let sources: [&[u8]; 4] = [
            b"a\n$k[set x X]\\n{q};z",
            b"A\0B$k[set x X]",
            b"${x\ny}P\nQ",
            b"d($k)",
        ];
        let config = LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84));
        let rows = include_str!("../testdata/native_jim_subst/tokens.tsv");
        let mut compared = 0;
        for (case, source) in sources.into_iter().enumerate() {
            for flags in 0..8 {
                let roster = jim_subst_tokens(&SourceImage::native(source), config, flags).unwrap();
                let native: Vec<Vec<u32>> = rows
                    .lines()
                    .map(|line| {
                        line.split('\t')
                            .skip(1)
                            .map(|n| n.parse().unwrap())
                            .collect()
                    })
                    .filter(|row: &Vec<u32>| {
                        row[0] == u32::try_from(case).expect("fixture case index")
                            && row[1] == u32::from(flags)
                    })
                    .collect();
                assert_eq!(
                    roster.tokens.len(),
                    native.len(),
                    "case {case} flags {flags}"
                );
                for (token, row) in roster.tokens.iter().zip(native) {
                    let kind = match token.kind {
                        JimScriptTokenKind::String => 1,
                        JimScriptTokenKind::Escaped => 2,
                        JimScriptTokenKind::Variable => 3,
                        JimScriptTokenKind::IndexedVariable => 4,
                        JimScriptTokenKind::Command => 5,
                        JimScriptTokenKind::Expression => 17,
                        other => panic!("unexpected substitution separator: {other:?}"),
                    };
                    assert_eq!(
                        (
                            kind,
                            token.line_delta + 1,
                            token.source.start(),
                            token.source.end(),
                            token.value.start(),
                            token.value.end()
                        ),
                        (row[3], row[4], row[5], row[6], row[7], row[8]),
                        "case {case} flags {flags} token {}",
                        row[2],
                    );
                    compared += 1;
                }
            }
        }
        assert_eq!(compared, rows.lines().count());
    }

    #[test]
    fn substitution_roster_requires_native_input_and_selected_jim_flags() {
        let config = LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84));
        assert_eq!(
            jim_subst_tokens(&SourceImage::native(&b"$x"[..]), config, 8),
            Err(JimScriptTokensUnavailable::Grammar),
        );
        assert!(
            jim_subst_tokens(&SourceImage::native(&b"$x"[..]), LexerConfig::default(), 0).is_err()
        );
    }

    #[test]
    fn real_substitution_flag_preserves_native_parser_tokens() {
        let config = LexerConfig::from_grammar(grammar(Family::Jim, Release::JIM_0_84));
        let image = SourceImage::native(&b"$x[break]\\n"[..]);
        for flags in 0..8 {
            let indexed = jim_subst_tokens(&image, config, flags).unwrap();
            let command = jim_subst_tokens(&image, config, flags | 128).unwrap();
            assert_eq!(indexed.tokens, command.tokens);
            assert_eq!(command.flags, flags | 128);
        }
    }
}
