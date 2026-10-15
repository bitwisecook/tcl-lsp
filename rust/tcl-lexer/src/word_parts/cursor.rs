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

//! Incomplete variable source prefixes retain lexical ownership without values.

use super::{
    ArrayIndexEnd, BracedVarEnd, LexerConfig, SubstFlags, TemplateVariableSyntax, WordPart,
    braced_var_name_end, scan_array_index, scan_var_ref, shallow_spanned_parts,
};
use crate::{ParseIssueKind, SourceImage, Span, TokenType};

/// Source cursor geometry that cannot be represented by this variable purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableCursorUnavailable {
    /// The source region or cursor does not belong to the supplied image.
    SourceGeometry,
    /// The configured lexical stream is unavailable.
    LexicalStream,
    /// An index contains a command, expression or unavailable value prefix.
    IndexSubstitution,
    /// Nested variable cursor syntax exceeds the shared decomposition budget.
    IndexNesting,
}

/// A variable prefix in an actual configured lexical source region. The
/// enclosing source phase and any name/frame/edit authority remain independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCursorPrefix {
    image: SourceImage,
    config: LexerConfig,
    reference: Span,
    container: Span,
    partial: Span,
    root: Span,
    index: Option<Span>,
    braced: bool,
}

impl VariableCursorPrefix {
    /// Same complete source bytes/channel and all configured grammar axes.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        &self.image == image && self.config == config
    }
    /// Original `$` through the cursor, without invented closing syntax.
    #[must_use]
    pub const fn reference_span(&self) -> Span {
        self.reference
    }
    /// Genuine complete reference extent, or the lexer-owned unfinished extent.
    #[must_use]
    pub const fn container_span(&self) -> Span {
        self.container
    }
    /// Original prefix after `$` or `${`, including a separate index if present.
    #[must_use]
    pub const fn partial_span(&self) -> Span {
        self.partial
    }
    /// Original braced name prefix, bare root prefix or complete explicit index root.
    #[must_use]
    pub const fn root_span(&self) -> Span {
        self.root
    }
    /// Original index prefix, excluding its opening parenthesis.
    #[must_use]
    pub const fn index_span(&self) -> Option<Span> {
        self.index
    }
    /// Braced name syntax. Combined-name interpretation needs its independent owner.
    #[must_use]
    pub const fn is_braced(&self) -> bool {
        self.braced
    }
}

fn span(start: usize, end: usize) -> Result<Span, VariableCursorUnavailable> {
    Ok(Span::new(
        u32::try_from(start).map_err(|_| VariableCursorUnavailable::SourceGeometry)?,
        u32::try_from(end).map_err(|_| VariableCursorUnavailable::SourceGeometry)?,
    ))
}

fn terminal_dollar(bytes: &[u8], start: usize, config: LexerConfig) -> Option<usize> {
    let mut at = start;
    while at < bytes.len() {
        if bytes[at] == b'\\' && at + 1 < bytes.len() {
            at = crate::substitution::backslash_escape_end_bytes_in(bytes, at, config.escapes);
        } else if at + 1 == bytes.len() && bytes[at] == b'$' {
            return Some(at);
        } else {
            at += 1;
        }
    }
    None
}

