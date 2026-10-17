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

//! Pure SpecTcl source rows shared by the loader and source navigation.
//!
//! This module lives below both consumers because SpecTcl depends on Core's
//! VFS. It owns no loader, filesystem access, registration or execution facts.

mod original_word;
pub use original_word::{
    LocalPackSourceWord, OriginalPackSourceStatement, OriginalPackSourceWord,
    original_pack_source_statements,
};

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_lexer::{
    LeadingBom, LexerConfig, SourceImage, SourceMap, SourceSegmentationPurpose, TokenType,
};

use super::syntax::{build::build_document, segment::segments_from_document};
use crate::segmenter::SegmentedCommand;

/// The newest major vocabulary interpreted by the SpecTcl loader.
pub const NEWEST_SUPPORTED_MAJOR: u32 = 2;

/// Fixed SpecTcl file grammar, with an explicit whole-file or nested purpose.
#[must_use]
pub fn pack_source_lexer_config(purpose: SourceSegmentationPurpose) -> LexerConfig {
    purpose.config(LexerConfig {
        leading_bom: LeadingBom::Skip,
        // dialect-drift-ok: fixed SpecTcl DSL grammar, independently of its target.
        ..LexerConfig::default()
    })
}

/// The loader's configured CST statement reader, retaining original word tokens.
/// The supplied configuration is used whole; extracted bodies use `nested()`.
#[must_use]
pub fn pack_source_statements(source: &str, config: LexerConfig) -> Vec<SegmentedCommand> {
    let (document, _warnings) = build_document(source, config);
    segments_from_document(document, &SourceMap::new(source))
}

// Navigation needs the original complete-command prefix, rather than the loader's
// deliberately recovered rows. The shared parse-cut owner supplies the ordinal;
// the original CST rows and full configuration remain the SpecTcl source owner.
fn complete_pack_source_statements(
    source: &str,
    config: LexerConfig,
) -> Option<Vec<SegmentedCommand>> {
    let image = SourceImage::document(source);
    let cut = tcl_lexer::first_parse_cut_image_checked(&image, config).ok()?;
    let mut rows = pack_source_statements(source, config);
    if let Some(cut) = cut {
        if cut.command >= rows.len() {
            return None;
        }
        rows.truncate(cut.command);
    }
    Some(rows)
}

/// Whether a captured source word is already its value on the static loader path.
/// Braced content is verbatim; other substitution and escape forms need evaluation.
#[must_use]
pub fn pack_source_word_is_literal(text: &str, braced: bool) -> bool {
    braced || !(text.contains('$') || text.contains('[') || text.contains('\\'))
}

/// The loader's classification of a block-shaped captured value.
#[must_use]
pub fn pack_value_is_block(text: &str) -> bool {
    text.is_empty() || text.contains(char::is_whitespace)
}

/// Why a row's arguments do not name one included file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludeNameUnavailable {
    /// Exactly one nonempty argument is required.
    Arity,
    /// File names cannot carry separators or a parent-path component spelling.
    PathStructure,
}

/// The single include-name admission rule, before any resolver is called.
/// Surface-versus-file row classification is independent of this name predicate.
///
/// # Errors
///
/// Returns `Arity` unless exactly one nonempty word is supplied, or
/// `PathStructure` when that word contains a separator or `..`.
pub fn include_file_name<'a>(words: &[&'a str]) -> Result<&'a str, IncludeNameUnavailable> {
    let [name] = words else {
        return Err(IncludeNameUnavailable::Arity);
    };
    if name.is_empty() {
        return Err(IncludeNameUnavailable::Arity);
    }
    if name.contains(['/', '\\']) || name.contains("..") {
        return Err(IncludeNameUnavailable::PathStructure);
    }
    Ok(name)
}

/// The loader's sibling-file policy, without reading or testing the path.
/// There is no tilde expansion, workspace fallback or canonicalisation.
#[must_use]
pub fn include_file_path(including_file: &Path, name: &str) -> Option<PathBuf> {
    let name = include_file_name(&[name]).ok()?;
    Some(including_file.parent()?.join(name))
}

/// The numeric major component, independently of vocabulary support or currency.
#[must_use]
pub fn declared_pack_major(version: &str) -> Option<u32> {
    version
        .split('.')
        .next()
        .and_then(|major| major.parse().ok())
}

