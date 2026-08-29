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

//! `speclib` — the one loader directive of a `SpecTcl` pack.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "speclib name dsl-version { declarations }",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "speclib",
        // A pack body is a *declaration* scope, not enclosing-scope data flow
        // — the same classification `oo::class` / `snit::type` carry.
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY | Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SPECTCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Open a SpecTcl command pack.",
            synopsis: &["speclib name dsl-version { declarations }"],
            snippet: "The one loader directive of a `.tclspec` pack. `dsl-version` is the DSL *vocabulary* version, not the library's: it gates hard breaks (a word whose meaning changed), never additions. A pack is one Tcl script, read from the CST and never executed — the only Tcl ever evaluated is a hook body, at query time, in the sandbox.",
            source: "SpecTcl (docs/design/spec-packs.md)",
            examples: "speclib mylib 1.0 {\n    default required_package mylib\n    command mylib::sort {\n        arity 1..\n        option -command -takes commandprefix -appends {Exactly 2}\n    }\n}",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::Name), (2, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        definition_body: Some(&crate::definer::SPECTCL_PACK_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
