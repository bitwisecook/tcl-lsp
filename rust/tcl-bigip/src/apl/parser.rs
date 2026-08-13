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

//! Structural APL parser — `parse_apl` -> [`AplModel`].

use std::sync::OnceLock;

use regex::Regex;

use crate::range::{Position, Range};

use super::model::{AplField, AplInclude, AplModel, AplSection, AplTable};

/// APL field-type keywords (sorted into the canonical set order used to build
/// the alternation).
const FIELD_TYPE_KEYWORDS: &[&str] = &[
    "addrport",
    "choice",
    "disena",
    "editchoice",
    "enadis",
    "enadisdry",
    "falsetrue",
    "indefint",
    "message",
    "multichoice",
    "noyes",
    "password",
    "string",
    "tcpprof",
    "truefalse",
    "yesno",
];

fn field_decl_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"(?m)^\s*({})\s+(\S+)",
            FIELD_TYPE_KEYWORDS.join("|")
        ))
        .unwrap()
    })
}

fn section_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*section\s+(\S+)\s*\{").unwrap())
}

fn table_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*table\s+(\S+)\s*\{").unwrap())
}

fn define_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^\s*define\s+(\S+)\s+(\S+)").unwrap())
}

fn include_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?m)^\s*#include\s+"([^"]+)""#).unwrap())
}

fn required_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\brequired\b").unwrap())
}

/// Convert a byte offset to a code-point (character) index.
fn byte_to_cp(source: &str, byte: usize) -> usize {
    source
        .get(..byte.min(source.len()))
        .map_or(0, |s| s.chars().count())
}

/// Convert a byte offset to an LSP `(line, character)`.
///
/// `character` is a **UTF-16 code-unit** offset, per the LSP position encoding —
/// what [`crate::range::Position`] documents and what every other position in
/// this crate uses.  Counting *code points* instead would be right only for the
/// Basic Multilingual Plane: a single astral character (an emoji in an APL
/// `display` string) is one code point but two UTF-16 units, so every
/// diagnostic after it on that line would come out one column short.
fn offset_to_line_char(source: &str, byte: usize) -> (u32, u32) {
    let prefix = source.get(..byte.min(source.len())).unwrap_or(source);
    let line = u32::try_from(prefix.matches('\n').count()).unwrap_or(u32::MAX);
    let line_start = prefix.rfind('\n').map_or(0, |i| i + 1);
    let character = u32::try_from(
        source[line_start..byte.min(source.len())]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>(),
    )
    .unwrap_or(u32::MAX);
    (line, character)
}

/// Return the byte offset past the closing `}` matching the `{` at
/// `start`, skipping braces inside quoted strings.
fn find_brace_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut pos = start + 1;
    let mut depth = 1;
    while pos < len && depth > 0 {
        match bytes[pos] {
            b'"' => {
                pos += 1;
                while pos < len && bytes[pos] != b'"' {
                    if bytes[pos] == b'\\' {
                        pos += 1;
                    }
                    pos += 1;
                }
                pos += 1;
                continue;
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'\\' => pos += 1,
            _ => {}
        }
        pos += 1;
    }
    pos
}

/// Build a name range from a byte offset + the name (code-point spans).
fn name_range(source: &str, abs_byte: usize, name: &str) -> Range {
    let (line, character) = offset_to_line_char(source, abs_byte);
    let start_off = byte_to_cp(source, abs_byte);
    let name_len = u32::try_from(name.chars().count()).unwrap_or(0);
    let start_off_u = u32::try_from(start_off).unwrap_or(u32::MAX);
    Range {
        start: Position {
            line,
            character,
            offset: start_off_u,
        },
        end: Position {
            line,
            character: character + name_len,
            offset: start_off_u + name_len,
        },
    }
}