/// Whether a declared vocabulary major fails the loader closed.
#[must_use]
pub fn unsupported_pack_major(version: &str) -> bool {
    declared_pack_major(version).is_some_and(|major| major > NEWEST_SUPPORTED_MAJOR)
}

fn literal_statement(row: &SegmentedCommand) -> bool {
    !row.is_partial
        && row
            .expand_word
            .as_ref()
            .is_none_or(|expanded| !expanded.iter().any(|v| *v))
        && row.texts.len() == row.argv.len()
        && row.texts.iter().zip(&row.argv).all(|(text, token)| {
            pack_source_word_is_literal(text, token.kind == TokenType::Str && !token.in_quote)
        })
}

fn source_line(source: &str, offset: usize) -> u32 {
    u32::try_from(
        source[..offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count(),
    )
    .unwrap_or(u32::MAX)
    .saturating_add(1)
}

/// The first pack's body word selected by the shared source grammar.
/// No interpreter dispatch or registration is implied by this source container.
#[derive(Debug)]
pub struct PackSourceBody<'a> {
    source: &'a str,
    config: LexerConfig,
    header: SegmentedCommand,
    base: usize,
    end: usize,
}

impl PackSourceBody<'_> {
    /// Body text exactly as the loader's existing CST row reader captured it.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.header.texts[3]
    }
    /// Whether the actual body token is braced, as required by the verbatim index.
    #[must_use]
    pub fn is_braced(&self) -> bool {
        self.header
            .argv
            .get(3)
            .is_some_and(|token| token.kind == TokenType::Str && !token.in_quote)
    }
    /// One-based line of the original header command.
    #[must_use]
    pub fn statement_line(&self) -> u32 {
        source_line(self.source, self.header.argv[0].span.start() as usize)
    }
    /// One-based line of the original body word's opening delimiter.
    #[must_use]
    pub fn body_line(&self) -> u32 {
        source_line(self.source, self.header.argv[3].span.start() as usize)
    }
    /// Immediate captured body rows, with body-relative spans and nested purpose.
    #[must_use]
    pub fn statements(&self) -> Vec<SegmentedCommand> {
        pack_source_statements(self.text(), self.config.nested())
    }
}

/// Select the first authored `speclib NAME VERSION BODY` source container.
/// Captured body text remains separate from its original raw source extent.
#[must_use]
pub fn first_pack_source_body(source: &str, config: LexerConfig) -> Option<PackSourceBody<'_>> {
    let header = pack_source_statements(source, config)
        .into_iter()
        .find(|row| row.name() == "speclib")?;
    pack_source_body(source, config, header)
}

fn pack_source_body(
    source: &str,
    config: LexerConfig,
    header: SegmentedCommand,
) -> Option<PackSourceBody<'_>> {
    let body = header.argv.get(3)?;
    let base = body.span.start() as usize + usize::from(body.content_offset);
    let end = body.span.end() as usize;
    header.texts.get(3)?;
    source.get(base..end)?;
    Some(PackSourceBody {
        source,
        config,
        header,
        base,
        end,
    })
}

/// Whether an immediate pack row has the file-include form rather than a roster.
/// Name/cardinality admission remains a separate shared query.
#[must_use]
pub fn is_pack_file_include_row(row: &SegmentedCommand) -> bool {
    row.name() == "include" && row.texts.get(1).is_none_or(|word| word != "from")
}

/// An original static file-include row in the first authored pack body.
/// Its name extent belongs to this exact source/configuration, never decoded data.
#[derive(Debug)]
pub struct PackSourceInclude<'a> {
    parent: Arc<PackSourceBody<'a>>,
    row: SegmentedCommand,
    name: String,
    name_span: Range<usize>,
}

impl<'a> PackSourceInclude<'a> {
    /// Complete original document from which this row was parsed.
    #[must_use]
    pub fn source(&self) -> &'a str {
        self.parent.source
    }
    /// The complete supplied document parser configuration.
    #[must_use]
    pub fn config(&self) -> LexerConfig {
        self.parent.config
    }
    /// The complete actual source row, with body-relative word spans.
    #[must_use]
    pub const fn original_row(&self) -> &SegmentedCommand {
        &self.row
    }
    /// Original body base for the retained row.
    #[must_use]
    pub fn body_base(&self) -> usize {
        self.parent.base
    }
    /// The loader-admitted literal include file name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Original content extent, excluding only the actual opening/closing wrapper.
    #[must_use]
    pub fn name_span(&self) -> Range<usize> {
        self.name_span.clone()
    }
    /// The sibling the loader would ask its file context to read, without IO.
    #[must_use]
    pub fn sibling_path(&self, including_file: &Path) -> Option<PathBuf> {
        include_file_path(including_file, &self.name)
    }
}

