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

//! Low-level block / property / header extraction helpers.
//!
//! Only ASCII structural bytes (`{`, `}`, `"`, `\`, whitespace, `#`) are
//! inspected, so every recorded offset lands on a UTF-8 char boundary.

/// A parsed top-level config stanza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// Header text, e.g. `"ltm virtual /Common/my_vs"`.
    pub header: String,
    /// Body text between the outermost `{ }` (braces excluded).
    pub body: String,
    /// Byte offset of the opening `{`.
    pub start_offset: usize,
    /// Byte offset one past the closing `}`.
    pub end_offset: usize,
}

/// A top-level key/value property parsed from a block body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Property key.
    pub key: String,
    /// Property value (stripped).
    pub value: String,
    /// Value start offset within the body, when captured.
    pub value_start: Option<usize>,
    /// Value end offset (exclusive) within the body, when captured.
    pub value_end: Option<usize>,
}

/// Extract all top-level `keyword ... { ... }` blocks from `source`.
/// Handles nested braces and respects quoted strings.
#[must_use]
pub fn extract_blocks(source: &str) -> Vec<Block> {
    let bytes = source.as_bytes();
    let length = bytes.len();
    let mut blocks = Vec::new();
    let mut pos = 0;

    while pos < length {
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }
        if bytes[pos] == b'#' {
            while pos < length && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let header_start = pos;
        let mut hit_newline = false;
        while pos < length && bytes[pos] != b'{' {
            match bytes[pos] {
                b'\n' => {
                    pos += 1;
                    hit_newline = true;
                    break;
                }
                // tmsh permits braces inside a quoted object name; skip over
                // quoted spans (and escapes) so the real opening brace is found.
                b'\\' if pos + 1 < length => pos += 1,
                b'"' => {
                    pos += 1;
                    while pos < length && bytes[pos] != b'"' {
                        if bytes[pos] == b'\\' && pos + 1 < length {
                            pos += 1;
                        }
                        pos += 1;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        if hit_newline {
            continue;
        }
        if pos >= length || bytes[pos] != b'{' {
            break;
        }

        let header = source[header_start..pos].trim().to_owned();
        let brace_start = pos;
        pos += 1; // skip opening brace
        let mut depth = 1;
        let body_start = pos;
        while pos < length && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'\\' if pos + 1 < length => pos += 1,
                b'"' => {
                    pos += 1;
                    while pos < length && bytes[pos] != b'"' {
                        if bytes[pos] == b'\\' && pos + 1 < length {
                            pos += 1;
                        }
                        pos += 1;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        let body = source[body_start..pos.saturating_sub(1)].to_owned();
        blocks.push(Block {
            header,
            body,
            start_offset: brace_start,
            end_offset: pos,
        });
    }

    blocks
}

/// Scan a property value starting at `pos` (guaranteed not to be a newline).
/// Returns `(val_start, val_end, new_pos)`.
///
/// Three value shapes: a braced `{…}` block (quote- and escape-aware), a
/// double-quoted string — which may contain an embedded literal newline, so it
/// scans to the matching close quote rather than end-of-line —
/// and a bareword value to end-of-line. `val_end` is clamped to `length` so an
/// unterminated quote can't overshoot the slice.
fn scan_property_value(bytes: &[u8], length: usize, mut pos: usize) -> (usize, usize, usize) {
    let val_start = pos;
    if bytes[pos] == b'{' {
        pos += 1;
        let mut depth = 1;
        while pos < length && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'\\' if pos + 1 < length => pos += 1,
                b'"' => {
                    pos += 1;
                    while pos < length && bytes[pos] != b'"' {
                        if bytes[pos] == b'\\' && pos + 1 < length {
                            pos += 1;
                        }
                        pos += 1;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
    } else if bytes[pos] == b'"' {
        pos += 1;
        while pos < length && bytes[pos] != b'"' {
            if bytes[pos] == b'\\' && pos + 1 < length {
                pos += 2;
            } else {
                pos += 1;
            }
        }
        if pos < length {
            pos += 1; // consume the closing quote
        }
    } else {
        while pos < length && bytes[pos] != b'\n' {
            pos += 1;
        }
    }
    (val_start, pos.min(length), pos)
}

/// Parse top-level `key value` properties from a block body, capturing
/// each value's span relative to the body.
#[must_use]
pub fn parse_properties_with_spans(body: &str) -> Vec<(String, Property)> {
    let bytes = body.as_bytes();
    let length = bytes.len();
    // Preserve insertion order with last-wins semantics on duplicate keys.
    let mut props: Vec<(String, Property)> = Vec::new();
    let mut pos = 0;

    while pos < length {
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }
        // A `#` at statement position starts a comment line, not a property;
        // skip it so it isn't captured as a spurious `#`-keyed property.
        if bytes[pos] == b'#' {
            while pos < length && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let key_start = pos;
        while pos < length && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | b'{') {
            pos += 1;
        }
        let key = body[key_start..pos].trim().to_owned();
        if key.is_empty() {
            pos += 1;
            continue;
        }

        while pos < length && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }

        if pos >= length || bytes[pos] == b'\n' {
            insert_prop(
                &mut props,
                Property {
                    key,
                    value: String::new(),
                    value_start: None,
                    value_end: None,
                },
            );
            continue;
        }

        let (val_start, val_end, new_pos) = scan_property_value(bytes, length, pos);
        pos = new_pos;
        insert_prop(
            &mut props,
            Property {
                key,
                value: body[val_start..val_end].trim().to_owned(),
                value_start: Some(val_start),
                value_end: Some(val_end),
            },
        );
    }

    props
}

/// Insert a property with last-wins-on-key semantics while preserving
/// first-seen order.
fn insert_prop(props: &mut Vec<(String, Property)>, prop: Property) {
    if let Some(slot) = props.iter_mut().find(|(k, _)| *k == prop.key) {
        slot.1 = prop;
    } else {
        props.push((prop.key.clone(), prop));
    }
}

/// Parse simple `key value` properties from a block body, dropping
/// spans.
#[must_use]
pub fn parse_properties(body: &str) -> Vec<(String, String)> {
    parse_properties_with_spans(body)
        .into_iter()
        .map(|(k, p)| (k, p.value))
        .collect()
}

/// Strip a single layer of surrounding double quotes, when present.
#[must_use]
pub fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Extract top-level item names from a braced block, skipping nested
/// sub-blocks.
#[must_use]
pub fn parse_list_block(braced: &str) -> Vec<String> {
    let mut inner = braced.trim();
    inner = inner.strip_prefix('{').unwrap_or(inner);
    inner = inner.strip_suffix('}').unwrap_or(inner);
    let bytes = inner.as_bytes();
    let length = bytes.len();
    let mut items = Vec::new();
    let mut pos = 0;

    while pos < length {
        let loop_start = pos;
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }

        let name_start = pos;
        // A record key may be double-quoted because it contains spaces (a
        // data-group key like `"Mozilla/5.0 (Windows)"`); scan the whole quoted
        // run as one key instead of splitting it on the inner space, matching
        // the unquoting the header tokeniser already does.
        let name = if bytes[pos] == b'"' {
            pos += 1;
            let content_start = pos;
            while pos < length && bytes[pos] != b'"' {
                if bytes[pos] == b'\\' && pos + 1 < length {
                    pos += 2;
                } else {
                    pos += 1;
                }
            }
            let content = inner[content_start..pos].to_owned();
            if pos < length {
                pos += 1; // consume the closing quote
            }
            content
        } else {
            while pos < length && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}')
            {
                if bytes[pos] == b'\\' && pos + 1 < length {
                    pos += 2;
                    continue;
                }
                pos += 1;
            }
            inner[name_start..pos].trim().to_owned()
        };

        while pos < length && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }

        if pos < length && bytes[pos] == b'{' {
            pos += 1;
            let mut depth = 1;
            while pos < length && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'\\' if pos + 1 < length => pos += 1,
                    _ => {}
                }
                pos += 1;
            }
        }

        if !name.is_empty() && name != "{" && name != "}" {
            items.push(name);
        }

        if pos == loop_start {
            break;
        }
    }

    items
}

/// Extract `(name, body)` pairs from a keyed-block list.
#[must_use]
pub fn parse_keyed_block_entries(braced: &str) -> Vec<(String, String)> {
    let mut inner = braced.trim();
    inner = inner.strip_prefix('{').unwrap_or(inner);
    inner = inner.strip_suffix('}').unwrap_or(inner);
    let bytes = inner.as_bytes();
    let length = bytes.len();
    let mut entries = Vec::new();
    let mut pos = 0;

    while pos < length {
        let loop_start = pos;
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }

        let name_start = pos;
        while pos < length && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}') {
            if bytes[pos] == b'\\' && pos + 1 < length {
                pos += 2;
                continue;
            }
            pos += 1;
        }
        let name = inner[name_start..pos].trim().to_owned();

        while pos < length && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }

        let mut body = String::new();
        if pos < length && bytes[pos] == b'{' {
            let body_start = pos + 1;
            pos += 1;
            let mut depth = 1;
            while pos < length && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    b'\\' if pos + 1 < length => pos += 1,
                    _ => {}
                }
                pos += 1;
            }
            body.push_str(&inner[body_start..pos]);
            if pos < length {
                pos += 1; // consume closing brace
            }
        }

        if !name.is_empty() && name != "{" && name != "}" {
            entries.push((name, body));
        }

        if pos == loop_start {
            break;
        }
    }

    entries
}

