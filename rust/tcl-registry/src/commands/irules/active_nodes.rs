//! `active_nodes` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "active_nodes",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the alias for active members of the specified pool (for BIG-IP version 4",
            &["active_nodes ('-list')? POOL_OBJ"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