/// Literal file-include syntax in the first pack, without running its program.
/// Dynamic/malformed rows, nested command bodies and surface includes supply no
/// file target. This is not a complete include inventory or registration result.
#[must_use]
pub fn pack_source_includes(source: &str, config: LexerConfig) -> Vec<PackSourceInclude<'_>> {
    let Some(header) = complete_pack_source_statements(source, config)
        .and_then(|rows| rows.into_iter().find(|row| row.name() == "speclib"))
    else {
        return Vec::new();
    };
    let Some(body) = pack_source_body(source, config, header) else {
        return Vec::new();
    };
    let pack = &body.header;
    if body.source.get(body.base..body.end) != Some(body.text())
        || !literal_statement(pack)
        || pack
            .texts
            .get(1)
            .is_none_or(|name| !name.is_empty() && pack_value_is_block(name))
        || pack
            .texts
            .get(2)
            .is_none_or(|version| unsupported_pack_major(version))
    {
        return Vec::new();
    }
    let Some(rows) = complete_pack_source_statements(body.text(), config.nested()) else {
        return Vec::new();
    };
    let body_base = body.base;
    let body = Arc::new(body);
    let mut includes = Vec::new();
    for row in rows {
        if !is_pack_file_include_row(&row) || !literal_statement(&row) {
            continue;
        }
        let arguments: Vec<&str> = row.texts.iter().skip(1).map(String::as_str).collect();
        let Ok(name) = include_file_name(&arguments) else {
            continue;
        };
        let Some(token) = row.argv.get(1) else {
            continue;
        };
        let start = body_base + token.span.start() as usize + usize::from(token.content_offset);
        let end = body_base + token.span.end() as usize;
        // Preserve original extent even when wrappers differ; no label-to-span inverse.
        if source.get(start..end).is_none() {
            continue;
        }
        let name = name.to_owned();
        includes.push(PackSourceInclude {
            parent: Arc::clone(&body),
            row,
            name,
            name_span: start..end,
        });
    }
    includes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_source_major_query_keeps_numeric_and_supported_questions_separate() {
        for (text, numeric, unsupported) in [
            ("2.1", Some(2), false),
            ("3.0", Some(3), true),
            ("0.15", Some(0), false),
            ("", None, false),
            ("future", None, false),
        ] {
            assert_eq!(declared_pack_major(text), numeric);
            assert_eq!(unsupported_pack_major(text), unsupported);
        }
        // Bare name admission remains separate from the actual roster-row selector.
        assert_eq!(include_file_name(&["from"]), Ok("from"));
        assert_eq!(
            include_file_name(&["from", "tcl"]),
            Err(IncludeNameUnavailable::Arity)
        );
        let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        assert!(pack_source_includes("speclib p 2.0 {include from}", config).is_empty());
    }

    #[test]
    fn pack_source_includes_retain_actual_rows_config_and_wrapped_unicode_extent() {
        let source =
            "\u{feff}speclib \u{03c0} 2.0 {\n include {shared \u{03b1}.tclspec}\n include \"~\"\n}"
                .to_owned();
        let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        let rows = pack_source_includes(&source, config);
        assert_eq!(rows.len(), 2);
        assert!(std::ptr::eq(rows[0].source(), source.as_str()));
        assert_eq!(rows[0].config(), config);
        assert_eq!(rows[0].name(), "shared \u{03b1}.tclspec");
        assert_eq!(&source[rows[0].name_span()], rows[0].name());
        assert_eq!(&source[rows[1].name_span()], "~");
        assert_eq!(
            rows[0].original_row().texts,
            ["include", "shared \u{03b1}.tclspec"]
        );
        assert_eq!(
            rows[0].sibling_path(Path::new("/packs/p.tclspec")),
            Some(PathBuf::from("/packs/shared \u{03b1}.tclspec"))
        );
        assert_eq!(
            rows[1].sibling_path(Path::new("/packs/p.tclspec")),
            Some(PathBuf::from("/packs/~"))
        );
        assert!(rows[0].sibling_path(Path::new("/")).is_none());
    }

    #[test]
    fn pack_source_includes_preserve_whole_config_and_nested_bom_content() {
        let source = "\u{feff}speclib p 2.0 {\u{feff}include hidden; include shown}";
        let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        let rows = pack_source_includes(source, config);
        assert_eq!(
            rows.iter().map(PackSourceInclude::name).collect::<Vec<_>>(),
            ["shown"]
        );
        let content = LexerConfig {
            leading_bom: LeadingBom::Content,
            ..config
        };
        assert!(pack_source_includes(source, content).is_empty());
        assert_eq!(rows[0].config(), config);
    }

    #[test]
    fn pack_source_includes_withhold_dynamic_malformed_nested_and_unloaded_rows() {
        let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        for row in [
            "include $file",
            "include [set file]",
            "include ../outside",
            "include sub/file",
            "include {}",
            "include one two",
            "include from",
            "include from tcl into jim {a}",
            "command c {include hidden}",
            "{*}include expanded",
        ] {
            let source = format!("speclib p 2.0 {{{row}}}");
            assert!(pack_source_includes(&source, config).is_empty(), "{row}");
        }
        for source in [
            "speclib p 3.0 {include hidden}",
            "speclib {not a name} 2.0 {include hidden}",
            "speclib p 2.0 {include {unclosed}",
            "speclib $p 2.0 {include hidden}",
        ] {
            assert!(pack_source_includes(source, config).is_empty(), "{source}");
        }
        let source = "speclib p 2.0 {include first}; speclib q 2.0 {include second}";
        assert_eq!(
            pack_source_includes(source, config)
                .iter()
                .map(PackSourceInclude::name)
                .collect::<Vec<_>>(),
            ["first"]
        );
        // Braced sigils are literal file-name data; the same unbraced word is dynamic.
        let source = "speclib p 2.0 {include {$file}}";
        assert_eq!(pack_source_includes(source, config)[0].name(), "$file");
    }

    #[test]
    fn pack_source_navigation_keeps_complete_prefix_without_loader_recovery() {
        let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        let malformed = "speclib p 2.0 {include {unclosed}";
        // The loader's original recovery projection remains available. It cannot
        // supply a navigation target without a complete original header.
        assert!(first_pack_source_body(malformed, config).is_some());
        assert!(!pack_source_statements(malformed, config).is_empty());
        assert!(pack_source_includes(malformed, config).is_empty());

        for source in [
            "speclib p 2.0 {include kept}; trailing {unclosed",
            "speclib p 2.0 {include kept; include \"unclosed}",
        ] {
            assert_eq!(
                pack_source_includes(source, config)
                    .iter()
                    .map(PackSourceInclude::name)
                    .collect::<Vec<_>>(),
                ["kept"],
                "{source}"
            );
        }
        assert!(pack_source_includes("speclib p 2.0 {include \"unclosed}", config).is_empty());
    }
}

