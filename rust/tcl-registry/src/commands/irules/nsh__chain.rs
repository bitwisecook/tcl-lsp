//! `NSH::chain` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NSH::chain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the Chain for flow.",
            &["NSH::chain DIRECTION CHAIN_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
