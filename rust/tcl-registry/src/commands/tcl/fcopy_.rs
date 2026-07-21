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

const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "fcopy inputChan outputChan ?-size size? ?-command callback?" },
];

/// Command spec for `fcopy`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fcopy",
        dialects: None,
        // C Tcl 9.0 ``Tcl_FcopyObjCmd`` accepts up to four optional
        // option-pair flags after the two channels (``-size N``,
        // ``-command cb``).  Args after command name: 2..6.
        arity: Arity::new(2, 6),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
hover: Some(HoverSnippet {
            summary: "Copy data from one channel to another",
            synopsis: &["fcopy inputChan outputChan ?-size size? ?-command callback?"],
            snippet: "The fcopy command copies data from one I/O channel, inchan, to another I/O channel, outchan.",
            source: "Tcl man page fcopy.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
