// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly labels of complete resident bytes, independent of source spelling.

/// Label a counted native name without replacing or dropping invalid bytes.
/// Backslashes and control characters are escaped, so distinct original bytes
/// remain distinct even when an ordinary name contains a literal escape.
/// The label grants no source spelling, lookup, comparison or edit authority.
#[must_use]
pub fn resident_name_label(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut label = String::new();
    for chunk in bytes.utf8_chunks() {
        for character in chunk.valid().chars() {
            match character {
                '\\' => label.push_str("\\\\"),
                character if character.is_control() => {
                    // Writing to String is infallible.
                    let _ = write!(label, "\\u{{{:x}}}", u32::from(character));
                }
                character => label.push(character),
            }
        }
        for byte in chunk.invalid() {
            let _ = write!(label, "\\x{byte:02x}");
        }
    }
    label
}

#[cfg(test)]
mod tests {
    use super::resident_name_label;

    #[test]
    fn labels_preserve_counted_bytes_without_claiming_source_roundtrips() {
        // Implementation contract: naming.presentation.resident-name-label
        // docs/design/analysis/name-resolution-proofs/resident-name-label.md
        assert_eq!(resident_name_label(b"greet"), "greet");
        assert_eq!(resident_name_label("café".as_bytes()), "café");
        let names: &[&[u8]] = &[
            b"p\xed\xa0\x80",
            b"p\xed\xa0\x81",
            b"p\xff",
            b"p\\xff",
            b"p\0tail",
            b"p\xc0\x80tail",
            b"p\\u{0}tail",
        ];
        let labels = names
            .iter()
            .map(|bytes| resident_name_label(bytes))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(labels.len(), names.len());
        assert_eq!(resident_name_label(b"p\0tail"), "p\\u{0}tail");
    }
}
