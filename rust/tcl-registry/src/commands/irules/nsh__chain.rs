//! `NSH::chain` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::chain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the Chain for flow.",
            synopsis: &["NSH::chain DIRECTION CHAIN_NAME"],
            snippet: "Set: chain for NSH.",
            source: "https://clouddocs.f5.com/api/irules/NSH__chain.html",
            examples: "ntext for NSH.\n            when CLIENT_ACCEPTED {\n                NSH::chain clientside_ingress test1\n            }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "NSH::chain DIRECTION CHAIN_NAME" },
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
