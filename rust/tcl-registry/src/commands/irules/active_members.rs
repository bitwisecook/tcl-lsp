//! `active_members` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "active_members",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the number or list of active members in the specified pool.",
            &["active_members ('-list')? POOL_OBJ"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
