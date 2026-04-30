//! `members` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "members",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Lists all members of a given pool for v10.x.x.",
            &["members ('-list')? (POOL_OBJ)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
