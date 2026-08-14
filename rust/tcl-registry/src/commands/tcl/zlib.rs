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

//! `zlib` — compression, decompression, and checksumming via the zlib
//! library (Tcl 8.6+, TIP 234).
//!
//! Absent before Tcl 8.6: zlib.html 404s on both the 8.4 and 8.5
//! tcl-lang.org doc trees, and `zlib` appears in neither version's
//! TclCmd/contents.htm command index — so the whole command is gated
//! `TCL86_PLUS`, not `ALL_TCL`. Tcl 8.6.18's, 9.0.4's, and 9.1b0's
//! zlib.n/zlib.htm are identical for every subcommand, option, default,
//! and error condition modelled below (including the STREAMING INSTANCE
//! COMMAND's `header` subcommand — present in the 8.6.18 body text even
//! though it is missing its own numbered `#M` anchor and so is absent
//! from the 8.6 page's auto-generated outline, unlike 9.0/9.1's, which
//! number it `#M62`; the content itself, not just the anchor, is what
//! matters for a `DialectSet` gate, and that content is present in all
//! three) — so no per-fact version split exists below beyond the
//! whole-command Tcl-8.6-floor gate.
//!
//! Re-verified directly against live tcl-lang.org fetches (2026-07-23):
//! 8.6.18/9.0.4/9.1b0's zlib pages diff byte-for-byte identical once tag
//! case (8.6's page predates the site's lowercase-HTML template) and the
//! doc-anchor line-number IDs are discounted — the *only* textual delta
//! anywhere in the three bodies is "compression and decompression
//! operations" (8.6 NAME line) vs "Compression and decompression
//! operations" (9.0/9.1, capitalised), confirming both the identical-
//! content claim above and the `stream header` `#M62`-anchor quirk.
//! Tcl 8.5's zlib.html was likewise re-fetched live and still serves the
//! same soft-404 "URL Not Found" page template `coroutine.rs` documents
//! for `coroutine.n` on the same 8.4/8.5 doc trees (a 200-status page
//! that says "The page you are looking for cannot be found", not a real
//! manual page).
//! Tcl 8.4's zlib.html could not be re-fetched this pass — 10+ attempts
//! (both it and a known-good control page from the same 8.4 doc tree,
//! `open.html`) all failed with either a bare connection timeout or a
//! Cloudflare 522, indicating a transient host-side outage of the 8.4
//! tree specifically rather than a real 404 — so the 8.4-absence
//! conclusion above rests on the prior audit's finding plus today's live
//! 8.5 reconfirmation and TIP 234 landing in 8.6 (years after 8.4's
//! feature-freeze), not on a fresh fetch of the 8.4 page itself.
//!
//! The manual documents each option's *value* precisely but not which
//! of the six `push`/`stream` modes (compress/decompress/deflate/gunzip/
//! gzip/inflate) actually accepts which option — e.g. it describes
//! `-dictionary` as "not valid for transformations that work with
//! gzip-format data" without saying whether that is a hard per-mode
//! rejection or a documented-but-silently-ignored case, and says nothing
//! about `-limit`/`-level`'s mode applicability at all. The per-mode
//! option gating on [`PUSH_OPTIONS`] and [`STREAM_OPTIONS`] below (and
//! the numeric ranges/defaults for `-limit` and `bufferSize`) were
//! cross-checked directly against `ZlibPushSubcmd`/`ZlibStreamSubcmd` in
//! `generic/tclZlib.c` on both the `core-8-6-16` and `core-9-0-4` tags
//! (identical option tables and error paths on both), since that is
//! real per-mode behaviour the prose alone under-specifies.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "zlib subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// `push`'s `mode` positional argument (index 0, after the `push` word) —
/// always a single flat word (`Tcl_GetIndexFromObj` against a flat string
/// table in `ZlibPushSubcmd`, never a list), so — unlike `open`'s
/// POSIX-flag-list access argument — this is safe to also list in
/// `closed_value_args`.
const PUSH_MODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "compress",
        detail: "Compressing transformation producing zlib-format data on channel, which must be writable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "decompress",
        detail: "Decompressing transformation reading zlib-format data from channel, which must be readable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "deflate",
        detail: "Compressing transformation producing raw compressed data (no zlib/gzip framing) on channel, which must be writable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "gunzip",
        detail: "Decompressing transformation reading gzip-format data from channel, which must be readable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "gzip",
        detail: "Compressing transformation producing gzip-format data on channel, which must be writable.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "inflate",
        detail: "Decompressing transformation reading raw compressed data (no zlib/gzip framing expected) from channel, which must be readable.",
        ..ArgValue::DEFAULT
    },
];

