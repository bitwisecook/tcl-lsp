//! `session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Utilizes the persistence table to store arbitrary information based on the same ",
            &["session add SESSION_MODE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
