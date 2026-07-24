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

//! `foreach_in_collection` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "foreach_in_collection var collection body",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreach_in_collection",
        traits: Traits::CONTROL_FLOW
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Iterate over objects in a collection.",
            &["foreach_in_collection var collection body"],
            "F5",
        )),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        analyser_hook: Some(crate::hooks::AnalyserHookId::Foreach),
        ..CommandSpec::DEFAULT
    }
}