/// One entry returned by [`parse_keyed_block_entries_with_offsets`]:
/// `(key, body, key_start, key_end, body_start, body_end, item_start,
/// item_end)`. All offsets are relative to the `braced` input.
pub type KeyedBlockEntryOffsets = (String, String, usize, usize, usize, usize, usize, usize);

/// Like [`parse_keyed_block_entries`] but also returns per-entry offsets.
///
/// Each entry is `(key, body, key_start, key_end, body_start, body_end,
/// item_start, item_end)`. `item_start`/`item_end` bracket the entire
/// `key { body }` span; `key_start`/`key_end` bracket just the key;
/// `body_start`/`body_end` bracket the body bytes inside the inner
/// `{ ... }` (without the braces). All offsets are relative to `braced`.
#[must_use]
pub fn parse_keyed_block_entries_with_offsets(braced: &str) -> Vec<KeyedBlockEntryOffsets> {
    let bytes = braced.as_bytes();
    let length = bytes.len();
    let mut out = Vec::new();
    let mut pos = 0;

    // Skip the first `{` so item offsets reference the surrounding body.
    if pos < length && bytes[pos] == b'{' {
        pos += 1;
    }

    while pos < length {
        let loop_start = pos;
        while pos < length && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        if pos >= length {
            break;
        }
        if bytes[pos] == b'}' {
            break;
        }

        let item_start = pos;
        let key_start = pos;
        while pos < length && !matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | b'{' | b'}') {
            if bytes[pos] == b'\\' && pos + 1 < length {
                pos += 2;
                continue;
            }
            pos += 1;
        }
        let key_end = pos;
        let name = braced[key_start..key_end].trim().to_owned();

        // Skip horizontal whitespace before optional `{`.
        while pos < length && matches!(bytes[pos], b' ' | b'\t') {
            pos += 1;
        }

        let mut body = String::new();
        let mut body_start = pos;
        let mut body_end = pos;
        if pos < length && bytes[pos] == b'{' {
            body_start = pos + 1;
            pos += 1;
            let mut depth = 1;
            while pos < length && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    b'\\' if pos + 1 < length => pos += 1,
                    _ => {}
                }
                pos += 1;
            }
            body_end = pos;
            body.push_str(&braced[body_start..body_end]);
            if pos < length {
                pos += 1; // consume closing `}`
            }
        }
        let item_end = pos;

        if !name.is_empty() && name != "{" && name != "}" {
            out.push((
                name, body, key_start, key_end, body_start, body_end, item_start, item_end,
            ));
        }

        if pos == loop_start {
            break;
        }
    }

    out
}

