//! `PROFILE::stream` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::stream",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a Stream profile setting.",
            &["PROFILE::stream ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
