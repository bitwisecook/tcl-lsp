// SPDX-License-Identifier: AGPL-3.0-or-later
//! C9 `TclCompileRegexpCmd` selection and ordered stack recipe.

use crate::native_compilation::{
    NativeCompilationGuard, NativeCompilationSelection, NativeCompilationWordShape,
};
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use crate::semantic_operation::SemanticOperationId;
use tcl_dialect::TclVersion;

/// `TCL_REG_ADVANCED`, always passed by the compiled regexp instruction.
pub const REGEXP_ADVANCED: i32 = 3;
/// `TCL_REG_NOCASE`, added for an accepted `-nocase` option.
pub const REGEXP_NOCASE: i32 = 0o10;

/// Matcher selected by the C compiler after its `TclReToGlob` attempt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeRegexpMatcher {
    /// An anchored literal without `-nocase`: push the converted bytes, then `STR_EQ`.
    Equal(Vec<u8>),
    /// Push the converted glob, then `STR_MATCH` with the case flag.
    Glob {
        /// Converted glob pattern bytes.
        pattern: Vec<u8>,
        /// Whether `-nocase` was accepted.
        nocase: bool,
    },
    /// Compile the original pattern word, then `REGEXP` with its flags.
    Regexp {
        /// Original pattern word, compiled before the subject.
        pattern: NativeCompilerWordOperand,
        /// Whether `-nocase` was accepted.
        nocase: bool,
    },
}

impl NativeRegexpMatcher {
    /// The `REGEXP` immediate, or `None` for a glob reduction.
    #[must_use]
    pub const fn regexp_flags(&self) -> Option<i32> {
        match self {
            Self::Regexp { nocase, .. } => Some(if *nocase {
                REGEXP_ADVANCED | REGEXP_NOCASE
            } else {
                REGEXP_ADVANCED
            }),
            Self::Equal(_) | Self::Glob { .. } => None,
        }
    }
}

/// Pattern then subject, followed by the selected matcher instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeRegexpInstruction {
    /// Selected matcher, retaining either converted bytes or the original pattern word.
    pub matcher: NativeRegexpMatcher,
    /// Original subject word, compiled after the pattern.
    pub subject: NativeCompilerWordOperand,
}

impl NativeRegexpInstruction {
    /// Original words compiled by this recipe, in stack order.
    pub fn original_operands(&self) -> impl Iterator<Item = &NativeCompilerWordOperand> {
        let pattern = match &self.matcher {
            NativeRegexpMatcher::Regexp { pattern, .. } => Some(pattern),
            NativeRegexpMatcher::Equal(_) | NativeRegexpMatcher::Glob { .. } => None,
        };
        pattern.into_iter().chain(std::iter::once(&self.subject))
    }
}

/// Whether this release's regexp compiler is modelled here.
#[must_use]
pub const fn modelled(version: TclVersion) -> bool {
    matches!(version, TclVersion::V9_0 | TclVersion::V9_1)
}

/// The bytes of a `TCL_TOKEN_SIMPLE_WORD`, or `None` for any other token.
///
/// `ParseTokens` gives a `$` that starts no variable, and a raw NUL, a TEXT
/// token of its own, so an unbraced word holding one beside other text has
/// several components. A parser-expanded list member is always simple.
pub(crate) fn simple_word_bytes(
    shape: NativeCompilationWordShape,
    value: Option<&[u8]>,
    parser_expanded: bool,
) -> Option<&[u8]> {
    use NativeCompilationWordShape as Shape;
    let value = value?;
    match shape {
        Shape::BracedLiteral => Some(value),
        Shape::Literal | Shape::QuotedLiteral
            if parser_expanded
                || value.len() == 1
                || !value.iter().any(|byte| matches!(byte, b'$' | 0)) =>
        {
            Some(value)
        }
        _ => None,
    }
}

fn projected_simple(word: &NativeProjectedCompilerWord) -> Option<&[u8]> {
    simple_word_bytes(
        word.shape,
        word.literal.as_deref(),
        matches!(
            word.operand,
            NativeCompilerWordOperand::LiteralExpansion { .. }
        ),
    )
}

/// Option prefix accepted by the compiler, and whether it ends the options.
pub(crate) enum RegexpOption {
    Nocase,
    Last,
}

pub(crate) fn regexp_option(option: &[u8]) -> Option<RegexpOption> {
    if option == b"--" {
        Some(RegexpOption::Last)
    } else if option.len() > 1 && b"-nocase".starts_with(option) {
        Some(RegexpOption::Nocase)
    } else {
        None
    }
}

