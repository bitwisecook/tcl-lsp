//! `BOTDEFENSE::reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the reason for the Bot Defense action.",
            &["BOTDEFENSE::reason"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
