//! `htonl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "htonl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Converts the unsigned integer from host byte order to network byte order.",
            &["htonl NUMBER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
