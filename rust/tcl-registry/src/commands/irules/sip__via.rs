//! `SIP::via` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::via",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets SIP via header information.",
            &["SIP::via ?field? ?INDEX?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
