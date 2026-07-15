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

//! `unset` — delete variables.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unset ?-nocomplain? ?--? ?name name name ...?",
}];

/// `unset ?-nocomplain? ?--? ?name name name ...?` — every trailing word is a
/// variable name, not just the first.  Resolve `VarWrite` dynamically (skipping
/// the leading options, mirroring `lower_unset`) so all names highlight as
/// variables rather than only the first (issue #774).
///
/// `unset` recognises exactly two options: `-nocomplain` (skippable, repeatable)
/// and `--` (terminator).  Any *other* leading word — including one that begins
/// with `-`, such as `unset -foo bar` — is a real variable name, not an option
/// (verified against tclsh 8.6/9.0); it must keep its `VarWrite` role.
fn unset_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-nocomplain" => i += 1,
            "--" => {
                i += 1;
                break;
            }
            _ => break,
        }
    }
    (i..args.len())
        .filter_map(|j| u8::try_from(j).ok().map(|j| (j, ArgRole::VarWrite)))
        .collect()
}

/// Command spec for `unset`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unset",
        // `FIRE_AND_FORGET_TEARDOWN`: `Tcl_UnsetObjCmd` (tclCmdMZ.c) removes the
        // variable and errors ("can't unset …: no such variable") when the
        // target is already gone (absent `-nocomplain`) — the property the
        // W302 fire-and-forget suppression (`catch {unset x}`) keys off.
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::DESTROYS_VARIABLE
            | Traits::FIRST_ARG_VARNAME
            | Traits::FIRE_AND_FORGET_TEARDOWN,
        // Names are optional: tclsh 8.5+ accepts `unset`, `unset -nocomplain`,
        // and `unset --` as valid no-ops (the `-nocomplain` option was added in
        // 8.5).  The "consumed all args, unset nothing" footgun is surfaced by
        // W217, not an arity error.
        arity: Arity::at_least(0),
        arg_role_resolver: Some(unset_arg_roles),
        // NOTE: `unset` does NOT set `assigns_variable_at`. That field means
        // "arg N is assigned a value" (the `set`/`incr` shape); `unset`
        // *destroys* variables, which is modelled by the `DESTROYS_VARIABLE`
        // trait plus the `VarWrite` roles from `arg_role_resolver`. Declaring
        // `assigns_variable_at: Some(0)` both contradicts the arg-role resolver
        // (arg 0 may be `-nocomplain`) and pre-empted the destroy branch in the
        // side-effect classifier, modelling `unset x` as a *write* to `x` and
        // `unset -nocomplain x` as a write to `-nocomplain` (RUST_ISSUE_081/078).
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-nocomplain",
                    value: OptionValue::flag(),
                    detail: "Suppress errors for non-existent variables.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "End of options.",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Delete variables",
            synopsis: &["unset ?-nocomplain? ?--? ?name name name ...?"],
            snippet: "This command removes one or more variables.",
            source: "Tcl man page unset.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Unset),
        codegen_hook: Some(CodegenHookId::Unset),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
