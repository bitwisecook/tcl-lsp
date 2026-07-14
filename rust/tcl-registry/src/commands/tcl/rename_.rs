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

//! `rename` — rename or delete a command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "rename oldName newName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rename",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        // `HAS_DESTRUCTIVE_OPS`: `Tcl_RenameObjCmd` → `TclRenameCommand`
        // (tclNamesp.c / tclBasic.c) deletes `oldName` (an empty `newName`
        // deletes the command outright) and errors when `oldName` doesn't
        // exist — the property the W302 fire-and-forget suppression
        // (`catch {rename foo ""}`) keys off.
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD | Traits::HAS_DESTRUCTIVE_OPS,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Rename or delete a command",
            synopsis: &["rename oldName newName"],
            snippet: "Rename the command that used to be called oldName so that it is now called newName.",
            source: "Tcl man page rename.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