/// Extract field declarations from a brace-delimited block.
fn parse_fields_in_block(
    block_source: &str,
    prefix: &str,
    base_offset: usize,
    source: &str,
) -> Vec<(String, AplField)> {
    let mut fields: Vec<(String, AplField)> = Vec::new();
    for m in field_decl_re().captures_iter(block_source) {
        let whole = m.get(0).unwrap();
        let ftype = m.get(1).unwrap().as_str();
        let fname_m = m.get(2).unwrap();
        let fname = fname_m.as_str();
        if matches!(fname, "default" | "display" | "required" | "validator") {
            continue;
        }
        if fname.starts_with('{') || fname.starts_with('"') {
            continue;
        }
        let qname = if prefix.is_empty() {
            fname.to_owned()
        } else {
            format!("{prefix}.{fname}")
        };
        let rest_start = whole.end();
        let rest_end = block_source[rest_start..]
            .find('\n')
            .map_or(block_source.len(), |i| rest_start + i);
        let rest = &block_source[rest_start..rest_end];
        let is_req = required_re().is_match(whole.as_str()) || required_re().is_match(rest);

        let abs_byte = base_offset + fname_m.start();
        let field = AplField {
            name: fname.to_owned(),
            qualified_name: qname,
            field_type: ftype.to_owned(),
            is_required: is_req,
            range: name_range(source, abs_byte, fname),
        };
        // Last-wins on duplicate field name, first-seen order.
        if let Some(slot) = fields.iter_mut().find(|(k, _)| k == fname) {
            slot.1 = field;
        } else {
            fields.push((fname.to_owned(), field));
        }
    }
    fields
}

fn insert_field<'a>(all: &mut Vec<(String, AplField)>, fields: impl Iterator<Item = &'a AplField>) {
    for f in fields {
        if !all.iter().any(|(k, _)| *k == f.qualified_name) {
            all.push((f.qualified_name.clone(), f.clone()));
        }
    }
}