/// `stream`'s `mode` positional argument (index 0, after the `stream`
/// word) — same six words as [`PUSH_MODE_VALUES`], always a single flat
/// word. The accepted-option summary in each detail is the per-mode
/// `OptDescriptor` table `ZlibStreamSubcmd` actually dispatches through
/// (`compressionOpts`/`gzipOpts`/`expansionOpts`/`gunzipOpts`), not just
/// the manual's prose — see [`STREAM_OPTIONS`] for the option-level
/// detail.
const STREAM_MODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "compress",
        detail: "Compressing stream producing zlib-format output. Accepts -dictionary and -level.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "decompress",
        detail: "Decompressing stream taking zlib-format input and producing uncompressed output. Accepts -dictionary only.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "deflate",
        detail: "Compressing stream producing raw output; the data carries no metadata about which compression dictionary, if any, was used. Accepts -dictionary and -level.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "gunzip",
        detail: "Decompressing stream taking gzip-format input and producing uncompressed output. Accepts no options at all.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "gzip",
        detail: "Compressing stream producing gzip-format output. Accepts -header and -level (not -dictionary).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "inflate",
        detail: "Decompressing stream taking raw compressed input and producing uncompressed output; does not check that a supplied -dictionary is actually the correct one. Accepts -dictionary only.",
        ..ArgValue::DEFAULT
    },
];

/// `zlib gzip`'s options.
const GZIP_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-level",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Range(0, 9)),
            hint: "level",
            ..OptionArg::DEFAULT
        }),
        detail: "Compression level: 0 (uncompressed) through 9 (maximally compressed). Omitted, zlib's own default level is used (Z_DEFAULT_COMPRESSION, which the zlib library maps to level 6).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-header",
        value: OptionValue::value("dict"),
        detail: "Dict describing the gzip header to write. Recognised keys, all optional: comment (text); crc (boolean — leave unset when the data will be interchanged with the gzip program); filename; os (RFC 1952 operating-system code); time (as from clock seconds or file mtime); type (binary or text).",
        ..OptionSpec::DEFAULT
    },
];

/// `zlib gunzip`'s options.
const GUNZIP_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-headerVar",
    value: OptionValue::Takes(OptionArg {
        role: ArgRole::VarWrite,
        hint: "varName",
        ..OptionArg::DEFAULT
    }),
    detail: "Variable to receive a dict describing the gzip header that was read. Possible keys, each present only when known: comment; crc (boolean); filename; os (RFC 1952 code); size (the uncompressed data's size); time (suitable for clock format); type (binary or text).",
    ..OptionSpec::DEFAULT
}];

/// `zlib push`'s creation options. Per-mode gating (`ZlibPushSubcmd`,
/// `generic/tclZlib.c`, identical on the `core-8-6-16` and `core-9-0-4`
/// tags): `-dictionary`/`-header`/`-level` are syntactically accepted for
/// every mode (`pushCompressOptions`/`pushDecompressOptions` both list
/// them), but `-limit` is only *in* `pushDecompressOptions` — passing it
/// to a compress/deflate/gzip push is an unrecognised-option error, not a
/// silently-ignored one.
const PUSH_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::value("binData"),
        detail: "Compression dictionary: strings likely to recur later in the data to compress, with the most common ones preferably toward the end. Rejected outright (\"a compression dictionary may not be set in the gzip format\") on a gzip- or gunzip-mode push.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-header",
        value: OptionValue::value("dict"),
        detail: "Gzip header description, in the same dict format as zlib gzip's -header. Meaningful for a gzip-producing (mode gzip) push.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-level",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Range(0, 9)),
            hint: "level",
            ..OptionArg::DEFAULT
        }),
        detail: "Compression level 0-9; omitted, zlib's own default level is used. Meaningful only for a compressing push (compress/deflate/gzip) — syntactically accepted but without effect on a decompressing one.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-limit",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Range(1, 65536)),
            hint: "bytes",
            ..OptionArg::DEFAULT
        }),
        detail: "Maximum bytes to read ahead while decompressing; 1-65536 (default 4096). Accepted only by a decompressing push (decompress/gunzip/inflate) — rejected as an unrecognised option on a compressing one. No longer functionally significant: Tcl now returns any bytes it read past the end of the compressed stream back to the channel instead of withholding them, so tuning this rarely matters.",
        ..OptionSpec::DEFAULT
    },
];

