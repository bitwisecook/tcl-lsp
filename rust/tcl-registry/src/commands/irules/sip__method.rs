//! `SIP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the type of SIP request method.",
            &["SIP::method"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
