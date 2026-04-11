//! `ntohs` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ntohs",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Converts the unsigned short integer from network byte order to host byte order.",
            &["ntohs NUMBER"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
