//! `NSH::context` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::context",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets/Get the Context header for NSH.",
            synopsis: &["NSH::context NSH_CONTEXT_IDX DIRECTION (CONTEXT)?"],
            snippet: "Set: context for NSH.\n            Get(NSH_CONTEXT_IDX and DIRECTION as the only parameter): context from NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__context.html",
            examples: "ntext for NSH.\n            when CLIENT_ACCEPTED {\n                NSH::context 1 serverside_egress 1111\n                set myctx1 [NSH::context 1 serverside_egress]\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "NSH::context NSH_CONTEXT_IDX DIRECTION (CONTEXT)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
