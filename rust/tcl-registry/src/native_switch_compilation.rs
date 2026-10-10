// SPDX-License-Identifier: AGPL-3.0-or-later
//! `TclCompileSwitchCmd` selection and original arm compilation order.

use crate::native_compilation::NativeCompilationWordShape;
use crate::native_compiler_word_projection::{
    NativeCompilerWordOperand, NativeProjectedCompilerWord, project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_lexer::Span;

/// Matching mode selected by the native switch compiler's literal options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeSwitchMode {
    /// Compare exact string keys through a jump table.
    Exact,
    /// Match each pattern using native glob semantics.
    Glob,
    /// Match regular expressions, including native literal/glob reductions.
    Regexp,
    /// Convert the subject to a wide integer and dispatch by numeric key.
    Integer,
}
/// Native matching instruction selected for one original pattern.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeSwitchMatch {
    /// Dispatch using the counted exact-key table.
    ExactTable,
    /// Dispatch using the wide-integer key table.
    IntegerTable,
    /// Match the retained native glob pattern bytes.
    Glob(Vec<u8>),
    /// Compare the retained exact bytes after a regexp reduction.
    Equal(Vec<u8>),
    /// Execute the retained regular expression without a glob reduction.
    Regexp(Vec<u8>),
    /// An empty regular expression matches every subject.
    Always,
}
/// Original switch arm and its source-backed body compilation decision.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeSwitchArm {
    /// Original literal pattern bytes before instruction-specific reduction.
    pub pattern: Vec<u8>,
    /// Pattern body extent in the original source image.
    pub pattern_span: Span,
    /// Parsed numeric key when integer-table compilation was selected.
    pub integer: Option<i64>,
    /// Selected native matcher for this pattern.
    pub matcher: NativeSwitchMatch,
    /// Original script body extent; continuation arms have no body.
    pub body: Option<Span>,
    /// Native jump-table duplicate suppression can omit compilation entirely.
    pub compile_body: bool,
    /// Original real body selected by a matching continuation arm.
    pub target: usize,
}
/// Portable switch instruction recipe retaining original operands and arm order.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeSwitchInstruction {
    /// Native compiler release whose option and matching rules were selected.
    pub version: TclVersion,
    /// Original subject operand, including an authenticated literal expansion.
    pub subject: NativeCompilerWordOperand,
    /// Selected matching mode.
    pub mode: NativeSwitchMode,
    /// Whether the selected compiler performs case-insensitive matching.
    pub nocase: bool,
    /// Arms in original source order, including continuations and masked bodies.
    pub arms: Vec<NativeSwitchArm>,
    /// Whether the last arm is the native terminal default branch.
    pub terminal_default: bool,
}
/// Compiler decline or unavailable original-source evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeSwitchUnavailable {
    /// The native compiler declines this command and leaves generic invocation.
    Generic,
    /// Original word indices or source extents are inconsistent.
    Geometry,
    /// Original compiler-word projection could not be established.
    Projection,
}

fn simple(word: &NativeProjectedCompilerWord) -> bool {
    matches!(
        word.shape,
        NativeCompilationWordShape::Literal
            | NativeCompilationWordShape::QuotedLiteral
            | NativeCompilationWordShape::BracedLiteral
    )
}
fn span(
    words: &NativeCompilerWords<'_>,
    operand: &NativeCompilerWordOperand,
) -> Result<Span, NativeSwitchUnavailable> {
    match operand {
        NativeCompilerWordOperand::Original(index) => words
            .original_words()
            .get(*index)
            .ok_or(NativeSwitchUnavailable::Geometry)?
            .content_span()
            .map_err(|_| NativeSwitchUnavailable::Geometry),
        NativeCompilerWordOperand::LiteralExpansion { value_span, .. } => Ok(*value_span),
    }
}
fn prefix(bytes: &[u8], name: &[u8], minimum: usize) -> bool {
    bytes.len() >= minimum && name.starts_with(bytes)
}

