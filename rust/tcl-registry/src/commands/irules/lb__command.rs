//! `LB::command` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::command",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "LB::command",
            &["LB::command ('transparent_port')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
