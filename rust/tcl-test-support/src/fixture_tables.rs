// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lossless, keyed text fixtures shared by conformance consumers.

/// Decode one complete original text image, including its final newline.
/// Keys retain the original relative file spelling. Missing/duplicate keys,
/// malformed records, invalid hex and non-UTF-8 text are fixture failures.
#[must_use]
pub fn text(table: &str, key: &str) -> String {
    String::from_utf8(bytes(table, key)).unwrap_or_else(|_| panic!("fixture text must be UTF-8"))
}

/// Decode one complete original byte image without imposing UTF-8.
/// Original family/release/path keys remain separate observation channels.
#[must_use]
pub fn bytes(table: &str, key: &str) -> Vec<u8> {
    let mut selected = table
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| line.split_once('\t').expect("fixture record delimiter"))
        .filter(|(path, _)| *path == key);
    let (_, hex) = selected.next().expect("fixture key missing");
    assert!(selected.next().is_none(), "duplicate fixture key");
    crate::encoding_utf8::decode_hex(hex)
}

#[cfg(test)]
mod tests {
    #[test]
    fn retains_empty_newline_and_na_bytes() {
        let table = "empty\t\nlf\t410a\ncrlf\t410d0a\nna\t4e4f545f4150504c494341424c450a\n";
        assert_eq!(super::text(table, "empty"), "");
        assert_eq!(super::text(table, "lf"), "A\n");
        assert_eq!(super::text(table, "crlf"), "A\r\n");
        assert_eq!(super::text(table, "na"), "NOT_APPLICABLE\n");
    }

    #[test]
    #[should_panic(expected = "fixture key missing")]
    fn missing_key_refuses() {
        let _ = super::text("other\t41\n", "absent");
    }

    #[test]
    #[should_panic(expected = "duplicate fixture key")]
    fn duplicate_key_refuses() {
        let _ = super::text("same\t41\nsame\t42\n", "same");
    }

    #[test]
    #[should_panic(expected = "fixture record delimiter")]
    fn malformed_record_refuses() {
        let _ = super::text("broken\n", "absent");
    }
    #[test]
    fn byte_channel_retains_nul_invalid_utf8_and_newline() {
        assert_eq!(super::bytes("raw\t00ff0a\n", "raw"), [0, 255, 10]);
    }

    #[test]
    #[should_panic(expected = "fixture text must be UTF-8")]
    fn text_channel_refuses_invalid_utf8() {
        let _ = super::text("raw\tff\n", "raw");
    }
}
