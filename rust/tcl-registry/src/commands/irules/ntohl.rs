//! `ntohl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ntohl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Converts the unsigned integer from network byte order to host byte order.",
            &["ntohl NUMBER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
