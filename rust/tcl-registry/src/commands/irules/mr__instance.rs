//! `MR::instance` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::instance",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the name of the current mr_router instance.",
            synopsis: &["MR::instance"],
            snippet: "returns the name of the current mr_router instance",
            source: "https://clouddocs.f5.com/api/irules/MR__instance.html",
            examples: "when MR_INGRESS {\n    log local0. \"[MR::protocol] router instance [MR::instance]\"\n}",
            return_value: "returns the name of the current mr_router instance",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::instance",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
