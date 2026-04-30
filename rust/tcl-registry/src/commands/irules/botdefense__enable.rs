//! `BOTDEFENSE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables processing by Bot Defense on the connection.",
            &["BOTDEFENSE::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
