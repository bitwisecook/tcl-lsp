//! `pool` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pool",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Select a load-balancing pool for the current flow.",
            &["pool pool_name ?member_addr member_port?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
