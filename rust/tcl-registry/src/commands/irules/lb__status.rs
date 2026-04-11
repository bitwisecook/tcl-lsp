//! `LB::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the status of a node address or pool member.",
            &["LB::status (LB_STATUS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
