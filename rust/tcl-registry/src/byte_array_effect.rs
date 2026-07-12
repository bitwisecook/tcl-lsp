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

//! Byte-array corruption effect of a value-transforming command / subcommand.
//!
//! Drives the S110 byte-array-corruption check
//! (`tcl_compiler::shimmer::byte_array`). When a byte-array (binary) value —
//! e.g. `binary format`/`binary decode` output, `encoding convertto` output,
//! or an iRules `*::payload` getter — flows through one of these operations,
//! the effect says whether the result still holds the byte-array internal
//! representation (safe to write back to a byte sink) or has been coerced /
//! mangled into a character string (which a byte sink re-encodes, silently
//! corrupting every byte `>= 0x80`).
//!
//! This replaces the hardcoded `string`-subcommand and coercing-command name
//! lists that used to live in the compiler: the classification is registry
//! data on [`crate::CommandSpec`] (whole commands like `format` / `join`) and
//! [`crate::SubCommand`] (`string`'s subcommands), and the S110 pass is a
//! generic consumer that never names a command.

/// How an operation treats a byte-array (binary) operand it derives its result
/// from.
///
/// Default is [`ByteArrayEffect::None`] — the operation is not a byte-array
/// value-transform and the S110 pass ignores it — so existing specs don't need
/// touching when the field is added. Stamp a non-default effect only on a
/// command / subcommand whose result derives from a (possibly binary) value
/// operand.
///
/// The classifications are verified against tclsh 8.6 and 9.0 via
/// `tcl::unsupported::representation` and a round-trip through a
/// `-translation binary` sink.
// Deliberately *not* `#[non_exhaustive]`: every consumer (the S110 pass) must
// handle each effect explicitly, so a future variant should break the match
// rather than fall through a wildcard to the wrong behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ByteArrayEffect {
    /// Not a byte-array value-transform (the default): the S110 pass ignores
    /// it. Also correct for operations that read a byte array without deriving
    /// a new string from it (`string length` returns the byte count).
    #[default]
    None,
    /// The result keeps the operand's byte-array representation: the bytes are
    /// returned unchanged and are byte-exact at a byte sink. Provenance passes
    /// through unchanged (a binary operand stays binary; an
    /// already-damaged operand stays damaged).
    ///
    /// `string range` / `index` / `reverse` / `trim` / `trimleft` /
    /// `trimright` — each keeps the byte-array intrep in both tclsh 8.6 and
    /// 9.0. So `string range $payload 0 5` written back with `*::payload
    /// replace` is byte-exact and must **not** raise S110.
    Transparent,
    /// The result is a character string derived from the operand: Latin-1-
    /// preserving in isolation, but a byte sink re-encodes every byte `>= 0x80`
    /// (latin-1 decode → UTF-8 encode). Marks the value **damaged**.
    ///
    /// `string map` / `replace` / `insert` / `cat` / `repeat` (the
    /// concatenating / rewriting forms produce a fresh string), and the
    /// whole-command string builders `format` / `join` / `concat` / `split` /
    /// `subst` / `regsub`.
    Coerces,
    /// The result reinterprets the bytes as Unicode code points, mangling every
    /// byte `>= 0x80` **directly** — corrupt with or without a byte sink.
    /// Marks the value damaged and warns even when it is never written back.
    ///
    /// `string tolower` / `toupper` / `totitle` (verified: byte `200` → `232`).
    CaseFolds,
}

impl ByteArrayEffect {
    /// Whether the effect leaves the operand's byte-array representation intact
    /// (byte-exact at a byte sink).
    #[must_use]
    pub const fn is_transparent(self) -> bool {
        matches!(self, Self::Transparent)
    }

    /// Whether the effect turns a byte array into a character string (damaging
    /// it at a byte sink) — either by string-building ([`Self::Coerces`]) or by
    /// case-folding ([`Self::CaseFolds`]).
    #[must_use]
    pub const fn corrupts(self) -> bool {
        matches!(self, Self::Coerces | Self::CaseFolds)
    }

    /// Whether the effect corrupts the bytes directly, with or without a byte
    /// sink (case-folding) — the S110 pass warns on these even when the value
    /// is never written back.
    #[must_use]
    pub const fn corrupts_in_place(self) -> bool {
        matches!(self, Self::CaseFolds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandRegistry;

    #[test]
    fn string_subcommand_effects_are_classified() {
        let reg = CommandRegistry::build_default();
        let string = reg.get("string").expect("string command");
        let effect = |sub: &str| {
            string
                .resolve_subcommand(sub)
                .unwrap_or_else(|| panic!("no string subcommand {sub}"))
                .byte_array_effect
        };
        // Byte-array-transparent (verified vs tclsh 8.6 and 9.0).
        for sub in ["range", "index", "reverse", "trim", "trimleft", "trimright"] {
            assert_eq!(effect(sub), ByteArrayEffect::Transparent, "string {sub}");
        }
        // Coerce the byte array to a character string.
        for sub in ["map", "replace", "insert", "cat", "repeat"] {
            assert_eq!(effect(sub), ByteArrayEffect::Coerces, "string {sub}");
        }
        // Case-fold mangles the bytes directly.
        for sub in ["tolower", "toupper", "totitle"] {
            assert_eq!(effect(sub), ByteArrayEffect::CaseFolds, "string {sub}");
        }
        // `length` reads the byte count without deriving a new string.
        assert_eq!(effect("length"), ByteArrayEffect::None);
    }

    #[test]
    fn whole_command_coercers_are_classified() {
        let reg = CommandRegistry::build_default();
        for cmd in ["format", "subst", "regsub", "join", "concat", "split"] {
            assert_eq!(
                reg.get(cmd).unwrap().byte_array_effect,
                ByteArrayEffect::Coerces,
                "{cmd}",
            );
        }
    }

    #[test]
    fn effect_predicates() {
        assert!(ByteArrayEffect::Transparent.is_transparent());
        assert!(!ByteArrayEffect::Coerces.is_transparent());
        assert!(ByteArrayEffect::Coerces.corrupts());
        assert!(ByteArrayEffect::CaseFolds.corrupts());
        assert!(!ByteArrayEffect::None.corrupts());
        assert!(ByteArrayEffect::CaseFolds.corrupts_in_place());
        assert!(!ByteArrayEffect::Coerces.corrupts_in_place());
    }
}
