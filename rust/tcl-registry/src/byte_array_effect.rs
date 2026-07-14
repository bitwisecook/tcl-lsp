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

//! Byte-array corruption vocabulary for the S110 check.
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
//! The registry carries the *whole* S110 command model, so the pass never
//! matches a command by name:
//!
//! - **Transforms** — [`ByteArrayEffect`] on [`crate::CommandSpec`] (whole
//!   commands like `format` / `join` / `append`) and [`crate::SubCommand`]
//!   (`string`'s subcommands).
//! - **Sources** — a command / subcommand whose
//!   `return_type == Some(TclType::ByteArray)` returns a byte array
//!   (`binary format`, `binary decode`, `encoding convertto`).
//! - **Encoders** — [`ByteArrayEffect::Encodes`]: a byte source whose
//!   character operand, when *already* binary, is double-encoded
//!   (`encoding convertto`).
//! - **Re-binarifiers** — [`ByteArrayEffect::Rebinarifies`]: the operation
//!   reads a value operand as bytes and installs the byte-array rep on it in
//!   place (`binary scan`, `binary encode`) — the documented S110 fix.
//! - **Payload getters / sinks** — [`crate::BytePayloadSpec`] on the iRules
//!   `*::payload` commands; the getter/sink word classification lives on
//!   [`BytePayloadSpec::NON_GETTER_SUBS`] / [`BytePayloadSpec::replace_data_arg`]
//!   below.

use crate::spec::BytePayloadSpec;

/// How an operation treats a byte-array (binary) operand it derives its result
/// from.
///
/// Default is [`ByteArrayEffect::None`] — the operation is not a byte-array
/// value-transform and the S110 pass ignores it — so existing specs don't need
/// touching when the field is added. Stamp a non-default effect only on a
/// command / subcommand whose result derives from a (possibly binary) value
/// operand, or that installs a byte-array rep on an operand in place.
///
/// The classifications are verified against tclsh 8.6.14 via
/// `tcl::unsupported::representation` probes (both the interpreted command
/// procs and the byte-compiled `INST_STR_*` paths) and against the C sources
/// of Tcl 8.6.14 and 9.0.1 (`tclStringObj.c`, `tclCmdMZ.c`, `tclBinary.c` —
/// the latter reworked by TIP 568 in 9.0: `Tcl_GetBytesFromObj` demands a
/// *proper* byte sequence and errors on characters `> 0xFF`, where 8.6's
/// `SetByteArrayFromAny` silently truncated them via `UCHAR(ch)`).
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
    /// `string range` / `index` / `reverse` — each keeps a *pure* byte-array
    /// intrep in both 8.6 and 9.0 (tclsh 8.6.14-verified; 9.0.1
    /// `tclStringObj.c`: `Tcl_GetRange`, `StringIndexCmd`, and
    /// `TclStringReverse` all take a `TclIsPureByteArray` fast path returning
    /// `Tcl_NewByteArrayObj`). So `string range $payload 0 5` written back
    /// with `*::payload replace` is byte-exact and must **not** raise S110.
    Transparent,
    /// The result is a character string derived from the operand: Latin-1-
    /// preserving in isolation, but a byte sink re-encodes every byte `>= 0x80`
    /// (latin-1 decode → UTF-8 encode). Marks the value **damaged**.
    ///
    /// `string map` / `replace` / `insert` / `cat` / `repeat` / `trim` /
    /// `trimleft` / `trimright` (see each subcommand's doc comment in
    /// `commands/tcl/string_.rs` for the per-version evidence and the
    /// operand-conditional exceptions), the read-modify-write builder
    /// `append`, and the whole-command string builders `format` / `join` /
    /// `concat` / `split` / `subst` / `regsub`.
    Coerces,
    /// The result reinterprets the bytes as Unicode code points, mangling every
    /// byte `>= 0x80` **directly** — corrupt with or without a byte sink.
    /// Marks the value damaged and warns even when it is never written back.
    ///
    /// `string tolower` / `toupper` / `totitle` (tclsh 8.6.14-verified: byte
    /// `200` → `232`; both versions case-fold via the Unicode tables in
    /// `tclCmdMZ.c` `StringLowerCmd` et al.).
    CaseFolds,
    /// Encodes its character-string operand into a **new byte array** (the
    /// result is a byte source — its `return_type` is `TclType::ByteArray`).
    /// On an operand that is *already* binary, this is the double-encode bug:
    /// the bytes are reinterpreted as characters and re-encoded, so every byte
    /// `>= 0x80` becomes a multi-byte sequence (tclsh 8.6.14-verified:
    /// `encoding convertto utf-8` on byte `200` yields bytes `C3 88`).
    ///
    /// `encoding convertto`. The reverse direction, `encoding convertfrom`,
    /// is the legitimate decode and stays [`ByteArrayEffect::None`].
    Encodes,
    /// Reads a value operand **as bytes**, installing the byte-array internal
    /// rep on that operand *in place* — the documented S110 fix (F5
    /// K22406348). `value_arg` is the subcommand-relative index (0-based,
    /// after the subcommand word) of the re-binarified operand.
    ///
    /// `binary scan` (`value_arg: 0`) and `binary encode` (`value_arg: 1`).
    /// In 8.6 the conversion (`Tcl_GetByteArrayFromObj` →
    /// `SetByteArrayFromAny`, `tclBinary.c`) silently truncates characters
    /// `> 0xFF` via `UCHAR(ch)`; in 9.0 (TIP 568) `Tcl_GetBytesFromObj`
    /// errors — "expected byte sequence but character …" — and only installs
    /// the rep for a proper byte sequence. Damaged values tracked by S110
    /// hold only characters `<= 0xFF`, so the fix works in both versions.
    Rebinarifies {
        /// Subcommand-relative index of the operand whose byte-array rep is
        /// (re-)installed in place.
        value_arg: u8,
    },
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
    /// case-folding ([`Self::CaseFolds`]). [`Self::Encodes`] is deliberately
    /// excluded: its result is a byte array (a *source*); only a
    /// binary-operand encode is the double-encode corruption, which the S110
    /// pass reports separately.
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

