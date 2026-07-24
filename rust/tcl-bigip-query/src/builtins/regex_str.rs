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

//! Regex / string-category builtins — the regex family `match`, `sub`,
//! `gsub`, `test`, `scan`, `capture`, `splits`, plus the codepoint /
//! row-format helpers `index`, `ltrimstr`, `rtrimstr`, `explode`, `implode`,
//! `ascii`, `utf8bytelength`, `csv`, `tsv`.
//!
//! Behaviour notes:
//! - Every regex compile routes through the parent `safe_regex_compile`
//!   chokepoint (length + nested-quantifier guards) so the guard error
//!   text stays uniform across the regex family.
//! - The `regex` crate's dialect differs on
//!   backreferences / lookaround (documented divergence the user accepted).
//!   `sub` / `gsub` reproduce the `\g<1>`-style replacement-template syntax
//!   (`\1`, `\g<name>`, `\g<1>`) by expanding it by hand rather than using
//!   the crate's `$`-expansion, so the common-subset cases stay identical.
//! - `match` / `capture` / `scan` offsets are byte offsets in the `regex`
//!   crate vs code points; for ASCII inputs they coincide.
//!   `match` / `test` are boolean predicates here (this DSL's `match` is
//!   jq's `test`, not jq's structured `match`).
//! - `explode` / `implode` use Unicode code points (`char as u32`);
//!   `utf8bytelength` is the UTF-8 byte length; `ascii` is `chr` of an int.

use regex::Regex;

use crate::builtins::{
    BuiltinSpec, as_int, as_sequence, as_str, plain, safe_regex_compile, type_name,
};
use crate::errors::QueryError;
use crate::value::{self, Value};

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        plain("match", "string", 2, Some(2), false, bi_match),
        plain("sub", "string", 3, Some(4), false, bi_sub),
        plain("gsub", "string", 3, Some(4), false, bi_gsub),
        plain("test", "string", 2, Some(3), false, bi_test),
        plain("scan", "string", 2, Some(3), false, bi_scan),
        plain("capture", "string", 2, Some(3), false, bi_capture),
        plain("splits", "string", 2, Some(3), false, bi_splits),
        plain("index", "string", 2, Some(2), false, bi_index),
        plain("ltrimstr", "string", 2, Some(2), false, bi_ltrimstr),
        plain("rtrimstr", "string", 2, Some(2), false, bi_rtrimstr),
        plain("explode", "string", 1, Some(1), false, bi_explode),
        plain("implode", "string", 1, Some(1), true, bi_implode),
        plain("ascii", "string", 1, Some(1), false, bi_ascii),
        plain(
            "utf8bytelength",
            "string",
            1,
            Some(1),
            false,
            bi_utf8bytelength,
        ),
        plain("csv", "string", 1, None, false, bi_csv),
        plain("tsv", "string", 1, None, false, bi_tsv),
    ]
}

// Regex flag handling

/// Translate jq-style flag letters (`i` / `x` / `s` / `m`) to the leading
/// inline flag group the `regex` crate understands, validating each letter
/// the way `_jq_regex_flags` does (an unknown letter raises verbatim).
fn jq_regex_flags(flags: &str, name: &str) -> Result<String, QueryError> {
    let mut out = String::new();
    for ch in flags.chars() {
        match ch {
            'i' => out.push('i'),
            'x' => out.push('x'),
            's' => out.push('s'),
            'm' => out.push('m'),
            _ => {
                return Err(QueryError::builtin(format!(
                    "{name}: unsupported regex flag {} (supported: i, x, s, m)",
                    py_char_repr(ch)
                )));
            }
        }
    }
    Ok(out)
}

/// Repr-style rendering of a single character (used in the flag error text).
fn py_char_repr(ch: char) -> String {
    if ch == '\'' {
        "\"'\"".to_string()
    } else {
        format!("'{ch}'")
    }
}

/// Compile a pattern with optional jq flags, routing through the guard. The
/// flags are applied via a leading `(?flags)` inline group.
fn compile_with_flags(pattern: &str, flags: &str, name: &str) -> Result<Regex, QueryError> {
    let flag_letters = jq_regex_flags(flags, name)?;
    if flag_letters.is_empty() {
        safe_regex_compile(pattern, name)
    } else {
        safe_regex_compile(&format!("(?{flag_letters}){pattern}"), name)
    }
}

/// Read the optional trailing `flags` argument (the `flags != ""` test).
fn optional_flags(args: &[Value], idx: usize, name: &str) -> Result<String, QueryError> {
    match args.get(idx) {
        None => Ok(String::new()),
        Some(Value::Str(s)) if s.is_empty() => Ok(String::new()),
        Some(v) => as_str(v, name, idx + 1),
    }
}

// Regex builtins

