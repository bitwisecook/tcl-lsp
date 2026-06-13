//! `TAP::insight` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::insight",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Accumulates or sends key:value pairs to TAP, returns token.",
            synopsis: &["TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*", "TAP::insight send TAP_INSIGHT_EVENT_TYPE TAP_INSIGHT_REASON"],
            snippet: "With arguments accumulates them as key:value pairs, without arguments sends accumulated to TAP.\nReturns token supplied by TAP service.",
            source: "https://clouddocs.f5.com/api/irules/TAP__insight.html",
            examples: "when TAP_REQUEST {\n    if { ([TAP::insight] eq \"block\") } {\n        drop\n    }\n}",
            return_value: "Returns one of the following actions: allow, block, captcha, conviction, deception, timeout.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
