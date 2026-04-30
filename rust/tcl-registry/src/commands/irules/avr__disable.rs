//! `AVR::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables the AVR plugin for the current connection.",
            &["AVR::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