/// Native `TclReToGlob` projection on counted pattern bytes. Failure retains REGEXP.
fn regexp_glob(pattern: &[u8]) -> Option<(Vec<u8>, bool)> {
    if let Some(literal) = pattern.strip_prefix(b"***=") {
        let mut out = vec![b'*'];
        for &byte in literal {
            if b"\\*[]?".contains(&byte) {
                out.push(b'\\');
            }
            out.push(byte);
        }
        out.push(b'*');
        return Some((out, false));
    }
    let mut out = Vec::new();
    let mut at = usize::from(pattern.first() == Some(&b'^'));
    let mut left = at != 0;
    let mut right = false;
    let mut last_star = !left;
    let mut stars = 0;
    if !left {
        out.push(b'*');
    }
    while at < pattern.len() {
        let byte = pattern[at];
        match byte {
            b'\\' => {
                at += 1;
                let escaped = *pattern.get(at)?;
                match escaped {
                    b'a' => out.push(7),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    b'n' => out.push(10),
                    b'r' => out.push(13),
                    b't' => out.push(9),
                    b'v' => out.push(11),
                    b'B' | b'\\' => {
                        out.extend_from_slice(b"\\\\");
                        left = false;
                    }
                    b'*' | b'[' | b']' | b'?' => {
                        out.push(b'\\');
                        out.push(escaped);
                        left = false;
                    }
                    b'{' | b'}' | b'(' | b')' | b'+' | b'.' | b'|' | b'^' | b'$' => {
                        out.push(escaped);
                    }
                    _ => return None,
                }
            }
            b'.' => {
                left = false;
                match pattern.get(at + 1) {
                    Some(b'*') => {
                        at += 2;
                        if !last_star {
                            out.push(b'*');
                            stars += 1;
                            last_star = true;
                        }
                        continue;
                    }
                    Some(b'+') => {
                        at += 2;
                        out.extend_from_slice(b"?*");
                        stars += 1;
                        last_star = true;
                        continue;
                    }
                    _ => out.push(b'?'),
                }
            }
            b'$' => {
                if at + 1 != pattern.len() {
                    return None;
                }
                right = true;
            }
            b'*' | b'+' | b'?' | b'|' | b'^' | b'{' | b'}' | b'(' | b')' | b'[' | b']' => {
                return None;
            }
            _ => out.push(byte),
        }
        last_star = false;
        at += 1;
    }
    if stars > 1 {
        return None;
    }
    if !right && !last_star {
        out.push(b'*');
    }
    Some((out, left && right))
}

struct SwitchOptions {
    value: usize,
    mode: NativeSwitchMode,
    nocase: bool,
}

fn switch_options(
    args: &[NativeProjectedCompilerWord],
    version: TclVersion,
) -> Result<SwitchOptions, NativeSwitchUnavailable> {
    use NativeSwitchUnavailable as Error;
    let mut value = 0;
    let mut mode = NativeSwitchMode::Exact;
    let mut found_mode = false;
    let mut nocase = false;
    if args.len() != 2 {
        let mut terminator = false;
        while args.len() - value >= 3 {
            let option = &args[value];
            if !simple(option) {
                return Err(Error::Generic);
            }
            let bytes = option.literal.as_deref().ok_or(Error::Geometry)?;
            let selected = if prefix(bytes, b"-exact", 2) {
                Some(NativeSwitchMode::Exact)
            } else if prefix(bytes, b"-glob", 2) {
                Some(NativeSwitchMode::Glob)
            } else if prefix(bytes, b"-regexp", 2) {
                Some(NativeSwitchMode::Regexp)
            } else if version >= TclVersion::V9_1 && prefix(bytes, b"-integer", 4) {
                Some(NativeSwitchMode::Integer)
            } else if prefix(bytes, b"-nocase", 2) {
                nocase = true;
                None
            } else if bytes == b"--" {
                value += 1;
                terminator = true;
                break;
            } else {
                return Err(Error::Generic);
            };
            if let Some(selected) = selected {
                if found_mode {
                    return Err(Error::Generic);
                }
                found_mode = true;
                mode = selected;
            }
            value += 1;
        }
        if !terminator || args.len() - value < 2 {
            return Err(Error::Generic);
        }
        if version < TclVersion::V9_1 && nocase && mode == NativeSwitchMode::Exact {
            return Err(Error::Generic);
        }
    }
    Ok(SwitchOptions {
        value,
        mode,
        nocase,
    })
}

type SwitchPair = (Vec<u8>, Span);

