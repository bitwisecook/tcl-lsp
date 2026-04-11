//! `LB::prime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::prime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Prime server connections.",
            &["LB::prime"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
