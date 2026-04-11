//! `AVR::log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Log an event for stats.",
            &["AVR::log"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