fn switch_pairs(
    words: &NativeCompilerWords<'_>,
    remaining: &[NativeProjectedCompilerWord],
) -> Result<Vec<SwitchPair>, NativeSwitchUnavailable> {
    use NativeSwitchUnavailable as Error;
    let mut pairs = Vec::new();
    if remaining.len() == 1 {
        let word = &remaining[0];
        if !simple(word) {
            return Err(Error::Generic);
        }
        let content = span(words, &word.operand)?;
        let image = words.original_words()[0].image();
        let bytes = image
            .bytes()
            .get(content.as_range())
            .ok_or(Error::Geometry)?;
        let mut offset = 0;
        while let Some(element) =
            tcl_syntax::list::find_element_bytes(bytes, offset).map_err(|_| Error::Generic)?
        {
            if !element.literal {
                return Err(Error::Generic);
            }
            let start = content
                .start()
                .checked_add(u32::try_from(element.value.start).map_err(|_| Error::Geometry)?)
                .ok_or(Error::Geometry)?;
            let end = content
                .start()
                .checked_add(u32::try_from(element.value.end).map_err(|_| Error::Geometry)?)
                .ok_or(Error::Geometry)?;
            let value = tcl_syntax::backslash::native_source_literal_bytes(
                &bytes[element.value.clone()],
                image.channel(),
                words.source_protocol(),
            )
            .map_err(|_| Error::Geometry)?
            .into_owned();
            pairs.push((value, Span::new(start, end)));
            offset = element.next;
        }
    } else {
        for word in remaining {
            if !simple(word) {
                return Err(Error::Generic);
            }
            pairs.push((
                word.literal.clone().ok_or(Error::Geometry)?,
                span(words, &word.operand)?,
            ));
        }
    }
    Ok(pairs)
}

fn switch_arms(
    pairs: &[SwitchPair],
    mode: NativeSwitchMode,
    nocase: bool,
    version: TclVersion,
    terminal_default: bool,
) -> Result<Vec<NativeSwitchArm>, NativeSwitchUnavailable> {
    let mut arms = Vec::new();
    let mut keys = std::collections::HashSet::new();
    let mut integer_keys = std::collections::HashSet::new();
    let mut must_generate = true;
    for (index, pair) in pairs.as_chunks::<2>().0.iter().enumerate() {
        let default = terminal_default && index + 1 == pairs.len() / 2;
        let (pattern, pattern_span) = &pair[0];
        let body = (pair[1].0 != b"-").then_some(pair[1].1);
        let integer = switch_integer(pattern, mode, default, version)?;
        let matcher = if default {
            NativeSwitchMatch::Always
        } else {
            match mode {
                NativeSwitchMode::Exact => NativeSwitchMatch::ExactTable,
                NativeSwitchMode::Integer => NativeSwitchMatch::IntegerTable,
                NativeSwitchMode::Glob => NativeSwitchMatch::Glob(pattern.clone()),
                NativeSwitchMode::Regexp if pattern.is_empty() => NativeSwitchMatch::Always,
                NativeSwitchMode::Regexp => match regexp_glob(pattern) {
                    Some((glob, true)) if !nocase => NativeSwitchMatch::Equal(glob),
                    Some((glob, _)) => NativeSwitchMatch::Glob(glob),
                    None => NativeSwitchMatch::Regexp(pattern.clone()),
                },
            }
        };
        let new = if default {
            true
        } else if mode == NativeSwitchMode::Exact {
            let key = if nocase {
                tcl_syntax::native_glob::lower_c_string_bytes(version, pattern)
            } else {
                tcl_core_types::c_string_extent(pattern).to_vec()
            };
            keys.insert(key)
        } else if mode == NativeSwitchMode::Integer {
            integer_keys.insert(integer.unwrap())
        } else {
            true
        };
        let compile_body = body.is_some() && (new || must_generate);
        if body.is_none() {
            must_generate = true;
        } else if compile_body {
            must_generate = false;
        }
        arms.push(NativeSwitchArm {
            pattern: pattern.clone(),
            pattern_span: *pattern_span,
            integer,
            matcher,
            body,
            compile_body,
            target: index,
        });
    }
    for index in (0..arms.len()).rev() {
        if arms[index].body.is_none() {
            arms[index].target = arms[index + 1].target;
        }
    }
    Ok(arms)
}

