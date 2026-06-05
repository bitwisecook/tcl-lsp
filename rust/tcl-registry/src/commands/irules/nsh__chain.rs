//! `NSH::chain` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
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
        ..CommandSpec::DEFAULT
    }
}
