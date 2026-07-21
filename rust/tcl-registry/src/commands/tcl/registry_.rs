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

//! `registry` — Windows registry access (Windows-only Tcl command).
//!
//! Module name has a trailing underscore to avoid a collision with the
//! crate's [`crate::registry`] module.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "registry subcommand keyName ?args ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "registry",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Windows registry manipulation (Windows-only).",
            synopsis: &[
                "registry broadcast keyName ?-timeout ms?",
                "registry delete keyName ?valueName?",
                "registry get keyName valueName",
                "registry keys keyName ?pattern?",
                "registry set keyName ?valueName data ?type??",
                "registry type keyName valueName",
                "registry values keyName ?pattern?",
            ],
            snippet: "Not available in the WASM sandbox (Windows-specific) — traps with ``unsupported command: registry``.",
            source: "Tcl man page registry.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