fn prefix_at(
    image: &SourceImage,
    region: Span,
    config: LexerConfig,
    mut at: usize,
    cursor: usize,
    enclosing_end: usize,
) -> Result<Option<VariableCursorPrefix>, VariableCursorUnavailable> {
    let source = image
        .bytes()
        .get(..region.end() as usize)
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    let prefix = source
        .get(..cursor)
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    for _ in 0..=super::MAX_INDEX_DEPTH {
        if prefix.get(at) != Some(&b'$') {
            return Ok(None);
        }
        let braced = prefix.get(at + 1) == Some(&b'{');
        let name = at + if braced { 2 } else { 1 };
        let mut index = None;
        let root_end;
        if braced {
            if matches!(
                braced_var_name_end(prefix, name, config.braced_var),
                BracedVarEnd::Closed(_)
            ) {
                return Ok(None);
            }
            root_end = cursor;
        } else {
            let end = tcl_core_types::naming::scan_var_name_end_with(
                prefix,
                name,
                config.var_syntax.name_allows_high_bytes(),
            );
            if end == cursor {
                root_end = end;
            } else if prefix.get(end) == Some(&b'(')
                && !(end == name && config.var_syntax.has_expr_sugar())
            {
                root_end = end;
                let scan = if config.var_syntax.index_parens_nest() {
                    let end = crate::ranges::jim_parenthesis_body_end(prefix, end + 1);
                    if prefix.get(end) == Some(&b')') || end < cursor {
                        return Ok(None);
                    }
                    None
                } else {
                    Some(scan_array_index(
                        prefix,
                        end,
                        config.array_index,
                        config.braced_var,
                    ))
                };
                if scan.is_some_and(|scan| scan.invalid.is_some()) {
                    return Err(VariableCursorUnavailable::IndexSubstitution);
                }
                if scan.is_some_and(|scan| matches!(scan.end, ArrayIndexEnd::Closed(_))) {
                    return Ok(None);
                }
                let start = end + 1;
                let parts = shallow_spanned_parts(
                    &prefix[start..],
                    SubstFlags::default(),
                    config,
                    TemplateVariableSyntax::WrittenWord,
                );
                if let Some(last) = parts.last() {
                    let nested = match &last.part {
                        WordPart::Variable(_) if last.end == cursor - start => {
                            Some(start + last.start)
                        }
                        WordPart::ParseError(_)
                            if matches!(
                                last.error_kind,
                                Some(
                                    ParseIssueKind::MissingVariableBrace
                                        | ParseIssueKind::MissingParenthesis
                                )
                            ) && prefix.get(start + last.start) == Some(&b'$') =>
                        {
                            Some(start + last.start)
                        }
                        WordPart::Text(_) => terminal_dollar(&prefix[start..], last.start, config)
                            .map(|at| start + at),
                        _ => None,
                    };
                    if let Some(nested) = nested {
                        at = nested;
                        continue;
                    }
                }
                if parts
                    .iter()
                    .any(|part| !matches!(part.part, WordPart::Text(_)))
                {
                    return Err(VariableCursorUnavailable::IndexSubstitution);
                }
                index = Some(span(start, cursor)?);
            } else {
                return Ok(None);
            }
        }
        let container_end = match scan_var_ref(source, at, config) {
            Ok(Some(reference)) => reference.next,
            Ok(None) => at + 1,
            Err(_) => enclosing_end,
        };
        if container_end < cursor || container_end > region.end() as usize {
            return Err(VariableCursorUnavailable::SourceGeometry);
        }
        return Ok(Some(VariableCursorPrefix {
            image: image.clone(),
            config,
            reference: span(at, cursor)?,
            container: span(at, container_end)?,
            partial: span(name, cursor)?,
            root: span(name, root_end)?,
            index,
            braced,
        }));
    }
    Err(VariableCursorUnavailable::IndexNesting)
}

