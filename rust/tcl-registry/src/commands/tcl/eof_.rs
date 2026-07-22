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

//! `eof` — check for end-of-file condition on a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "eof channel",
    dialects: None,
}];

/// Command spec for `eof`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "eof",
        dialects: None,
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Check for end of file condition on channel",
            synopsis: &["eof channel"],
            snippet: "Returns 1 if an end-of-file condition occurred during the most recent input operation on channel (such as gets or read), 0 otherwise. eof performs no I/O of its own; it only reports the outcome of the last read, so a later read that reaches new data (for example on a pipe or socket where more input arrives) can clear the condition again. Tcl 8.5 introduced chan eof, an equivalent command reached through the chan ensemble; from Tcl 9.0 the eof manual page documents itself purely as an alias for chan eof, though eof keeps working exactly as before in every version through 9.1.",
            source: "Tcl eof(n)",
            examples: "set f [open somefile.txt]\nwhile {1} {\n    set line [gets $f]\n    if {[eof $f]} {\n        close $f\n        break\n    }\n    puts \"Read line: $line\"\n}",
            return_value: "1 if the channel is at end-of-file, 0 otherwise.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
