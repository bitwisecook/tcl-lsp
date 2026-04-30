//! `PROFILE::httpcompression` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::httpcompression",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of an HTTP compression profile setting.",
            &["PROFILE::httpcompression ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
