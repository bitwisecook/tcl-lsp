//! `when` iRules command.
use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Dynamic arg-role resolver for `when EVENT ?priority? { body }`.
///
/// The last argument is always the event-handler body.  The
/// optional `priority` token sits between `EVENT` and `BODY`.
/// Mirrors `_when_arg_roles` in
/// `core/commands/registry/irules/when.py:25-29`.
fn when_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2 {
        if let Ok(idx) = u8::try_from(args.len() - 1) {
            return vec![(idx, ArgRole::Body)];
        }
    }
    Vec::new()
}

/// Keyword tail values: `priority` / `timing` after the event name.
const WHEN_KEYWORD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "priority",
        detail: "Declare handler priority.",
    },
    ArgValue {
        value: "timing",
        detail: "Enable/disable timing metrics for this handler.",
    },
];

/// Timing values: `enable` / `disable` after the `timing` keyword.
const WHEN_TIMING_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "enable",
        detail: "Enable timing metrics for this handler.",
    },
    ArgValue {
        value: "disable",
        detail: "Disable timing metrics for this handler.",
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        traits: Traits::LANGUAGE_KEYWORD
            .union(Traits::IS_EVENT_HANDLER)
            .union(Traits::IRULES_TOP_LEVEL_ONLY),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(2, 6),
        arg_role_resolver: Some(when_arg_roles),
        lowering_hook: Some(LoweringHookId::When),
        // SYNC2: iRules event handler bodies run in the event
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
                kind: FormKind::Default,
                synopsis: "when EVENT { body }",
            },
            FormSpec {
                kind: FormKind::Default,
                synopsis: "when EVENT priority N { body }",
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
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
