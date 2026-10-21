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
use tcl_dialect::model::SpecSurface;

/// Dynamic arg-role resolver for `when EVENT ?priority? { body }`.
///
/// The last argument is always the event-handler body.  The
/// optional `priority` token sits between `EVENT` and `BODY`.
fn when_arg_roles(argument_count: usize) -> Vec<(u8, ArgRole)> {
    if argument_count >= 2
        && let Ok(idx) = u8::try_from(argument_count - 1)
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

/// Timing values completion offers after the `timing` keyword: `enable` /
/// `disable`, the spellings this command's documentation
/// (<https://clouddocs.f5.com/api/irules/when.html>, quoted in the hover
/// synopsis below as `when EVENT timing enable|disable { body }`) writes. The
/// declaration layout still accepts every documented spelling
/// ([`crate::events::IRULES_TIMING_SPELLINGS`], `on`/`off` included); only
/// these are offered.
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
            .union(Traits::IRULES_TOP_LEVEL_ONLY)
            // Registering a handler stores its body for the dispatcher to
            // run on a later event; nothing in it executes at this call.
            .union(Traits::DEFERS_BODY)
            // Each firing runs the body in a frame of its own, the event's.
            .union(Traits::BODY_RUNS_IN_OWN_FRAME),
        surface: Some(SpecSurface::IRULES),
        event_handler_priority: Some(BIGIP_EVENT_HANDLER_PRIORITY),
        arity: Arity::new(2, 6),
        arg_role_count_resolver: Some(when_arg_roles),
        arg_role_resolver_roles: &[ArgRole::Body],
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
        // The clause vocabulary of the declaration layout: argument 1 holds
        // the clause keywords and argument 2 the timing values (the value
        // position of a leading `timing` clause). Which of them a given
        // position takes — a clause keyword at most once and in layout order,
        // a timing value only after `timing` — is the registry's
        // `irules_event_declaration_slot`, not a per-index table.  Event-name
        // completion at index 0 is handled via `Traits::IS_EVENT_HANDLER`.
        arg_values: &[(1, WHEN_KEYWORD_VALUES), (2, WHEN_TIMING_VALUES)],
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slot each position of an incomplete `when` takes, by the layout
    /// `irules_event_declaration_shape` validates.
    #[test]
    fn declaration_slots_follow_the_declaration_layout() {
        use crate::events::IrulesDeclarationSlot as Slot;
        let context = crate::model::ingress::static_context_for("f5-irules");
        let registry = context.commands();
        let when = registry.get("when").expect("when");
        let slot = |written: &[Option<&str>]| {
            registry.irules_event_declaration_slot(when, written, written.len(), None)
        };
        let words = |slot: Option<Slot>| match slot {
            Some(Slot::Clause(values) | Slot::Timing(values)) => {
                values.iter().map(|value| value.value).collect::<Vec<_>>()
            }
            other => panic!("expected a word list, got {other:?}"),
        };
        let e = Some("HTTP_REQUEST");
        assert_eq!(slot(&[]), Some(Slot::Event));
        assert_eq!(words(slot(&[e])), ["priority", "timing"]);
        assert_eq!(slot(&[e, Some("priority")]), Some(Slot::Priority));
        assert_eq!(words(slot(&[e, Some("timing")])), ["enable", "disable"]);
        assert_eq!(slot(&[e, Some("bogus")]), None);
        // After `priority N` only `timing` may follow; after `timing V` only
        // the body.
        assert_eq!(words(slot(&[e, Some("priority"), Some("10")])), ["timing"]);
        assert_eq!(slot(&[e, Some("timing"), Some("enable")]), Some(Slot::Body));
        assert_eq!(
            words(slot(&[e, Some("priority"), Some("10"), Some("timing")])),
            ["enable", "disable"]
        );
        assert_eq!(
            slot(&[e, Some("priority"), Some("10"), Some("timing"), Some("off")]),
            Some(Slot::Body)
        );
        // Out-of-order, repeated or rejected clauses leave the layout.
        assert_eq!(
            slot(&[e, Some("timing"), Some("on"), Some("priority")]),
            None
        );
        assert_eq!(
            slot(&[e, Some("priority"), Some("1001"), Some("timing")]),
            None
        );
        assert_eq!(
            slot(&[e, Some("timing"), Some("sometimes"), Some("x")]),
            None
        );
        // A non-literal word before the position is not inferred across.
        assert_eq!(slot(&[None, Some("priority")]), None);
        assert_eq!(slot(&[e, None]), None);
        // Not an event handler.
        let proc_spec = registry.get("proc").expect("proc");
        assert_eq!(
            registry.irules_event_declaration_slot(proc_spec, &[e], 1, None),
            None
        );
    }

    /// Walking the slots along every complete form the shape accepts reaches
    /// the body exactly where the shape puts it: the two read one layout.
    #[test]
    fn declaration_slots_agree_with_the_complete_shape() {
        use crate::events::IrulesDeclarationSlot as Slot;
        let context = crate::model::ingress::static_context_for("f5-irules");
        let registry = context.commands();
        let when = registry.get("when").expect("when");
        for form in [
            &["HTTP_REQUEST"][..],
            &["HTTP_REQUEST", "priority", "0"],
            &["HTTP_REQUEST", "timing", "on"],
            &["HTTP_REQUEST", "timing", "disable"],
            &["HTTP_REQUEST", "priority", "1000", "timing", "off"],
        ] {
            let written: Vec<Option<&str>> = form.iter().copied().map(Some).collect();
            let body_slot =
                registry.irules_event_declaration_slot(when, &written, written.len(), None);
            let source = format!("when {} {{}}\n", form.join(" "));
            let accepted =
                crate::events::top_level_when_handler_candidates_with_registry_and_head_resolver(
                    &source,
                    registry,
                    &crate::events::WrittenHeadResolver,
                );
            assert_eq!(accepted.len(), 1, "{form:?}: the shape accepts the form");
            if form.len() == 5 {
                assert_eq!(body_slot, Some(Slot::Body), "{form:?}");
            } else {
                assert!(
                    matches!(body_slot, Some(Slot::Body | Slot::Clause(_))),
                    "{form:?}: {body_slot:?}"
                );
            }
        }
    }

    #[test]
    fn source_body_roles_require_cardinality_and_preserve_unknown_values() {
        use crate::InvocationWord::{Dynamic, Expanded, Literal};
        let context = crate::model::ingress::static_context_for("f5-irules");
        for arguments in [
            vec![Literal("HTTP_REQUEST"), Dynamic],
            vec![
                Literal("HTTP_REQUEST"),
                Literal("priority"),
                Literal("10"),
                Dynamic,
            ],
        ] {
            let resolution =
                crate::model::assembly::resolve_structured_invocation_in_resolved_context(
                    context.commands(),
                    Some(context.context()),
                    crate::InvocationWords::structured(Literal("when"), &arguments),
                    tcl_dialect::model::InvocationRealm::RuleLoader,
                );
            let selected = resolution.resolved().unwrap();
            assert_eq!(
                selected.authored_source_argument_roles(),
                (
                    vec![(u8::try_from(arguments.len() - 1).unwrap(), ArgRole::Body)],
                    true
                )
            );
        }
        let arguments = [Literal("HTTP_REQUEST"), Expanded];
        let resolution = crate::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            crate::InvocationWords::structured(Literal("when"), &arguments),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        );
        assert!(
            !resolution
                .resolved()
                .unwrap()
                .authored_source_argument_roles()
                .1
        );
    }
}
