//! `BOTDEFENSE::cs_allowed` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::cs_allowed",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or sets whether it is allowed for Bot Defense to take a client-side acti",
            &["BOTDEFENSE::cs_allowed (BOOLEAN)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
