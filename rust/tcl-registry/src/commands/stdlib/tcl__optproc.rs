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

//! `tcl::OptProc` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::OptProc",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        // `DEFINES_PROCEDURE` drives the proc-name-declaration semantic
        // token/hover override (`semantic_tokens.rs`'s `ArgOverride::ProcNameDef`)
        // the same way it does for `proc` (issue #923 idx 90). The runtime
        // installs the defined proc as an ordinary, byte-compiled Tcl proc
        // (`NOT_PROC_FACTORY`/`BYTE_COMPILED`/`NEVER_INLINE_BODY` all follow
        // `proc`'s own precedent); `LANGUAGE_KEYWORD` and
        // `IRULES_TOP_LEVEL_ONLY` are deliberately omitted — this is a
        // `package require opt` library proc, not a core keyword, and
        // nothing establishes the iRules-top-level restriction for it.
        // `DEFERS_BODY` for the same reason `proc` carries it: this *defines*
        // a procedure, so the body is stored, never run here. tclsh 8.6.16 /
        // 9.0.4, byte-identical: `proc p {} { ::tcl::OptProc q {} {error
        // stop}; set ::reached 1 }` sets `::reached` (issue #1672 audit).
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::DEFINES_PROCEDURE
            | Traits::NEVER_INLINE_BODY
            | Traits::DEFERS_BODY,
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        // Runs in its own fresh call frame on each call, exactly like
        // `proc` — lets generic `body_indices_to_skip` consumers (SSA,
        // dead-store, def-use) treat it like every other structural-body
        // definer without a string-match special case (issue #923 idx 90).
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Define a proc with automatic option parsing.",
            synopsis: &["tcl::OptProc name optlist body"],
            snippet: "Defines a procedure *name* whose arguments are parsed according to *optlist*, a list of option descriptions.  Inside *body*, option values are available as local variables.",
            source: "Tcl stdlib opt package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("opt"),
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::OptProc),
        command_table_effect: Some(crate::command_table::CommandTableEffect::DefinesProcedure),
        ..CommandSpec::DEFAULT
    }
}
