//! `PROFILE::avr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::avr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a avr profile setting.",
            &["PROFILE::avr ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
