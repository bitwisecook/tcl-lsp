//! `URI::port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the host port from the given URI.",
            &["URI::port URI_STRING"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