/// The `*::payload` getter/sink word model consumed by the S110 pass.
///
/// [`BytePayloadSpec`] (declared in `spec.rs`, stamped on each
/// `<proto>::payload` command via `CommandSpec::byte_array_payload`) describes
/// the `replace` sink layout; the associated data here classifies the call
/// *forms* — which subcommand words make a call a non-getter, and where the
/// sink's `<data>` operand sits — so the compiler pass carries no payload
/// subcommand words of its own.
impl BytePayloadSpec {
    /// Non-getter `*::payload` subcommand words: these forms do not return raw
    /// payload bytes, so they are not binary sources. Every other first word
    /// (none, a `<size>`/`<offset>` number, …) is the getter form.
    ///
    /// Uniform across the F5 payload commands (`TCP::` / `UDP::` / `HTTP::` /
    /// `SCTP::` / `GTP::` / `MQTT::` / `DIAMETER::payload`): `replace` is the
    /// byte sink, `length` returns a count, and `rechunk` / `unchunk`
    /// (`HTTP::payload`) return nothing.
    pub const NON_GETTER_SUBS: &[&str] = &["replace", "length", "rechunk", "unchunk"];

    /// The byte-sink subcommand word — `<cmd> replace … <data>` writes bytes
    /// back to the wire.
    pub const REPLACE_SUB: &str = "replace";

    /// `GTP::payload`'s optional leading flag: `replace -message <value> …`
    /// shifts every positional operand by two (see
    /// [`BytePayloadSpec::message_flag_shift`]).
    pub const MESSAGE_FLAG: &str = "-message";

