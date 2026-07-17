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

//! `source` — evaluate a file or resource as a Tcl script.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "source ?-encoding name? fileName",
}];

/// Command spec for `source`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "source",
        dialects: None,
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::SOURCES_FILE
            | Traits::DYNAMIC_EVAL_BODY
            | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: const {
            &[OptionSpec {
                name: "-encoding",
                value: OptionValue::value("encoding"),
                detail: "",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Evaluate a file or resource as a Tcl script.",
            synopsis: &["source ?-encoding name? fileName"],
            snippet: "",
            source: "Tcl source(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Source),
        ..CommandSpec::DEFAULT
    }
}
