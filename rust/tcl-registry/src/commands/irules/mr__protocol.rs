//! `MR::protocol` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns generic, sip or diameter.",
            synopsis: &["MR::protocol"],
            snippet: "returns generic, sip or diameter",
            source: "https://clouddocs.f5.com/api/irules/MR__protocol.html",
            examples: "when MR_INGRESS {\n    log local0. \"[MR::protocol] router instance [MR::instance]\"\n}",
            return_value: "returns generic, sip or diameter",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::protocol" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
