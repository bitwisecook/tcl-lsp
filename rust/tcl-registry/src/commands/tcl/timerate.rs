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

//! `timerate` — measure the rate of execution of a script.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
}];

/// Command spec for `timerate`.
///
/// Like its sibling `time`, the command takes a `command` argument plus
/// optional `time` / `max-count` positionals and leading `-direct`
/// / `-calibrate` / `-overhead` options, returning a measured-rate
/// summary string.  Arg 0 is a `BODY`, arg 1 an `INT` (shimmers),
/// the arity is unbounded, and there is an `UNKNOWN`-target read+write
/// side effect (the body runs arbitrary code).  Follows the tcl pack's
/// `dialects: None` convention for vanilla commands — iRules exclusion is
/// handled via the separate iRules pack, not per-command (sibling `time`
/// is `None` too).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timerate",
        dialects: None,
        // The body's position varies with the leading options, so we
        // don't pin a fixed BODY index beyond arg 0; the option /
        // positional mix makes the upper arity bound unbounded.
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Measure the rate of execution of a script.",
            synopsis: &[
                "timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
            ],
            snippet: "",
            source: "Tcl timerate(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
