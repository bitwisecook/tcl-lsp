// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-language literal data for the existing editor-template publishers.
//! No command, registration identity or executable-name authority is inferred.

use anyhow::{Result, bail};
use serde::Serialize;
use serde_json::ser::{CharEscape, CompactFormatter, Formatter};
use std::io::{self, Write};

#[derive(Clone, Copy)]
enum QuotedTarget {
    Kotlin,
    Lua,
}

struct TargetFormatter(QuotedTarget);

impl Formatter for TargetFormatter {
    fn write_string_fragment<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        fragment: &str,
    ) -> io::Result<()> {
        let (delimiter, escaped) = match self.0 {
            QuotedTarget::Kotlin => ('$', b"\\$".as_slice()),
            QuotedTarget::Lua => ('\'', b"\\'".as_slice()),
        };
        let mut pieces = fragment.split(delimiter);
        writer.write_all(pieces.next().unwrap_or_default().as_bytes())?;
        for piece in pieces {
            writer.write_all(escaped)?;
            writer.write_all(piece.as_bytes())?;
        }
        Ok(())
    }

    fn begin_string<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        match self.0 {
            QuotedTarget::Kotlin => writer.write_all(b"\""),
            QuotedTarget::Lua => writer.write_all(b"'"),
        }
    }

    fn end_string<W: ?Sized + Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.begin_string(writer)
    }

    fn write_char_escape<W: ?Sized + Write>(
        &mut self,
        writer: &mut W,
        escape: CharEscape,
    ) -> io::Result<()> {
        match (self.0, escape) {
            (QuotedTarget::Kotlin, CharEscape::FormFeed) => writer.write_all(b"\\u000c"),
            (QuotedTarget::Lua, CharEscape::Quote) => writer.write_all(b"\""),
            (QuotedTarget::Lua, CharEscape::AsciiControl(byte)) => {
                write!(writer, "\\{byte:03}")
            }
            (_, escape) => CompactFormatter.write_char_escape(writer, escape),
        }
    }
}

fn quoted(value: &str, target: QuotedTarget) -> Result<String> {
    let mut bytes = Vec::new();
    let mut serializer =
        serde_json::Serializer::with_formatter(&mut bytes, TargetFormatter(target));
    value.serialize(&mut serializer)?;
    Ok(String::from_utf8(bytes)?)
}

/// A complete Kotlin ordinary string; dollar interpolation is always inert.
pub(crate) fn kotlin_string(value: &str) -> Result<String> {
    quoted(value, QuotedTarget::Kotlin)
}

/// A complete Lua ordinary string. JSON's otherwise unsupported Unicode
/// control escapes are projected as exactly three decimal escape digits.
pub(crate) fn lua_string(value: &str) -> Result<String> {
    quoted(value, QuotedTarget::Lua)
}

/// JSON's complete quoted Unicode string is also a Python ordinary string:
/// the serializer emits no JSON-only slash escape or non-string token.
pub(crate) fn python_string(value: &str) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

/// JSON's quoted string escapes are accepted by Emacs Lisp strings. This
/// constructor supplies the containing string only, never regexp quoting.
pub(crate) fn elisp_string(value: &str) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

/// A complete TOML string produced by the actual TOML value serializer.
pub(crate) fn toml_string(value: &str) -> Result<String> {
    let mut output = String::new();
    value.serialize(toml::ser::ValueSerializer::new(&mut output))?;
    Ok(output)
}

/// A complete TOML array of literal strings, preserving input order/values.
pub(crate) fn toml_array(values: &[String]) -> Result<String> {
    let mut output = String::new();
    values.serialize(toml::ser::ValueSerializer::new(&mut output))?;
    Ok(output)
}

/// XML 1.0 attribute data for the `JetBrains` semicolon extension-list target.
/// A value containing the target's delimiter cannot denote one extension.
pub(crate) fn xml_extension_list(values: &[String]) -> Result<String> {
    let mut out = String::new();
    for (index, value) in values.iter().enumerate() {
        if value.contains(';') {
            bail!("an editor extension contains the XML extension-list delimiter");
        }
        if index != 0 {
            out.push(';');
        }
        for ch in value.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '"' => out.push_str("&quot;"),
                '\t' => out.push_str("&#9;"),
                '\n' => out.push_str("&#10;"),
                '\r' => out.push_str("&#13;"),
                '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}' => {
                    out.push(ch);
                }
                _ => bail!("an editor extension is not XML 1.0 attribute data"),
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_literals_preserve_interpolation_controls_and_delimiters() {
        assert_eq!(
            kotlin_string("$x\u{c}\\\"").unwrap(),
            "\"\\$x\\u000c\\\\\\\"\""
        );
        assert_eq!(lua_string("x\0a").unwrap(), "'x\\000a'");
        assert_eq!(lua_string("'\"").unwrap(), "'\\'\"'");
        assert_eq!(kotlin_string("\\f").unwrap(), "\"\\\\f\"");
        assert_eq!(python_string("é\n\"").unwrap(), "\"é\\n\\\"\"");
        assert_eq!(elisp_string("\\.x").unwrap(), "\"\\\\.x\"");
        let value = "quote\"\\\n\u{7f}🙂";
        let text = format!("value = {}", toml_string(value).unwrap());
        let parsed: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(parsed["value"].as_str(), Some(value));
        assert_eq!(
            xml_extension_list(&["a&\"<\n".into()]).unwrap(),
            "a&amp;&quot;&lt;&#10;"
        );
        assert!(xml_extension_list(&["a;b".into()]).is_err());
        assert!(xml_extension_list(&["a\0b".into()]).is_err());
    }
}
