//! `htons` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "htons",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Converts the unsigned short integer from host byte order to network byte order.",
            &["htons NUMBER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