/// Distinct data channels of the fixed SpecTcl source authoring language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackWordSourcePurpose {
    /// Preserve the loader's captured CST text, including braced continuations.
    CapturedRegistration,
    /// Preserve both the captured text and the static Unicode descriptor value.
    StaticDescriptor,
}

/// Historical presentation of an authored SpecTcl word, not a safety proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackWordSourceStyle {
    /// Prefer the existing compact ASCII presentation, otherwise use braces.
    Compact,
    /// Keep prose, blocks and explicitly grouped input braced.
    Braced,
}

// This is the authored export/Studio presentation recipe, not a Tcl parser.
// Every proposed spelling below is separately joined to the actual loader CST.
fn compact_pack_word_presentation(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('#')
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || "-_.:/+@=,<>*?!%^~|'`".contains(c) && c.is_ascii()
        })
}

fn pack_word_source_matches(proposal: &str, text: &str, purpose: PackWordSourcePurpose) -> bool {
    let admits = || -> Option<()> {
        let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
        let end = u32::try_from(proposal.len()).ok()?;
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::document(proposal),
            tcl_lexer::Span::new(0, end),
            config,
        )
        .ok()?;
        let [command] = plan.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        if plan.fatal_tail.is_some()
            || word.group().expand
            || word.span() != tcl_lexer::Span::new(0, end)
        {
            return None;
        }
        let rows = pack_source_statements(proposal, config);
        let [row] = rows.as_slice() else {
            return None;
        };
        if row.is_partial
            || row.argv.len() != 1
            || row.texts.as_slice() != [text]
            || !pack_source_word_is_literal(text, word.group().kind == tcl_lexer::WordKind::Braced)
        {
            return None;
        }
        if purpose == PackWordSourcePurpose::StaticDescriptor
            && !tcl_syntax::word_rules::lexical_unicode_source_word_matches(proposal, text, config)
        {
            return None;
        }
        Some(())
    };
    admits().is_some()
}

