//! `nodes` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "nodes",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Lists all nodes within a given pool.",
            &["nodes (-list)? POOL_OBJ"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
