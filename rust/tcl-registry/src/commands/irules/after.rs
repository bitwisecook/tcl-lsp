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

//! `after` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// Dynamic arg-role resolver for `after`.
///
/// `after cancel ...` / `after info ...` take no script.  The
/// timer-scheduling form `after MILLI_SECONDS (-periodic)?
/// (NESTING_SCRIPT)?` carries the deferred body as its trailing
/// argument (never the `-periodic` flag, and never when only the delay
/// is given).  The script runs later from a timer wakeup in its own
/// dispatch context, so `body_kind` is `Structural`.
fn after_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    match args {
        [] | ["cancel" | "info", ..] => Vec::new(),
        _ => {
            let last = args.len() - 1;
            if last >= 1 && args[last] != "-periodic" {
                u8::try_from(last).map_or_else(|_| Vec::new(), |idx| vec![(idx, ArgRole::Body)])
            } else {
                Vec::new()
            }
        }
    }
}

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "after",
        // `DEFERS_BODY` — the "deferred body" the comment below already
        // states. The Tcl `after` carries it for the same reason
        // (`tcl/after_.rs`); this is the iRules spec of the same command and
        // must not disagree with it (issue #1672 audit). No iRules oracle
        // exists here, but the TMOS `after` is the Tcl one with a timer
        // wakeup, and the divergence would be the surprising claim.
        traits: Traits::DIAGRAM_ACTION.union(Traits::DEFERS_BODY),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(1),
        // The timer form's trailing nesting script is a deferred body
        // (runs from a timer wakeup, not the caller's frame).
        arg_role_resolver: Some(after_arg_roles),
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Execute iRules code after a set period of delay.",
            synopsis: &[
                "after MILLI_SECONDS (-periodic)? (NESTING_SCRIPT)?",
                "after cancel (-current | (ID)+)",
                "after info (ID)*",
            ],
            snippet: "The after command allows you to insert a delay into the processing of your iRule, executing the specified script after a certain amount of time has passed. It also allows for things like periodic (repeat) execution of a script, as well as looking up or canceling currently delayed scripts.\n\nNote: The after command is not available in GTM.\n    \nDelays rule execution for the given milliseconds. If SCRIPT parameter is given, schedules it for execution after the given milliseconds.  If -periodic switch is supplied, the script will be evaluated every given milliseconds.",
            source: "https://clouddocs.f5.com/api/irules/after.html",
            examples: "when RULE_INIT {\n   set users_last_sec 0\n   set new_user_count 0\n   after 1000 ¨Cperiodic {\n      set users_last_sec $new_user_count\n      set new_user_count 0\n   }\n}",
            return_value: "When script is named, an id is returned for the script.",
        }),
        forms: &[FormSpec {
            synopsis: "after MILLI_SECONDS (-periodic)? (NESTING_SCRIPT)?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-periodic",
                    value: OptionValue::flag(),
                    detail: "Option -periodic.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-current",
                    value: OptionValue::flag(),
                    detail: "Option -current.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        xc_translatable: Some(false),
        ..CommandSpec::DEFAULT
    }
}