/// Select the compiler's outcome on the parser's projected words.
#[must_use]
pub fn select_original(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    operation: SemanticOperationId,
) -> NativeCompilationSelection {
    match recipe(words, operand_from, version) {
        Ok(_) => NativeCompilationSelection::Inline {
            operation,
            guard: NativeCompilationGuard::BeforeArguments,
        },
        Err(selection) => selection,
    }
}

/// Project an admitted inline selection onto its stack recipe.
#[must_use]
pub fn instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Option<NativeRegexpInstruction> {
    recipe(words, operand_from, version).ok()
}

fn recipe(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeRegexpInstruction, NativeCompilationSelection> {
    use NativeCompilationSelection as Selection;
    if !modelled(version) {
        return Err(Selection::Unknown);
    }
    let projected =
        project_native_compiler_words(words, version).map_err(|_| Selection::Unknown)?;
    let arguments = projected.get(operand_from..).ok_or(Selection::Unknown)?;
    if arguments
        .iter()
        .any(|word| word.shape == NativeCompilationWordShape::Opaque)
    {
        return Err(Selection::Unknown);
    }
    if arguments.len() < 2
        || arguments
            .iter()
            .any(|word| word.shape == NativeCompilationWordShape::Expanded)
    {
        return Err(Selection::Generic);
    }
    let mut nocase = false;
    let mut saw_last = false;
    let mut index = 0;
    while index < arguments.len() - 2 {
        let option = projected_simple(&arguments[index]).ok_or(Selection::Generic)?;
        index += 1;
        match regexp_option(option).ok_or(Selection::Generic)? {
            RegexpOption::Last => {
                saw_last = true;
                break;
            }
            RegexpOption::Nocase => nocase = true,
        }
    }
    let [pattern, subject] = &arguments[index..] else {
        return Err(Selection::Generic);
    };
    let glob = match projected_simple(pattern) {
        Some(bytes) if bytes.first() == Some(&b'-') && !saw_last => {
            return Err(Selection::Generic);
        }
        Some(bytes) => tcl_syntax::glob::re_to_glob(bytes),
        None => None,
    };
    let matcher = match glob {
        Some((glob, true)) if !nocase => NativeRegexpMatcher::Equal(glob),
        Some((glob, _)) => NativeRegexpMatcher::Glob {
            pattern: glob,
            nocase,
        },
        None => NativeRegexpMatcher::Regexp {
            pattern: pattern.operand.clone(),
            nocase,
        },
    };
    Ok(NativeRegexpInstruction {
        matcher,
        subject: subject.operand.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    fn select(
        source: &[u8],
        version: TclVersion,
    ) -> Result<NativeRegexpInstruction, NativeCompilationSelection> {
        let profile = match version {
            TclVersion::V9_1 => "tcl9.1",
            TclVersion::V8_6 => "tcl8.6",
            _ => "tcl9.0",
        };
        let parsed = native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            LexerConfig::from_grammar(tcl_dialect::DialectProfile::find(profile).unwrap().grammar),
        )
        .unwrap();
        let words = NativeCompilerWords::capture(
            &parsed.commands[0].words,
            NativeStringProtocol::C(version),
        )
        .unwrap();
        let selected = select_original(&words, 1, version, SemanticOperationId::Invoke);
        let recipe = recipe(&words, 1, version);
        assert_eq!(
            matches!(selected, NativeCompilationSelection::Inline { .. }),
            recipe.is_ok(),
            "{}",
            String::from_utf8_lossy(source)
        );
        recipe
    }

    fn word(index: usize) -> NativeCompilerWordOperand {
        NativeCompilerWordOperand::Original(index)
    }

    fn glob(pattern: &[u8], nocase: bool) -> NativeRegexpMatcher {
        NativeRegexpMatcher::Glob {
            pattern: pattern.to_vec(),
            nocase,
        }
    }

    fn regexp(pattern: usize, nocase: bool) -> NativeRegexpMatcher {
        NativeRegexpMatcher::Regexp {
            pattern: word(pattern),
            nocase,
        }
    }

    // Each expectation is the tclsh 9.0.4 `disassemble lambda` listing.
    fn release_cases() -> [(&'static [u8], NativeRegexpMatcher, usize); 25] {
        use NativeRegexpMatcher::Equal;
        [
            // loadScalar re; loadScalar s; regexp +3
            (b"regexp -- $re $s", regexp(2, false), 3),
            (b"regexp $re $s", regexp(1, false), 2),
            // regexp +11
            (b"regexp -nocase $re $s", regexp(2, true), 3),
            (b"regexp -no -- $re $s", regexp(3, true), 4),
            // push "*abc*"; strmatch +0
            (b"regexp abc $s", glob(b"*abc*", false), 2),
            (b"regexp \"abc\" $s", glob(b"*abc*", false), 2),
            // push "^abc$" (two TEXT tokens); regexp +3
            (b"regexp ^abc$ $s", regexp(1, false), 2),
            (b"regexp \"a$\" $s", regexp(1, false), 2),
            // push "*"; strmatch +0: a lone `$` is one TEXT token.
            (b"regexp $ $s", glob(b"*", false), 2),
            (b"regexp {a$} $s", glob(b"*a", false), 2),
            // push "ax" (BS token); regexp +3
            (b"regexp a\\x $s", regexp(1, false), 2),
            // push "abc"; streq / strmatch +1
            (b"regexp {^abc$} $s", Equal(b"abc".to_vec()), 2),
            (b"regexp -nocase {^abc$} $s", glob(b"abc", true), 3),
            (b"regexp {^a.*b$} $s", glob(b"a*b", false), 2),
            (b"regexp {a+} $s", regexp(1, false), 2),
            (b"regexp {***=a*b} $s", glob(b"*a\\*b*", false), 2),
            (b"regexp {} $s", glob(b"*", false), 2),
            (b"regexp \"\" $s", glob(b"*", false), 2),
            (b"regexp -nocase {} $s", glob(b"*", true), 3),
            (b"regexp -- -abc $s", glob(b"*-abc*", false), 3),
            (b"regexp -- -- $s", glob(b"*--*", false), 3),
            (b"regexp -n abc $s", glob(b"*abc*", true), 3),
            (b"regexp {a\\.b} $s", glob(b"*a.b*", false), 2),
            (b"regexp {a.*b.*c} $s", regexp(1, false), 2),
            (b"regexp {a.+} $s", glob(b"*a?*", false), 2),
        ]
    }

    #[test]
    fn c9_regexp_compiler_matches_the_release_disassembly() {
        let cases = release_cases();
        for version in [TclVersion::V9_0, TclVersion::V9_1] {
            for (source, matcher, subject) in &cases {
                let recipe = select(source, version)
                    .unwrap_or_else(|selection| panic!("{source:?}: {selection:?}"));
                assert_eq!(
                    &recipe.matcher,
                    matcher,
                    "{}",
                    String::from_utf8_lossy(source)
                );
                assert_eq!(recipe.subject, word(*subject));
            }
        }
    }

    // tclsh 9.0.4 invokes the command (`invokeStk1`) for each of these.
    #[test]
    fn c9_regexp_compiler_declines_where_the_release_invokes_the_command() {
        for source in [
            b"regexp -abc $s".as_slice(),
            b"regexp -nocase -abc $s",
            b"regexp -all abc $s",
            b"regexp abc $s m",
            b"regexp $opt abc $s",
            b"regexp -- $s",
            b"regexp -nocase -- $s",
            b"regexp $s",
            b"regexp {*}$args",
        ] {
            assert_eq!(
                select(source, TclVersion::V9_0).unwrap_err(),
                NativeCompilationSelection::Generic,
                "{}",
                String::from_utf8_lossy(source)
            );
        }
    }

    #[test]
    fn parser_expanded_options_are_simple_words() {
        // push "*a*"; strmatch +1
        let recipe = select(b"regexp {*}{-nocase -- a} $s", TclVersion::V9_0).unwrap();
        assert_eq!(
            recipe.matcher,
            NativeRegexpMatcher::Glob {
                pattern: b"*a*".to_vec(),
                nocase: true
            }
        );
        assert_eq!(recipe.subject, word(2));
    }

    #[test]
    fn unmodelled_releases_stay_unselected() {
        assert_eq!(
            select(b"regexp -- $re $s", TclVersion::V8_6).unwrap_err(),
            NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn regexp_flags_follow_the_release_immediate() {
        let plain = NativeRegexpMatcher::Regexp {
            pattern: word(1),
            nocase: false,
        };
        let nocase = NativeRegexpMatcher::Regexp {
            pattern: word(2),
            nocase: true,
        };
        assert_eq!(plain.regexp_flags(), Some(3));
        assert_eq!(nocase.regexp_flags(), Some(11));
        assert_eq!(NativeRegexpMatcher::Equal(Vec::new()).regexp_flags(), None);
    }
}
