//! `AVR::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables the AVR plugin for the current connection.",
            &["AVR::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