/// Select an incomplete or complete variable prefix from the actual configured
/// lexical stream of an independently owned source script region. Comments,
/// literal braces and escaped sigils do not create variable cursor prefixes.
/// No completed `NativeWord`, name key, evaluated value or frame is issued.
///
/// # Errors
/// Returns unavailable source geometry, lexical ownership or index-prefix syntax.
pub fn variable_cursor_prefix_in(
    image: &SourceImage,
    region: Span,
    cursor: u32,
    config: LexerConfig,
) -> Result<Option<VariableCursorPrefix>, VariableCursorUnavailable> {
    if region.start() > cursor || cursor > region.end() || region.end() as usize > image.len() {
        return Err(VariableCursorUnavailable::SourceGeometry);
    }
    let tokens = crate::source_region_tokens_in(image, region, config)
        .map_err(|_| VariableCursorUnavailable::LexicalStream)?;
    let candidates = tokens
        .iter()
        .filter(|token| {
            token.span.start() < cursor
                && cursor <= token.span.end()
                && (token.kind == TokenType::Var
                    || (matches!(token.kind, TokenType::Str | TokenType::Esc)
                        && token.span.len() == 1
                        && image.bytes().get(token.span.as_range()) == Some(b"$")))
        })
        .collect::<Vec<_>>();
    let [token] = candidates.as_slice() else {
        return Ok(None);
    };
    prefix_at(
        image,
        region,
        config,
        token.span.start() as usize,
        cursor as usize,
        token.span.end() as usize,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(marked: &str, config: LexerConfig) -> (SourceImage, Option<VariableCursorPrefix>) {
        let cursor = marked.find("<|>").unwrap();
        let source = marked.replacen("<|>", "", 1);
        let image = SourceImage::document(&source);
        let result = variable_cursor_prefix_in(
            &image,
            Span::new(0, image.len() as u32),
            cursor as u32,
            config,
        )
        .unwrap();
        (image, result)
    }

    #[test]
    fn original_variable_cursor_keeps_braced_names_and_complete_container() {
        let config = LexerConfig::default();
        let (image, prefix) = at("puts ${cash$name<|>}suffix", config);
        let prefix = prefix.unwrap();
        assert!(prefix.is_braced());
        assert_eq!(
            &image.bytes()[prefix.partial_span().as_range()],
            b"cash$name"
        );
        assert_eq!(
            &image.bytes()[prefix.container_span().as_range()],
            b"${cash$name}"
        );
        assert!(prefix.index_span().is_none());
        let (image, prefix) = at("puts ${a b<|>}suffix", config);
        let prefix = prefix.unwrap();
        assert_eq!(&image.bytes()[prefix.partial_span().as_range()], b"a b");
        let (image, prefix) = at("puts ${scalar(open<|>}", config);
        let prefix = prefix.unwrap();
        assert_eq!(
            &image.bytes()[prefix.root_span().as_range()],
            b"scalar(open"
        );
        assert!(prefix.index_span().is_none());
        assert!(prefix.matches_source(&image, config));
        assert!(!prefix.matches_source(&SourceImage::native(image.shared_bytes()), config));
        let changed = LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        assert!(!prefix.matches_source(&image, changed));
    }

    #[test]
    fn original_variable_cursor_reuses_configured_name_and_closer_grammar() {
        let eight = LexerConfig {
            braced_var: crate::BracedVarStyle::FirstClose,
            ..LexerConfig::default()
        };
        let nine = LexerConfig {
            braced_var: crate::BracedVarStyle::Tcl9Nesting,
            ..eight
        };
        for (config, expected) in [
            (eight, b"${a{b}".as_slice()),
            (nine, b"${a{b}c}".as_slice()),
        ] {
            let (image, prefix) = at("puts ${a<|>{b}c}", config);
            assert_eq!(
                &image.bytes()[prefix.unwrap().container_span().as_range()],
                expected
            );
        }
        assert!(at("puts $a:<|>rest", eight).1.is_none());
        assert!(at("puts $caf\u{e9}<|>", eight).1.is_none());
        let jim = LexerConfig {
            var_syntax: tcl_dialect::VarSyntax::Jim,
            ..eight
        };
        let (image, prefix) = at("puts $caf\u{e9}<|>", jim);
        assert_eq!(
            &image.bytes()[prefix.unwrap().partial_span().as_range()],
            "café".as_bytes()
        );
    }

    #[test]
    fn original_variable_cursor_preserves_index_and_nested_reference_geometry() {
        let config = LexerConfig::default();
        let (image, prefix) = at("puts $arr(ke<|>y)", config);
        let prefix = prefix.unwrap();
        assert_eq!(&image.bytes()[prefix.root_span().as_range()], b"arr");
        assert_eq!(
            &image.bytes()[prefix.index_span().unwrap().as_range()],
            b"ke"
        );
        assert_eq!(
            &image.bytes()[prefix.container_span().as_range()],
            b"$arr(key)"
        );
        for marked in ["puts $arr($in<|>ner)", "puts $arr($<|>)"] {
            let (image, prefix) = at(marked, config);
            let prefix = prefix.unwrap();
            assert_eq!(
                image.bytes()[prefix.reference_span().start() as usize],
                b'$'
            );
            assert_eq!(
                &image.bytes()[prefix.container_span().as_range()],
                if marked.contains("$in") {
                    b"$inner".as_slice()
                } else {
                    b"$".as_slice()
                }
            );
            assert!(prefix.index_span().is_none());
        }
        let image = SourceImage::document("puts $arr([set x 1]suffix)");
        assert_eq!(
            variable_cursor_prefix_in(
                &image,
                Span::new(0, image.len() as u32),
                (image.len() - 1) as u32,
                config
            ),
            Err(VariableCursorUnavailable::IndexSubstitution)
        );
    }

    #[test]
    fn original_variable_cursor_keeps_inert_and_unavailable_source_distinct() {
        let config = LexerConfig::default();
        for marked in [
            "# $no<|>",
            "puts {$no<|>}",
            "puts \\$no<|>",
            "puts ${closed}<|>",
        ] {
            assert!(at(marked, config).1.is_none(), "{marked}");
        }
        let image = SourceImage::document("puts $name");
        assert_eq!(
            variable_cursor_prefix_in(&image, Span::new(0, 5), 8, config),
            Err(VariableCursorUnavailable::SourceGeometry)
        );
    }
}
