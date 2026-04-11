//! `BOTDEFENSE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disables processing by Bot Defense on the connection.",
            &["BOTDEFENSE::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
