//! `LB::reselect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::reselect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Advance to the next available node in a pool.",
            &["LB::reselect (clone pool POOL_OBJ (member IP_ADDR)?)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
