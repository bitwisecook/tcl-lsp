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

//! `tell` — return current access position for an open channel.

use crate::prelude::*;

// Tcl 8.4's tell.n SYNOPSIS/DESCRIPTION ("tell channelId": an integer
// byte offset that can be passed to seek, -1 for a channel that does not
// support seeking, and the valid channelId sources — a standard channel,
// an open/socket result, or an extension-created channel) are
// byte-for-byte identical in 8.5 and 8.6. From Tcl 9.0, tell.n itself
// shrinks to a two-line stub — "The tell command has been superceded by
// the chan tell command which supports the same syntax and options" —
// and its SYNOPSIS line renames the placeholder from `channelId` to
// `channel`; 9.1's page is otherwise byte-for-byte identical to 9.0's
// (both drop the EXAMPLE/KEYWORDS sections 8.4-8.6 carried). That is a
// documentation consolidation, not a behavioural or arity change: chan.n's
// own "chan tell" entry documents exactly the same
// byte-offset/-1-for-non-seekable behaviour, word for word identical
// across 8.5 (when chan was introduced), 8.6, 9.0, and 9.1, and plain
// `tell` keeps working exactly as described below in every version
// through 9.1.
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tell channelId",
    dialects: None,
}];

/// Command spec for `tell`.
///
/// Arity, the integer/byte-offset return, the -1-for-non-seekable-channel
/// case, and the valid channelId sources are unchanged across Tcl 8.4-9.1
/// — the only version-sensitive fact is that tell.n's own manual page
/// became a stub pointing at chan.n from Tcl 9.0 onward (see the `FORMS`
/// doc comment above), which is prose for the hover snippet, not a
/// dialect or arity gate.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tell",
        // Universal Tcl core command (`dialects: None`). Its absence from
        // F5 iRules — the sandboxed TMM interpreter has no real
        // filesystem/channel-seek support — is modelled as a subtractive
        // `IRULES_DISABLED_COMMANDS` entry in `tcl-dialect/src/profile.rs`
        // (confirmed: `"tell"` is on that list), never as a restriction on
        // this field — the established pattern for this whole class of
        // command (`open`/`seek`, also on the same disabled list, keep
        // `dialects: None` for the identical reason, and the
        // `irules_disable_list_is_load_bearing` contract test in
        // `tcl-registry/tests/dialect_profile.rs` requires the bare
        // `IRULES` mask to still admit this spec so the disable list has
        // something to subtract). No other modelled dialect (iApps, tmsh,
        // the EDA vendor shells, Tk, Expect, incr Tcl) disables or alters
        // `tell`.
        dialects: None,
        // Reads channel-table state that lives outside the argument list
        // — the access position can change between two calls if
        // something else reads from or seeks the channel meanwhile —
        // rather than being a pure function of its own arguments, so
        // `Traits::PURE` stays off `tell` for the same reason it stays
        // off its FileIo-reading siblings `close`/`eof`/`fblocked`/
        // `seek`/`pid` (see `pid.rs`).
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            // Query-only: reads the channel's current position but never
            // moves it — contrast `seek`, which is `writes: true` for the
            // identical FileIo target.
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Return the current access position for an open channel.",
            synopsis: &["tell channelId"],
            snippet: "Returns an integer string giving the current access position in channelId, as a byte offset — not a character offset, unlike read — that can be passed to seek to move the channel back to that position. The value is -1 for a channel whose underlying file or device does not support seeking, such as most pipes and sockets; unlike seek, which raises an error when asked to reposition a non-seekable channel, tell simply reports the sentinel -1 instead. channelId must already be open: a Tcl standard channel (stdin, stdout, stderr), the result of open or socket, or a channel created by a Tcl extension.\n\nTcl 8.5 introduced chan tell, an equivalent command reached through the chan ensemble; from Tcl 9.0 the tell manual page documents itself purely as an alias for chan tell (\"has been superceded by the chan tell command which supports the same syntax and options\"), though tell keeps working exactly as described above in every version through 9.1.",
            source: "Tcl tell(n)",
            examples: "set f [open data.log r]\nset offset [tell $f]\nif {[gets $f line] >= 0 && [string match \"ERROR:*\" $line]} {\n    puts \"found: $line\"\n} else {\n    # Not a match -- rewind so the next reader sees this line again\n    seek $f $offset\n}\nclose $f\n\n# A command pipeline is not seekable\nset p [open |[list cat somefile] r]\nputs [tell $p]\nclose $p",
            return_value: "An integer string: the channel's current access position in bytes, or -1 for a channel that does not support seeking.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