fn switch_integer(
    pattern: &[u8],
    mode: NativeSwitchMode,
    default: bool,
    version: TclVersion,
) -> Result<Option<i64>, NativeSwitchUnavailable> {
    use NativeSwitchUnavailable as Error;
    let integer = if mode == NativeSwitchMode::Integer && !default {
        if pattern == b"default" {
            return Err(Error::Generic);
        }
        let conversion =
            tcl_syntax::scalar_getter::NativeScalarGetterProtocol::for_tcl_version(version)
                .fresh_conversion(
                    tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
                    pattern,
                )
                .ok_or(Error::Generic)?;
        match conversion.outcome() {
            Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(number)) => Some(number),
            _ => return Err(Error::Generic),
        }
    } else {
        None
    };
    Ok(integer)
}

/// Select all options/arms before emitting the subject's first instruction.
/// This retains original source spans and authentic Generic declines.
pub fn native_switch_instruction(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
) -> Result<NativeSwitchInstruction, NativeSwitchUnavailable> {
    use NativeSwitchUnavailable as Error;
    if version < TclVersion::V8_5 {
        return Err(Error::Generic);
    }
    let projected = project_native_compiler_words(words, version).map_err(|_| Error::Projection)?;
    let args = projected.get(operand_from..).ok_or(Error::Geometry)?;
    if args.len() < 2 {
        return Err(Error::Generic);
    }
    let options = switch_options(args, version)?;
    let SwitchOptions {
        value,
        mode,
        nocase,
    } = options;
    let subject = args[value].operand.clone();
    let remaining = &args[value + 1..];
    let pairs = switch_pairs(words, remaining)?;
    if pairs.is_empty() || !pairs.len().is_multiple_of(2) || pairs.last().unwrap().0 == b"-" {
        return Err(Error::Generic);
    }
    let terminal_default = pairs[pairs.len() - 2].0 == b"default";
    let arms = switch_arms(&pairs, mode, nocase, version, terminal_default)?;
    Ok(NativeSwitchInstruction {
        version,
        subject,
        mode,
        nocase,
        arms,
        terminal_default,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn switch_selection_matches_56_native_compiler_frontiers() {
        let mut compared = 0;
        for row in include_str!("../tests/data/registered-switch56.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let version = match fields[0] {
                "8.5.19" => TclVersion::V8_5,
                "8.6.18" => TclVersion::V8_6,
                "9.0.4" => TclVersion::V9_0,
                "9.1.0" => TclVersion::V9_1,
                _ => panic!("native version"),
            };
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            let image = tcl_lexer::SourceImage::native(unhex(fields[2]));
            let commands = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = NativeCompilerWords::capture(
                &commands.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            let selected = native_switch_instruction(&words, 1, version);
            assert_eq!(
                selected.is_ok(),
                fields[5] == "inline",
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                selected
            );
            if fields[1] == "7" {
                let recipe = selected.as_ref().unwrap();
                assert_eq!(recipe.arms[0].target, 1);
                assert!(recipe.arms[0].body.is_none());
            }
            if fields[1] == "8" {
                let recipe = selected.as_ref().unwrap();
                assert!(recipe.arms[0].compile_body);
                assert!(!recipe.arms[1].compile_body);
            }
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
    #[test]
    fn switch_branch_selection_matches_56_native_compiler_frontiers() {
        let mut compared = 0;
        for row in include_str!("../tests/data/registered-switch-branches56.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let version = match fields[0] {
                "8.5.19" => TclVersion::V8_5,
                "8.6.18" => TclVersion::V8_6,
                "9.0.4" => TclVersion::V9_0,
                "9.1.0" => TclVersion::V9_1,
                _ => panic!("native version"),
            };
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            let image = tcl_lexer::SourceImage::native(unhex(fields[2]));
            let commands = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = NativeCompilerWords::capture(
                &commands.commands[0].words,
                tcl_syntax::native_string::NativeStringProtocol::C(version),
            )
            .unwrap();
            let selected = native_switch_instruction(&words, 1, version);
            assert_eq!(
                selected.is_ok(),
                fields[6] == "inline",
                "{}/{}: {:?}",
                fields[0],
                fields[1],
                selected
            );
            if fields[1] == "6" {
                assert!(matches!(
                    selected.as_ref().unwrap().arms[0].matcher,
                    NativeSwitchMatch::Regexp(_)
                ));
            }
            if fields[1] == "8" {
                assert!(matches!(
                    selected.as_ref().unwrap().arms[0].matcher,
                    NativeSwitchMatch::Always
                ));
            }
            compared += 1;
        }
        assert_eq!(compared, 56);
    }
}
