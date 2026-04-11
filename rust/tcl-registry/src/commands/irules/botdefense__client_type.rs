//! `BOTDEFENSE::client_type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::client_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the client type: browser, mobile application or bot.",
            &["BOTDEFENSE::client_type"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
