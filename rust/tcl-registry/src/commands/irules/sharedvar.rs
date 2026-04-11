//! `sharedvar` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sharedvar",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allows a variable to be accessed in both sides of a VIP-targetting-VIP.",
            &["sharedvar VARIABLE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
