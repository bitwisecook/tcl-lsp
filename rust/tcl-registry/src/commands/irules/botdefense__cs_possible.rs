//! `BOTDEFENSE::cs_possible` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_possible",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns whether it is possible for Bot Defense to take a client-side action.",
            &["BOTDEFENSE::cs_possible"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
