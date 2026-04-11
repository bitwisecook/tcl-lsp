//! `LSN::persistence` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::persistence",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set the translation address and port selection mode for the current connection, ",
            &["LSN::persistence none (TIMEOUT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
