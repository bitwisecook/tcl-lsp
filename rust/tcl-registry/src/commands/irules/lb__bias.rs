//! `LB::bias` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::bias",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `LB::bias`.",
            &["LB::bias (INTEGER)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