/// Author one word under the actual fixed SpecTcl loader grammar.
/// The real complete CST row must reproduce the raw captured text. Descriptor
/// data additionally requires exact static Unicode value correspondence.
/// This creates proposed source, never an original word or execution recipe.
#[must_use]
pub fn pack_verbatim_source_word(
    text: &str,
    purpose: PackWordSourcePurpose,
    style: PackWordSourceStyle,
) -> Option<String> {
    let proposal = if style == PackWordSourceStyle::Compact && compact_pack_word_presentation(text)
    {
        text.to_owned()
    } else {
        format!("{{{text}}}")
    };
    pack_word_source_matches(&proposal, text, purpose).then_some(proposal)
}

/// Author a descriptor list with exact element and outer-word correspondence.
/// Single compact elements retain the historical ungrouped presentation.
#[must_use]
pub fn pack_descriptor_list_source_word<S: AsRef<str>>(items: &[S]) -> Option<String> {
    if let [only] = items
        && compact_pack_word_presentation(only.as_ref())
    {
        return pack_verbatim_source_word(
            only.as_ref(),
            PackWordSourcePurpose::StaticDescriptor,
            PackWordSourceStyle::Compact,
        );
    }
    let words = items
        .iter()
        .map(|item| {
            pack_verbatim_source_word(
                item.as_ref(),
                PackWordSourcePurpose::StaticDescriptor,
                PackWordSourceStyle::Compact,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let value = words.join(" ");
    let config = pack_source_lexer_config(SourceSegmentationPurpose::WholeInput);
    let decoded = tcl_syntax::word_rules::WordValueRules::from_config(&config)
        .split_list(&value)
        .ok()?;
    (decoded.len() == items.len()
        && decoded
            .iter()
            .zip(items)
            .all(|(a, b)| a.as_ref() == b.as_ref()))
    .then_some(())?;
    pack_verbatim_source_word(
        &value,
        PackWordSourcePurpose::StaticDescriptor,
        PackWordSourceStyle::Braced,
    )
}

/// Historical quoted recovery proposal for an export that already records a
/// loss. Escaping does not establish raw CST round-trip or descriptor value;
/// callers must retain the explicit ExportLoss rather than use it as admission.
#[must_use]
pub fn pack_lossy_quoted_source_word(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if matches!(c, '"' | '\\' | '$' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

#[cfg(test)]
mod verbatim_pack_source_controls {
    use super::*;
    #[test]
    fn captured_continuations_are_not_static_descriptor_values() {
        let text = "one \\\n  two";
        assert_eq!(
            pack_verbatim_source_word(
                text,
                PackWordSourcePurpose::CapturedRegistration,
                PackWordSourceStyle::Braced
            ),
            Some(format!("{{{text}}}"))
        );
        assert_eq!(
            pack_verbatim_source_word(
                text,
                PackWordSourcePurpose::StaticDescriptor,
                PackWordSourceStyle::Braced
            ),
            None
        );
        for text in ["a { brace", "trailing\\", "} ; command"] {
            for purpose in [
                PackWordSourcePurpose::CapturedRegistration,
                PackWordSourcePurpose::StaticDescriptor,
            ] {
                assert!(
                    pack_verbatim_source_word(text, purpose, PackWordSourceStyle::Braced).is_none()
                );
            }
        }
    }
    #[test]
    fn authored_unicode_and_metacharacters_keep_the_actual_loader_channel() {
        for text in ["café", "${x}", "[command]", "# comment", "a b", ""] {
            let source = pack_verbatim_source_word(
                text,
                PackWordSourcePurpose::StaticDescriptor,
                PackWordSourceStyle::Compact,
            )
            .unwrap();
            assert!(pack_word_source_matches(
                &source,
                text,
                PackWordSourcePurpose::StaticDescriptor
            ));
        }
        assert_eq!(
            pack_descriptor_list_source_word(&["one", "a b", "${x}", "café"]),
            Some("{one {a b} {${x}} {café}}".to_owned())
        );
        assert_eq!(pack_descriptor_list_source_word(&["one \\\n two"]), None);
    }
}