fn bi_match(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "match", 1)?;
    let p = as_str(&args[1], "match", 2)?;
    let rx = safe_regex_compile(&p, "match")?;
    Ok(Value::Bool(rx.is_match(&s)))
}

fn bi_test(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "test", 1)?;
    let p = as_str(&args[1], "test", 2)?;
    let f = optional_flags(args, 2, "test")?;
    let rx = compile_with_flags(&p, &f, "test")?;
    Ok(Value::Bool(rx.is_match(&s)))
}

fn bi_sub(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "sub", 1)?;
    let p = as_str(&args[1], "sub", 2)?;
    let r = as_str(&args[2], "sub", 3)?;
    let f = optional_flags(args, 3, "sub")?;
    let rx = compile_with_flags(&p, &f, "sub")?;
    Ok(Value::Str(regex_sub(&rx, &s, &r, Some(1))))
}

fn bi_gsub(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "gsub", 1)?;
    let p = as_str(&args[1], "gsub", 2)?;
    let r = as_str(&args[2], "gsub", 3)?;
    let f = optional_flags(args, 3, "gsub")?;
    let rx = compile_with_flags(&p, &f, "gsub")?;
    Ok(Value::Str(regex_sub(&rx, &s, &r, None)))
}

/// Regex substitution with a replacement template. `count = Some(1)`
/// replaces only the first match (`sub`); `None` replaces all (`gsub`).
/// Replacement templates use `re` backref syntax (`\1`,
/// `\g<name>`, `\g<1>`); they are expanded by hand so the common subset
/// stays byte-identical despite the `regex` crate's native `$`-expansion.
fn regex_sub(rx: &Regex, text: &str, template: &str, count: Option<usize>) -> String {
    let mut out = String::new();
    let mut last_end = 0usize;
    for (replaced, caps) in rx.captures_iter(text).enumerate() {
        if let Some(limit) = count
            && replaced >= limit
        {
            break;
        }
        let m = caps.get(0).unwrap();
        out.push_str(&text[last_end..m.start()]);
        out.push_str(&expand_template(template, &caps));
        last_end = m.end();
    }
    out.push_str(&text[last_end..]);
    out
}

/// Expand a `\g<1>`-style replacement template against one match.
fn expand_template(template: &str, caps: &regex::Captures) -> String {
    let mut out = String::new();
    let chars: Vec<char> = template.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch != '\\' {
            out.push(ch);
            i += 1;
            continue;
        }
        // A backslash escape.
        if i + 1 >= chars.len() {
            out.push('\\');
            i += 1;
            continue;
        }
        let next = chars[i + 1];
        if next.is_ascii_digit() {
            // `\1` .. `\99` — numbered group (reads up to two digits).
            let mut j = i + 1;
            let mut num = String::new();
            while j < chars.len() && num.len() < 2 && chars[j].is_ascii_digit() {
                num.push(chars[j]);
                j += 1;
            }
            let group: usize = num.parse().unwrap();
            if let Some(g) = caps.get(group) {
                out.push_str(g.as_str());
            }
            i = j;
        } else if next == 'g' && i + 2 < chars.len() && chars[i + 2] == '<' {
            // `\g<name>` or `\g<1>`.
            let mut j = i + 3;
            let mut inner = String::new();
            while j < chars.len() && chars[j] != '>' {
                inner.push(chars[j]);
                j += 1;
            }
            // Skip the closing `>`.
            if j < chars.len() {
                j += 1;
            }
            if let Ok(num) = inner.parse::<usize>() {
                if let Some(g) = caps.get(num) {
                    out.push_str(g.as_str());
                }
            } else if let Some(g) = caps.name(&inner) {
                out.push_str(g.as_str());
            }
            i = j;
        } else if next == '\\' {
            out.push('\\');
            i += 2;
        } else if next == 'n' {
            out.push('\n');
            i += 2;
        } else if next == 't' {
            out.push('\t');
            i += 2;
        } else if next == 'r' {
            out.push('\r');
            i += 2;
        } else {
            // Unknown escape: keep the backslash and the character.
            out.push('\\');
            out.push(next);
            i += 2;
        }
    }
    out
}

fn bi_scan(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "scan", 1)?;
    let p = as_str(&args[1], "scan", 2)?;
    let f = optional_flags(args, 2, "scan")?;
    let rx = compile_with_flags(&p, &f, "scan")?;
    // `rx.groups == 0`: no capturing groups → matched substrings; otherwise
    // a list of capture values per match (the full match is not included).
    let n_groups = rx.captures_len() - 1;
    let mut out = Vec::new();
    if n_groups == 0 {
        for m in rx.find_iter(&s) {
            out.push(Value::Str(m.as_str().to_string()));
        }
    } else {
        for caps in rx.captures_iter(&s) {
            let groups: Vec<Value> = (1..=n_groups)
                .map(|g| match caps.get(g) {
                    Some(m) => Value::Str(m.as_str().to_string()),
                    None => Value::Null,
                })
                .collect();
            out.push(Value::List(groups));
        }
    }
    Ok(Value::List(out))
}

