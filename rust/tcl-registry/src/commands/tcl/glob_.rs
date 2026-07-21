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

//! `glob` — return names of files that match patterns.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "glob ?switches? ?--? pattern ?pattern ...?",
}];

/// Command spec for `glob`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "glob",
        dialects: None,
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-directory",
                    value: OptionValue::value("dir"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-join",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-nocomplain",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-path",
                    value: OptionValue::value("pathPrefix"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-tails",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-types",
                    value: OptionValue::value("typeList"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Return names of files that match patterns.",
            synopsis: &["glob ?switches? ?--? pattern ?pattern ...?"],
            snippet: "Performs file name globbing similar to `csh`. Returns a list of matching file names.\n\nUse `-nocomplain` to return an empty list instead of an error when no files match. Use `--` before patterns that may start with `-`.",
            source: "Tcl glob(1)",
            examples: "",
            return_value: "A list of file names matching the patterns.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