/// Split `header` on whitespace, honouring `"..."` quoted spans and
/// backslash escapes.
#[must_use]
pub fn tokenise_header(header: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    let mut in_quote = false;
    let mut escape = false;
    for ch in header.chars() {
        if escape {
            buf.push(ch);
            escape = false;
            continue;
        }
        match ch {
            '\\' => escape = true,
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !buf.is_empty() {
                    tokens.push(std::mem::take(&mut buf));
                }
            }
            c => buf.push(c),
        }
    }
    if !buf.is_empty() {
        tokens.push(buf);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_nested_blocks() {
        let src = "ltm pool /Common/p {\n    members {\n        1.2.3.4:80 { }\n    }\n}\n";
        let blocks = extract_blocks(src);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].header, "ltm pool /Common/p");
        assert!(blocks[0].body.contains("members"));
        // start_offset points at the opening brace.
        assert_eq!(&src[blocks[0].start_offset..=blocks[0].start_offset], "{");
    }

    #[test]
    fn parses_properties_with_subblocks() {
        let body = "\n    monitor /Common/http\n    members {\n        a { }\n    }\n";
        let props = parse_properties_with_spans(body);
        let map: std::collections::HashMap<_, _> = props
            .iter()
            .map(|(k, p)| (k.as_str(), p.value.as_str()))
            .collect();
        assert_eq!(map.get("monitor"), Some(&"/Common/http"));
        assert!(map.get("members").unwrap().starts_with('{'));
    }

    #[test]
    fn list_and_keyed_blocks() {
        assert_eq!(
            parse_list_block("{ /Common/a /Common/b }"),
            vec!["/Common/a".to_owned(), "/Common/b".to_owned()]
        );
        let entries = parse_keyed_block_entries(
            "{ /Common/clientssl { context clientside } /Common/http { } }",
        );
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "/Common/clientssl");
        assert!(entries[0].1.contains("context clientside"));
    }

    #[test]
    fn list_block_key_may_be_quoted_with_spaces() {
        // A data-group record key quoted because it contains
        // spaces is one key, not several.
        let keys = parse_list_block(r#"{ "Mozilla/5.0 (Windows)" { data blocked } }"#);
        assert_eq!(keys, vec!["Mozilla/5.0 (Windows)".to_owned()]);
        // FP-guard: ordinary unquoted keys still split on whitespace.
        assert_eq!(
            parse_list_block("{ /Common/a /Common/b }"),
            vec!["/Common/a".to_owned(), "/Common/b".to_owned()]
        );
    }

    #[test]
    fn quoted_property_value_may_span_newlines() {
        // A double-quoted value with an embedded newline is one
        // value — `line two"` must not parse as a new key.
        let body = "description \"line one\nline two\"\nmonitor /Common/http\n";
        let props = parse_properties(body);
        let map: std::collections::HashMap<_, _> = props
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(map.get("description"), Some(&"\"line one\nline two\""));
        assert_eq!(map.get("monitor"), Some(&"/Common/http"));
        assert!(!map.contains_key("line"), "no spurious `line` key: {map:?}");
    }

    #[test]
    fn tokenise_quoted_header() {
        assert_eq!(
            tokenise_header("security bot-defense signature \"/Common/Microsoft Access\""),
            vec![
                "security".to_owned(),
                "bot-defense".to_owned(),
                "signature".to_owned(),
                "/Common/Microsoft Access".to_owned(),
            ]
        );
    }

    #[test]
    fn header_scan_is_quote_aware() {
        // A brace inside a quoted object name must not truncate the header at
        // that inner brace; the real body brace is the one after the name.
        let src = "ltm rule /Common/\"weird{name\" {\n    when HTTP_REQUEST { }\n}\n";
        let blocks = extract_blocks(src);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].header, "ltm rule /Common/\"weird{name\"");
        assert!(blocks[0].body.contains("when HTTP_REQUEST"));
        assert_eq!(&src[blocks[0].start_offset..=blocks[0].start_offset], "{");
    }

    #[test]
    fn unterminated_quote_in_braced_value_does_not_panic() {
        // A stray unbalanced `"` inside a braced value must not crash the parse
        // via an out-of-bounds slice; it captures a clamped value instead.
        let body = "description { \"unterminated\n";
        let props = parse_properties_with_spans(body);
        assert!(props.iter().any(|(k, _)| k == "description"));
    }

    #[test]
    fn comment_lines_in_body_are_not_properties() {
        let body = "\n    # a comment line\n    monitor /Common/http\n";
        let props = parse_properties_with_spans(body);
        assert!(
            !props.iter().any(|(k, _)| k == "#"),
            "a `#` comment must not become a property"
        );
        assert!(
            props
                .iter()
                .any(|(k, p)| k == "monitor" && p.value == "/Common/http")
        );
    }
}
