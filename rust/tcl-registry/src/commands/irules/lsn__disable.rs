//! `LSN::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables LSN translation for the current connection if LSN translation has been ",
            &["LSN::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
