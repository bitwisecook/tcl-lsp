//! `LB::connect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::connect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "LB::connect",
            &["LB::connect"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