fn bi_capture(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "capture", 1)?;
    let p = as_str(&args[1], "capture", 2)?;
    let f = optional_flags(args, 2, "capture")?;
    // jq accepts `(?<name>...)`; rewrite to `(?P<name>...)`
    // spelling (also accepted by the `regex` crate).
    let converted = rewrite_named_groups(&p);
    let rx = compile_with_flags(&converted, &f, "capture")?;
    match rx.captures(&s) {
        None => Err(QueryError::builtin(format!(
            "capture: pattern {} did not match",
            py_str_repr(&p)
        ))),
        Some(caps) => {
            let mut m = indexmap::IndexMap::new();
            for name in rx.capture_names().flatten() {
                if let Some(g) = caps.name(name) {
                    m.insert(name.to_string(), Value::Str(g.as_str().to_string()));
                }
            }
            Ok(Value::Object(m))
        }
    }
}

/// Rewrite jq-style `(?<name>` to the `regex`-crate `(?P<name>`.
fn rewrite_named_groups(pattern: &str) -> String {
    static REWRITE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = REWRITE.get_or_init(|| {
        Regex::new(r"\(\?<([A-Za-z_][A-Za-z0-9_]*)>").expect("valid rewrite regex")
    });
    re.replace_all(pattern, "(?P<$1>").into_owned()
}

fn bi_splits(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "splits", 1)?;
    let p = as_str(&args[1], "splits", 2)?;
    let f = optional_flags(args, 2, "splits")?;
    let rx = compile_with_flags(&p, &f, "splits")?;
    let parts: Vec<Value> = rx
        .split(&s)
        .map(|piece| Value::Str(piece.to_string()))
        .collect();
    Ok(Value::List(parts))
}

// Non-regex string builtins

fn bi_index(args: &[Value]) -> Result<Value, QueryError> {
    let eq = |item: &Value, target: &Value| -> bool {
        let a = match item {
            Value::PathRef(p) => Value::Str(p.full_path.clone()),
            other => other.clone(),
        };
        let b = match target {
            Value::PathRef(p) => Value::Str(p.full_path.clone()),
            other => other.clone(),
        };
        value::py_eq(&a, &b, 0)
    };
    let needle = &args[1];
    match &args[0] {
        Value::Str(s) => {
            let n = as_str(needle, "index", 2)?;
            Ok(byte_to_char_index(s, &n))
        }
        Value::PathRef(p) => {
            let n = as_str(needle, "index", 2)?;
            Ok(byte_to_char_index(&p.full_path, &n))
        }
        Value::List(items) | Value::Stream(items) => {
            for (i, item) in items.iter().enumerate() {
                if eq(item, needle) {
                    return Ok(Value::Int(i as i64));
                }
            }
            Ok(Value::Null)
        }
        other => Err(QueryError::builtin(format!(
            "index: cannot search inside {}",
            type_name(other)
        ))),
    }
}

/// Returns a code-point index of the first match (`-1` → `null`).
fn byte_to_char_index(haystack: &str, needle: &str) -> Value {
    match haystack.find(needle) {
        Some(byte) => Value::Int(haystack[..byte].chars().count() as i64),
        None => Value::Null,
    }
}

fn bi_ltrimstr(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "ltrimstr", 1)?;
    let p = as_str(&args[1], "ltrimstr", 2)?;
    if s.starts_with(&p) {
        Ok(Value::Str(s[p.len()..].to_string()))
    } else {
        Ok(Value::Str(s))
    }
}

fn bi_rtrimstr(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "rtrimstr", 1)?;
    let p = as_str(&args[1], "rtrimstr", 2)?;
    if !p.is_empty() && s.ends_with(&p) {
        Ok(Value::Str(s[..s.len() - p.len()].to_string()))
    } else {
        Ok(Value::Str(s))
    }
}

fn bi_explode(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "explode", 1)?;
    Ok(Value::List(
        s.chars()
            .map(|ch| Value::Int(i64::from(ch as u32)))
            .collect(),
    ))
}

fn bi_implode(args: &[Value]) -> Result<Value, QueryError> {
    let items = as_sequence(&args[0], "implode", 1)?;
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        let n = match item {
            Value::Int(n) => *n,
            other => {
                return Err(QueryError::builtin(format!(
                    "implode: item {i} must be an integer codepoint, got {}",
                    type_name(other)
                )));
            }
        };
        if !(0..=0x10_FFFF).contains(&n) {
            return Err(QueryError::builtin(format!(
                "implode: item {i} = {n} is not a valid Unicode codepoint"
            )));
        }
        match char::from_u32(n as u32) {
            Some(ch) => out.push(ch),
            None => {
                return Err(QueryError::builtin(format!(
                    "implode: item {i} = {n} is not a valid Unicode codepoint"
                )));
            }
        }
    }
    Ok(Value::Str(out))
}

