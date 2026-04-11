//! `SIP::to` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::to",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of the To header in a SIP request.",
            &["SIP::to"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
