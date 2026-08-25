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

//! `fcopy` — copy data from one channel to another.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "fcopy inputChan outputChan ?-size size? ?-command callback?",
    ..FormSpec::DEFAULT
}];

/// `fcopy`'s two switches, present unchanged by name/shape in every Tcl
/// release from 8.4 through 9.1 — only their documented value semantics
/// shift across versions (see each `detail` below), so neither carries a
/// `dialects` restriction of its own.
///
/// `fcopy` itself is banned in F5 iRules (it is one of the 42 K36322151
/// commands — the TMM event sandbox has no general channel/filesystem
/// model), and that ban is expressed directly by its `dialects` group
/// below: `ALL_TCL` does not carry the `IRULES` bit, so the spec never
/// intersects the bare `IRULES` availability mask — the same treatment
/// `open` (also banned from the TMM sandbox) gets in `open_.rs`. Every
/// other modelled dialect (iApps, tmsh, Expect, the EDA vendor shells,
/// Tk) is covered by `ALL_TCL`, so `fcopy` resolves there normally.
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-size",
        value: OptionValue::value("size"),
        detail: "Maximum amount of data to transfer before stopping; without it, fcopy copies until end of file. Counted in bytes when the input channel is in binary mode, in characters otherwise (Tcl 9.0+); Tcl 8.6 counted bytes only when both channels shared an encoding, and Tcl 8.4/8.5 counted bytes unconditionally.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_command_prefix_n("callback", AppendedArity::AtLeast(1)),
        detail: "Run the copy in the background and return immediately; callback is invoked when the copy completes or fails, with the byte/character count appended and an error-message argument on failure. inputChan/outputChan are switched to non-blocking mode automatically; an active event loop (vwait or Tk) is required to drive the copy.",
        ..OptionSpec::DEFAULT
    },
];

/// Command spec for `fcopy`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fcopy",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Copy data from one channel to another, synchronously or in the background.",
            synopsis: &["fcopy inputChan outputChan ?-size size? ?-command callback?"],
            snippet: "Transfers data from inputChan to outputChan until -size bytes/characters have been copied or end of file is reached, whichever comes first; without -size, fcopy copies to end of file. Without -command, fcopy blocks until the copy finishes and returns the count written to outputChan. With -command, fcopy returns immediately and runs the copy in the background, invoking callback with the count (and an error message on failure) once it completes; if either channel is closed mid-copy, the copy stops silently and callback is never invoked. Only same-direction I/O is barred while a background copy runs — further reads on inputChan or writes on outputChan get a \"channel busy\" error — but a simultaneous fcopy in the opposite direction is fine (Tcl 8.6+, and is how a bidirectional copy pair is built); Tcl 8.4/8.5 instead barred all other I/O on either channel during a background copy. fcopy honors each channel's -translation and -encoding (and, from Tcl 9.0, -profile) configuration; the reported count is always outputChan's byte/character count, which can differ from the amount read from inputChan. From Tcl 9.0, fcopy can raise an encoding error (POSIX EILSEQ) when either channel uses the \"strict\" encoding profile.",
            source: "Tcl fcopy(n)",
            examples: "fcopy $in $out\nfcopy $in $out -size 4096\nfcopy $in $out -command [list onCopyDone $in $out]",
            return_value: "Synchronous: the number of bytes written to outputChan (or characters, under the same bytes/characters rule as -size, from Tcl 8.6 on — Tcl 8.4/8.5 always counted bytes). Asynchronous (-command given): fcopy returns immediately and the count is delivered to the callback instead.",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