    /// True when a `*::payload` call with `first_arg` as its first word is the
    /// getter form (returns raw payload bytes — a binary source).
    #[must_use]
    pub fn is_getter_call(first_arg: Option<&str>) -> bool {
        first_arg.is_none_or(|sub| !Self::NON_GETTER_SUBS.contains(&sub))
    }

    /// For a `<cmd> replace … <data>` sink call, the index (within `args`,
    /// which exclude the command name) of the `<data>` operand; `None` when
    /// the call is not a complete `replace` sink form. Applies this layout's
    /// [`BytePayloadSpec::replace_data_index`] and the
    /// [`Self::MESSAGE_FLAG`] shift, so each protocol's layout is correct
    /// without special-casing in the consumer.
    #[must_use]
    pub fn replace_data_arg<S: AsRef<str>>(&self, args: &[S]) -> Option<usize> {
        if args.first()?.as_ref() != Self::REPLACE_SUB {
            return None;
        }
        let mut idx = usize::from(self.replace_data_index);
        if self.message_flag_shift
            && args
                .get(1)
                .is_some_and(|a| a.as_ref() == Self::MESSAGE_FLAG)
        {
            idx += 2;
        }
        (args.len() > idx).then_some(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandRegistry, TclType};

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
        // Byte-array-transparent (verified vs tclsh 8.6.14 and the 9.0.1
        // sources).
        for sub in ["range", "index", "reverse"] {
            assert_eq!(effect(sub), ByteArrayEffect::Transparent, "string {sub}");
        }
        // Coerce the byte array to a character string. `trim`/`trimleft`/
        // `trimright` were reclassified from `Transparent`: whenever they
        // actually trim they build a fresh string in both 8.6 and 9.0
        // (`Tcl_NewStringObj` in `StringTrimCmd` / `INST_STR_TRIM`).
        for sub in [
            "map",
            "replace",
            "insert",
            "cat",
            "repeat",
            "trim",
            "trimleft",
            "trimright",
        ] {
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
        for cmd in [
            "format", "subst", "regsub", "join", "concat", "split", "append",
        ] {
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
        assert!(!ByteArrayEffect::Encodes.corrupts());
        assert!(!ByteArrayEffect::Rebinarifies { value_arg: 0 }.corrupts());
        assert!(ByteArrayEffect::CaseFolds.corrupts_in_place());
        assert!(!ByteArrayEffect::Coerces.corrupts_in_place());
    }

    /// Drift guard: the registry-derived S110 source / encoder / re-binarifier
    /// / append-style sets must match the behaviour the compiler pass used to
    /// hardcode by name. Scans **every** spec and subcommand in the default
    /// registry so a stray stamping elsewhere fails here, not in a shimmer
    /// regression.
    #[test]
    fn s110_registry_sets_match_legacy_hardcoded_behaviour() {
        let reg = CommandRegistry::build_default();
        let mut sources: Vec<String> = Vec::new();
        let mut encoders: Vec<String> = Vec::new();
        let mut rebinarifiers: Vec<(String, u8)> = Vec::new();
        let mut append_style: Vec<&str> = Vec::new();
        for name in reg.command_names() {
            for spec in reg.specs(name) {
                if spec.return_type == Some(TclType::ByteArray) {
                    sources.push(name.to_owned());
                }
                if spec.byte_array_effect == ByteArrayEffect::Encodes {
                    encoders.push(name.to_owned());
                }
                if let ByteArrayEffect::Rebinarifies { value_arg } = spec.byte_array_effect {
                    rebinarifiers.push((name.to_owned(), value_arg));
                }
                // The pass's generic `append` rule: a read-modify-write
                // variable target with a corrupting command-level effect.
                if spec.byte_array_effect.corrupts()
                    && spec.traits.contains(crate::Traits::READS_BEFORE_WRITE)
                    && spec.assigns_variable_at.is_some()
                {
                    append_style.push(spec.name);
                }
                for sub in spec.subcommands {
                    if sub.return_type == Some(TclType::ByteArray) {
                        sources.push(format!("{name} {}", sub.name));
                    }
                    if sub.byte_array_effect == ByteArrayEffect::Encodes {
                        encoders.push(format!("{name} {}", sub.name));
                    }
                    if let ByteArrayEffect::Rebinarifies { value_arg } = sub.byte_array_effect {
                        rebinarifiers.push((format!("{name} {}", sub.name), value_arg));
                    }
                }
            }
        }
        sources.sort_unstable();
        sources.dedup();
        encoders.sort_unstable();
        rebinarifiers.sort_unstable();
        append_style.sort_unstable();
        append_style.dedup();
        // The legacy pass hardcoded exactly these binary sources
        // (`binary format` / `binary decode` / `encoding convertto`).
        assert_eq!(
            sources,
            ["binary decode", "binary format", "encoding convertto"],
            "S110 byte-array sources drifted",
        );
        // … special-cased exactly `encoding convertto` as the double-encoder …
        assert_eq!(encoders, ["encoding convertto"], "S110 encoders drifted");
        // … re-binarified exactly `binary scan $v` (value at sub-relative 0);
        // `binary encode <fmt> <data>` (value at sub-relative 1) carries the
        // same in-place conversion in C Tcl and is stamped alongside.
        assert_eq!(
            rebinarifiers,
            [
                ("binary encode".to_owned(), 1),
                ("binary scan".to_owned(), 0)
            ],
            "S110 re-binarifiers drifted",
        );
        // … and hardcoded exactly `append` as the variable-target coercer.
        assert_eq!(append_style, ["append"], "S110 append-style set drifted");
    }

    /// Drift guard: the payload getter / sink word model matches the pass's
    /// old hardcoded `PAYLOAD_NON_GETTER_SUBS` / `replace` / `-message` logic.
    #[test]
    fn payload_word_model_matches_legacy_hardcoded_behaviour() {
        assert_eq!(
            BytePayloadSpec::NON_GETTER_SUBS,
            ["replace", "length", "rechunk", "unchunk"],
        );
        // Getter classification: bare call and size arg are getters, the
        // non-getter words are not.
        assert!(BytePayloadSpec::is_getter_call(None));
        assert!(BytePayloadSpec::is_getter_call(Some("100")));
        for sub in BytePayloadSpec::NON_GETTER_SUBS {
            assert!(!BytePayloadSpec::is_getter_call(Some(sub)), "{sub}");
        }

        // Default layout (TCP/HTTP/…): `replace <offset> <length> <data>`.
        let common = BytePayloadSpec::DEFAULT;
        assert_eq!(
            common.replace_data_arg(&["replace", "0", "100", "$q"]),
            Some(3),
        );
        assert_eq!(common.replace_data_arg(&["replace", "0", "100"]), None);
        assert_eq!(common.replace_data_arg(&["length"]), None);
        let empty: &[&str] = &[];
        assert_eq!(common.replace_data_arg(empty), None);

        // MQTT/DIAMETER layout: `replace <data> …`.
        let mqtt = BytePayloadSpec {
            replace_data_index: 1,
            ..BytePayloadSpec::DEFAULT
        };
        assert_eq!(mqtt.replace_data_arg(&["replace", "$bad"]), Some(1));

        // GTP layout: an optional leading `-message <value>` shifts the
        // positional operands by two.
        let gtp = BytePayloadSpec {
            message_flag_shift: true,
            ..BytePayloadSpec::DEFAULT
        };
        assert_eq!(
            gtp.replace_data_arg(&["replace", "-message", "$m", "0", "100", "$q"]),
            Some(5),
        );
        assert_eq!(
            gtp.replace_data_arg(&["replace", "0", "100", "$q"]),
            Some(3),
        );
    }
}
