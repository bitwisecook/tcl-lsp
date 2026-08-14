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

//! `when` iRules command.
use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Dynamic arg-role resolver for `when EVENT ?priority? { body }`.
///
/// The last argument is always the event-handler body.  The
/// optional `priority` token sits between `EVENT` and `BODY`.
fn when_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2
        && let Ok(idx) = u8::try_from(args.len() - 1)
    {
        return vec![(idx, ArgRole::Body)];
    }
    Vec::new()
}

/// Keyword tail values: `priority` / `timing` after the event name.
const WHEN_KEYWORD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "priority",
        detail: "Declare handler priority.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "timing",
        detail: "Enable/disable timing metrics for this handler.",
        ..ArgValue::DEFAULT
    },
];

/// Timing values: `enable` / `disable` after the `timing` keyword.
const WHEN_TIMING_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "enable",
        detail: "Enable timing metrics for this handler.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "disable",
        detail: "Disable timing metrics for this handler.",
        ..ArgValue::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        traits: Traits::LANGUAGE_KEYWORD
            .union(Traits::IS_EVENT_HANDLER)
            .union(Traits::IRULES_TOP_LEVEL_ONLY),
        dialects: Some(DialectSet::IRULES),
        event_handler_priority: Some(BIGIP_EVENT_HANDLER_PRIORITY),
        arity: Arity::new(2, 6),
        arg_role_resolver: Some(when_arg_roles),
        // The event name (argument 0) is the handler's outline entry: an
        // iRule's structure *is* its `when` blocks, so the outline, the
        // breadcrumb bar and workspace symbols list them the way they list a
        // `proc`.  Declared as registry data, so the document-symbol provider
        // discovers it generically rather than matching on the command name.
        defines_symbol: Some(SymbolDef::new(0, DefinedSymbolKind::Event)),
        lowering_hook: Some(LoweringHookId::When),
        // iRules event handler bodies run in the event
        // dispatcher's frame — separate from the top-level rule
        // file's evaluation context.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Declare an iRules event handler block.",
            synopsis: &[
                "when EVENT { body }",
                "when EVENT priority N { body }",
                "when EVENT timing enable|disable { body }",
            ],
            snippet: "`body` runs whenever the specified BIG-IP event fires.",
            source: "https://clouddocs.f5.com/api/irules/when.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec {
                synopsis: "when EVENT { body }",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                synopsis: "when EVENT priority N { body }",
                ..FormSpec::DEFAULT
            },
        ],
        // Command-level arg-value completion for the keyword tail:
        // `when EVENT priority|timing …` and `when EVENT timing
        // enable|disable …`.  Even indices that follow `timing` carry
        // the enable/disable values, odd indices carry the
        // priority/timing keywords.  Event-name completion at index 0
        // is handled separately via `Traits::IS_EVENT_HANDLER`.
        arg_values: &[
            (1, WHEN_KEYWORD_VALUES),
            (2, WHEN_TIMING_VALUES),
            (3, WHEN_KEYWORD_VALUES),
            (4, WHEN_TIMING_VALUES),
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
