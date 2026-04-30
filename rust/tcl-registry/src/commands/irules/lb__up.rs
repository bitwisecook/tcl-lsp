//! `LB::up` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::up",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the status of a node or pool member as being up.",
            &["LB::up"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
