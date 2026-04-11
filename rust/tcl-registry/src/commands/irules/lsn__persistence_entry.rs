//! `LSN::persistence-entry` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LSN::persistence-entry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create or lookup LSN translation address.",
            &["LSN::persistence-entry (delete|get) CLIENT_ADDR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
