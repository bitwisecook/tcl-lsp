//! `SSL::sessionid` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the SSL session ID.",
            &["SSL::sessionid (desired)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
