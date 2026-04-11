//! `LB::persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Forces the system to make a persistence decision.",
            &["LB::persist"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
