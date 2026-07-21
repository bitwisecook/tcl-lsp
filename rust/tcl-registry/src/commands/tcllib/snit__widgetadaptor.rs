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

//! `snit::widgetadaptor` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::widgetadaptor name definition",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::widgetadaptor",
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::CREATES_DYNAMIC_BARRIER,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Define a snit widget adaptor that wraps an existing widget.",
            synopsis: &["snit::widgetadaptor name definition"],
            snippet: "",
            source: "tcllib snit package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(1, ArgRole::Body)],
        tcllib_package: Some("snit"),
        required_package: Some("snit"),
        definition_body: Some(&crate::definer::SNIT_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