/// Parse APL source into a structured [`AplModel`].
#[must_use]
pub fn parse_apl(source: &str) -> AplModel {
    let mut model = AplModel::default();

    for m in include_re().captures_iter(source) {
        let whole = m.get(0).unwrap();
        let line = u32::try_from(source[..whole.start()].matches('\n').count()).unwrap_or(u32::MAX);
        model.includes.push(AplInclude {
            path: m.get(1).unwrap().as_str().to_owned(),
            line,
            resolved: false,
        });
    }

    for m in define_re().captures_iter(source) {
        let underlying = m.get(1).unwrap().as_str().to_owned();
        let name = m.get(2).unwrap().as_str().to_owned();
        // Last-wins assignment.
        if let Some(slot) = model.defines.iter_mut().find(|(k, _)| *k == name) {
            slot.1 = underlying;
        } else {
            model.defines.push((name, underlying));
        }
    }

    // (brace_start, brace_end, name) for nested-table scoping.
    let mut section_ranges: Vec<(usize, usize, String)> = Vec::new();
    for m in section_re().captures_iter(source) {
        let whole = m.get(0).unwrap();
        let name_m = m.get(1).unwrap();
        let sec_name = name_m.as_str().to_owned();
        let brace_pos = whole.end() - 1;
        let end_pos = find_brace_end(source, brace_pos);
        let block = &source[brace_pos + 1..end_pos.saturating_sub(1).max(brace_pos + 1)];
        section_ranges.push((brace_pos, end_pos, sec_name.clone()));

        let fields = parse_fields_in_block(block, &sec_name, brace_pos + 1, source);
        insert_field(&mut model.all_fields, fields.iter().map(|(_, f)| f));
        let section = AplSection {
            range: Some(name_range(source, name_m.start(), &sec_name)),
            name: sec_name.clone(),
            qualified_name: sec_name.clone(),
            fields,
        };
        if let Some(slot) = model.sections.iter_mut().find(|(k, _)| *k == sec_name) {
            slot.1 = section;
        } else {
            model.sections.push((sec_name, section));
        }
    }

    for m in table_re().captures_iter(source) {
        let whole = m.get(0).unwrap();
        let name_m = m.get(1).unwrap();
        let tbl_name = name_m.as_str().to_owned();
        let brace_pos = whole.end() - 1;
        let end_pos = find_brace_end(source, brace_pos);
        let block = &source[brace_pos + 1..end_pos.saturating_sub(1).max(brace_pos + 1)];

        let parent_prefix = section_ranges
            .iter()
            .find(|(s, e, _)| *s < whole.start() && whole.start() < *e)
            .map_or("", |(_, _, n)| n.as_str());
        let qualified = if parent_prefix.is_empty() {
            tbl_name.clone()
        } else {
            format!("{parent_prefix}.{tbl_name}")
        };

        let columns = parse_fields_in_block(block, &qualified, brace_pos + 1, source);
        insert_field(&mut model.all_fields, columns.iter().map(|(_, f)| f));
        let table = AplTable {
            range: Some(name_range(source, name_m.start(), &tbl_name)),
            name: tbl_name,
            qualified_name: qualified.clone(),
            columns,
        };
        if let Some(slot) = model.tables.iter_mut().find(|(k, _)| *k == qualified) {
            slot.1 = table;
        } else {
            model.tables.push((qualified, table));
        }
    }

    model
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../../samples/bigip/apl/sample.apl");

    fn get<'a>(v: &'a [(String, AplField)], k: &str) -> &'a AplField {
        &v.iter().find(|(n, _)| n == k).unwrap().1
    }

    #[test]
    fn parses_sample_apl() {
        let m = parse_apl(SAMPLE);
        assert_eq!(
            m.sections
                .iter()
                .map(|(k, _)| k.as_str())
                .collect::<Vec<_>>(),
            ["basic", "ssl"]
        );
        assert_eq!(
            m.tables.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            ["ssl.members", "standalone"]
        );
        assert_eq!(
            m.defines,
            vec![("yesno_choice".to_owned(), "choice".to_owned())]
        );
        assert_eq!(m.includes.len(), 1);
        assert_eq!(m.includes[0].path, "common.apl");
        assert_eq!(m.includes[0].line, 0);
        assert!(!m.includes[0].resolved);

        // basic.addr: required string at line 4, char 11, offset 80..84.
        let addr = get(
            &m.sections
                .iter()
                .find(|(k, _)| k == "basic")
                .unwrap()
                .1
                .fields,
            "addr",
        );
        assert!(addr.is_required);
        assert_eq!(addr.field_type, "string");
        assert_eq!(
            (
                addr.range.start.line,
                addr.range.start.character,
                addr.range.start.offset
            ),
            (4, 11, 80)
        );
        assert_eq!(addr.range.end.offset, 84);

        // The section block scan picks up the nested table's columns too.
        let ssl_fields = &m
            .sections
            .iter()
            .find(|(k, _)| k == "ssl")
            .unwrap()
            .1
            .fields;
        let mut names: Vec<&str> = ssl_fields.iter().map(|(k, _)| k.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["enable", "member_addr", "member_port"]);
        assert!(
            get(
                &m.tables
                    .iter()
                    .find(|(k, _)| k == "ssl.members")
                    .unwrap()
                    .1
                    .columns,
                "member_port"
            )
            .is_required
        );

        // Flattened all_fields includes both section- and table-qualified names.
        let all: Vec<&str> = m.all_fields.iter().map(|(k, _)| k.as_str()).collect();
        for q in [
            "basic.addr",
            "ssl.member_addr",
            "ssl.members.member_addr",
            "standalone.col1",
        ] {
            assert!(all.contains(&q), "missing {q} in {all:?}");
        }
        assert_eq!(m.tcl_variable_names()[0], "::basic__addr");
    }

    #[test]
    fn tcl_name_roundtrip() {
        use super::super::model::{apl_name_to_tcl_var, tcl_var_to_apl_name};
        assert_eq!(
            tcl_var_to_apl_name("::basic__addr").as_deref(),
            Some("basic.addr")
        );
        assert_eq!(tcl_var_to_apl_name("nodelimiter"), None);
        assert_eq!(apl_name_to_tcl_var("basic.addr"), "::basic__addr");
    }
}
