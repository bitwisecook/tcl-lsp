//! `traffic_group` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "traffic_group",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current traffic group.",
            &["traffic_group"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