fn bi_ascii(args: &[Value]) -> Result<Value, QueryError> {
    let n = as_int(&args[0], "ascii", 1)?;
    if !(0..=0x10_FFFF).contains(&n) {
        return Err(QueryError::builtin(format!(
            "ascii: {n} is not a valid Unicode codepoint"
        )));
    }
    match char::from_u32(n as u32) {
        Some(ch) => Ok(Value::Str(ch.to_string())),
        None => Err(QueryError::builtin(format!(
            "ascii: {n} is not a valid Unicode codepoint"
        ))),
    }
}

fn bi_utf8bytelength(args: &[Value]) -> Result<Value, QueryError> {
    let s = as_str(&args[0], "utf8bytelength", 1)?;
    Ok(Value::Int(s.len() as i64))
}

fn bi_csv(args: &[Value]) -> Result<Value, QueryError> {
    let mut cells = Vec::with_capacity(args.len());
    for (i, c) in args.iter().enumerate() {
        cells.push(csv_quote(&csv_cell_text(c, "csv", i + 1)?));
    }
    Ok(Value::Str(cells.join(",")))
}

fn bi_tsv(args: &[Value]) -> Result<Value, QueryError> {
    let mut cells = Vec::with_capacity(args.len());
    for (i, c) in args.iter().enumerate() {
        cells.push(csv_cell_text(c, "tsv", i + 1)?);
    }
    Ok(Value::Str(cells.join("\t")))
}

/// Coerce one cell to its scalar string form,
/// flattening embedded tabs / newlines / carriage returns to spaces.
fn csv_cell_text(value: &Value, name: &str, arg: usize) -> Result<String, QueryError> {
    let s = match value {
        Value::Null => return Ok(String::new()),
        Value::Bool(b) => return Ok(if *b { "true" } else { "false" }.to_string()),
        Value::Int(i) => return Ok(i.to_string()),
        Value::Float(f) => return Ok(crate::jsonfmt::py_float_repr(*f)),
        Value::PathRef(p) => p.full_path.clone(),
        Value::Str(s) => s.clone(),
        Value::List(items) | Value::Stream(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(csv_cell_text(item, name, arg)?);
            }
            parts.join(" ")
        }
        other => {
            return Err(QueryError::builtin(format!(
                "{name}: argument {arg} cannot be rendered as a row cell ({})",
                type_name(other)
            )));
        }
    };
    Ok(s.replace(['\t', '\n', '\r'], " "))
}

/// RFC 4180 quoting of a cell.
fn csv_quote(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

/// Repr-style rendering of a string (single-quoted), used in `capture`'s no-match
/// error. The patterns we interpolate are ASCII without quotes.
fn py_str_repr(s: &str) -> String {
    if s.contains('\'') && !s.contains('"') {
        format!("\"{s}\"")
    } else {
        let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Value {
        Value::Str(text.to_owned())
    }

    fn call_str(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> String {
        match f(args) {
            Ok(Value::Str(text)) => text,
            other => panic!("expected Str, got {other:?}"),
        }
    }

    fn call_int(f: fn(&[Value]) -> Result<Value, QueryError>, args: &[Value]) -> i64 {
        match f(args) {
            Ok(Value::Int(n)) => n,
            other => panic!("expected Int, got {other:?}"),
        }
    }

    #[test]
    fn sub_and_gsub_accept_flags() {
        // `sub` replaces the first match, `gsub` all; the `"i"` flag is
        // case-insensitive.
        assert_eq!(
            call_str(bi_sub, &[s("ABC"), s("abc"), s("XYZ"), s("i")]),
            "XYZ"
        );
        assert_eq!(
            call_str(bi_gsub, &[s("aBcD"), s("[bd]"), s("X"), s("i")]),
            "aXcX"
        );
    }

    #[test]
    fn ascii_maps_codepoint_to_char() {
        // ::test_ascii_codepoint_to_char
        assert_eq!(call_str(bi_ascii, &[Value::Int(65)]), "A");
    }

    #[test]
    fn utf8bytelength_counts_bytes_not_codepoints() {
        // ::test_utf8bytelength_counts_bytes_not_codepoints — `é` is two
        // UTF-8 bytes, so "héllo" is 6 bytes over 5 codepoints.
        assert_eq!(call_int(bi_utf8bytelength, &[s("hello")]), 5);
        assert_eq!(call_int(bi_utf8bytelength, &[s("héllo")]), 6);
    }
}
