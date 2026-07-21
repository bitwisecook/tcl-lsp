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

//! `stooop::class` — define a class.
//
// VERIFIED: tcllib stooop(n). `::stooop::class name body` takes exactly
// two arguments. The body is "similar in contents to a Tcl namespace
// (which a class actually also is)" and holds member `proc` definitions
// — it is evaluated as Tcl code in a class-definition context, not the
// caller's frame. Previously registered with `Arity::exact(1)`, which
// rejected every valid two-argument call.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "stooop::class name body",
    dialects: None,
}];

/// Command spec for `stooop::class`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "stooop::class",
        arity: Arity::exact(2),
        // `name` names the class (and becomes a command namespace);
        // `body` is a Tcl definition body to recurse into.
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        // The class body runs in a definition context (the class's own
        // namespace), not the caller's frame — like `proc` and
        // `snit::type` bodies, so it opts out of the enclosing block's
        // data flow.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Defines a stooop class.",
            synopsis: &["stooop::class name body"],
            snippet: "`body` is like a Tcl namespace holding member `proc` definitions; \
                      the class itself is a namespace.",
            source: "tcllib stooop package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("stooop"),
        required_package: Some("stooop"),
        ..CommandSpec::DEFAULT
    }
}