/// `zlib stream`'s options — the union across modes; see each mode's
/// entry in [`STREAM_MODE_VALUES`] and each option's own `detail` for the
/// exact per-mode gating (`ZlibStreamSubcmd`'s `compressionOpts` /
/// `gzipOpts` / `expansionOpts` / `gunzipOpts` tables — unlike `push`,
/// stream's tables really do vary the accepted *option set* per mode, not
/// just accept-and-ignore).
const STREAM_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-dictionary",
        value: OptionValue::value("bindata"),
        detail: "Compression dictionary. Accepted only by the compress/decompress/deflate/inflate modes (an unrecognised-option error for gzip/gunzip); inflate does not check that a supplied dictionary is actually the correct one.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-level",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Range(0, 9)),
            hint: "level",
            ..OptionArg::DEFAULT
        }),
        detail: "Compression level 0-9; omitted, zlib's own default level is used. Accepted only by the modes that produce compressed output (compress/deflate/gzip) — an unrecognised-option error for decompress/inflate/gunzip.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-header",
        value: OptionValue::value("dict"),
        detail: "Gzip header description dict (same keys as zlib gzip's -header). Accepted only by the gzip mode.",
        ..OptionSpec::DEFAULT
    },
];

// Every subcommand below that returns compressed/decompressed binary
// data (compress/decompress/deflate/inflate/gzip/gunzip) is typed
// `TclType::String`, not `TclType::ByteArray`, even though the C
// implementation returns a genuine byte-array `Tcl_Obj` in every case
// (`Tcl_NewByteArrayObj`) and the manual says outright that "the zlib
// command always operates on binary strings". `ByteArray` is
// deliberately not used here: `byte_array_effect.rs`'s
// `s110_registry_sets_match_legacy_hardcoded_behaviour` test asserts an
// *exhaustive*, closed list of every `return_type == ByteArray` spec in
// the whole registry (`binary decode`, `binary format`, `encoding
// convertto` — the S110 pass's hardcoded source list), and
// this file is not the place to grow that list. `read`/`gets` (which can
// equally return binary-configured-channel bytes) already set the same
// precedent of staying `String` for the same reason.
static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "compress",
        arity: Arity::new(1, 2),
        detail: "Return the zlib-format compressed binary data of string. The optional level (0, uncompressed, to 9, maximally compressed) selects the compression effort; omitted, zlib's own default level is used.",
        synopsis: "zlib compress string ?level?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "decompress",
        arity: Arity::new(1, 2),
        detail: "Return the uncompressed version of the raw zlib-format compressed binary data in string. The optional bufferSize, when given, must be 16-65536 and hints the initial output buffer size; Tcl grows the buffer automatically if more space is needed.",
        synopsis: "zlib decompress string ?bufferSize?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "deflate",
        arity: Arity::new(1, 2),
        detail: "Return the raw compressed binary data of string (no zlib/gzip framing). The optional level (0-9) selects the compression effort; omitted, zlib's own default level is used.",
        synopsis: "zlib deflate string ?level?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "gunzip",
        // Not `pure`: -headerVar, when given, writes a variable — same
        // "declare the write unconditionally since the static spec can't
        // see whether a call actually passes the option" convention as
        // `encoding convertfrom`'s -failindex.
        arity: Arity::new(1, 3),
        detail: "Return the uncompressed contents of string, which must be gzip-format data.",
        synopsis: "zlib gunzip string ?-headerVar varName?",
        options: GUNZIP_OPTIONS,
        return_type: Some(TclType::String),
        // -headerVar's target receives the gzip-header dict (comment/crc/
        // filename/os/size/time/type), not this subcommand's own String
        // return value — the same "written variable has a fixed intrep
        // unrelated to the return value" shape as `gets chan line`
        // (`VarWriteTyping::Fixed(TclType::String)` in gets_.rs). Without
        // this override the type lattice defaults to `ReturnValue` and
        // would mistype the header variable as a String.
        var_write_typing: VarWriteTyping::Fixed(TclType::Dict),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "gzip",
        arity: Arity::new(1, 5),
        detail: "Return the compressed contents of string in gzip format.",
        synopsis: "zlib gzip string ?-level level? ?-header dict?",
        options: GZIP_OPTIONS,
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inflate",
        arity: Arity::new(1, 2),
        detail: "Return the uncompressed version of the raw compressed binary data in string (the inverse of deflate; no zlib/gzip framing expected). The optional bufferSize, when given, must be 16-65536 and hints the initial output buffer size.",
        synopsis: "zlib inflate string ?bufferSize?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "push",
        arity: Arity::new(2, 10),
        detail: "Push a compressing or decompressing transformation of the given mode onto channel; returns channel unchanged, so the call can be chained or used directly. Removed again with chan pop. Once pushed, chan configure gains extra transform-specific options on the channel: -checksum (read-only), -dictionary (read-write, same restriction as here), -flush (write-only, sync or full), -header (read-only, decompressing gzip transforms only), and -limit (read-write, decompressing transforms only).",
        synopsis: "zlib push mode channel ?options ...?",
        arg_roles: &[(1, ArgRole::Channel)],
        arg_values: &[(0, PUSH_MODE_VALUES)],
        closed_value_args: &[0],
        options: PUSH_OPTIONS,
        return_type: Some(TclType::Channel),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "stream",
        arity: Arity::new(1, 5),
        detail: "Create a streaming compression/decompression command of the given mode and return its name. The returned command is used by calling its put subcommand one or more times to load data in and its get subcommand one or more times to extract the transformed data; its full subcommand set is add, checksum, close, eof, finalize, flush, fullflush, get, header (gunzip streams only — returns the gzip header dict), put, and reset. put also takes options: -dictionary (sets the compression dictionary), and the mutually exclusive -finalize/-flush/-fullflush (mark the stream finished / flushable-for-recovery / flushable-and-restartable, each at an increasing performance cost).",
        synopsis: "zlib stream mode ?options?",
        arg_values: &[(0, STREAM_MODE_VALUES)],
        closed_value_args: &[0],
        options: STREAM_OPTIONS,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "adler32",
        arity: Arity::new(1, 2),
        detail: "Compute a checksum of binary string string using the Adler-32 algorithm. The optional initValue seeds the checksum engine; omitted, the algorithm's own starting value (1) is used.",
        synopsis: "zlib adler32 string ?initValue?",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "crc32",
        arity: Arity::new(1, 2),
        detail: "Compute a checksum of binary string string using the CRC-32 algorithm. The optional initValue seeds the checksum engine; omitted, the algorithm's own starting value (0) is used.",
        synopsis: "zlib crc32 string ?initValue?",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `zlib`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "zlib",
        // See the module doc comment: no zlib.n page and no `zlib` TOC
        // entry on either the 8.4 or 8.5 tcl-lang.org doc trees, present
        // and textually identical (bar the 8.6-only `stream header`
        // outline-anchor quirk, not a content difference) across
        // 8.6.18/9.0.4/9.1b0.
        dialects: Some(DialectSet::TCL86_PLUS),
        // A real core-builtin ensemble command (registered by `Tcl_CreateObjCommand`,
        // not a library proc), so the minifier must not alias its head —
        // same reasoning `chan`/`encoding` already carry this trait for.
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        return_type: Some(TclType::String),
        // Command-level fallback only — every subcommand above declares
        // its own, more precise effect (or is `pure` and so never
        // reaches this): mirrors `encoding`'s top-level `Unknown` r/w
        // fallback for the same reason (a genuinely mixed-effect
        // ensemble, unlike `chan`'s uniformly-FileIo one).
        side_effects: &[SideEffect {
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Compress, decompress, and checksum data using zlib; also attaches a compression transform to a channel.",
            synopsis: &[
                "zlib compress string ?level?",
                "zlib decompress string ?bufferSize?",
                "zlib deflate string ?level?",
                "zlib gunzip string ?-headerVar varName?",
                "zlib gzip string ?-level level? ?-header dict?",
                "zlib inflate string ?bufferSize?",
                "zlib push mode channel ?options ...?",
                "zlib stream mode ?options?",
                "zlib adler32 string ?initValue?",
                "zlib crc32 string ?initValue?",
            ],
            snippet: "Tcl 8.6+ (TIP 234); not present in 8.4 or 8.5. compress/decompress use zlib-format framing; deflate/inflate use raw (unframed) data; gzip/gunzip use gzip framing (with an optional header dict). push stacks a compressing or decompressing transform onto an already-open channel, in the style of chan push; stream creates a standalone streaming compressor/decompressor command instead. adler32/crc32 compute the matching checksum. The command always operates on binary strings: encode a Tcl string with encoding convertto before compressing it, and decode the result with encoding convertfrom after decompressing. Not yet implemented in this project's WASM runtime — traps with ``unsupported command: zlib``.",
            source: "Tcl zlib(n)",
            examples: "set binData [encoding convertto utf-8 $string]\nset compData [zlib compress $binData]\n# ...\nset binData [zlib decompress $compData]\nset string [encoding convertfrom utf-8 $binData]\n\n# Streaming, for data arriving in stages.\nset strm [zlib stream compress]\n$strm put [encoding convertto utf-8 $string]\n$strm finalize\nset compData [$strm get]\n$strm close\n\n# Compress everything written to a file as it is written.\nset chan [open compressed.gz w]\nzlib push gzip $chan\nputs $chan $string\nclose $chan",
            return_value: "Depends on the subcommand: compress/decompress/deflate/inflate/gzip/gunzip return a binary string; push returns channel unchanged; stream returns the name of a new streaming command; adler32/crc32 return an integer checksum.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
