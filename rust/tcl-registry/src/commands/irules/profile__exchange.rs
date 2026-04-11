//! `PROFILE::exchange` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::exchange",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `PROFILE::exchange`.",
            &["PROFILE::exchange ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
