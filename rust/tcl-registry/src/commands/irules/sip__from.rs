//! `SIP::from` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::from",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of the From header in a SIP request.",
            &["SIP::from"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
