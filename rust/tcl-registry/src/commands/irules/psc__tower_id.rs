//! `PSC::tower_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::tower_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set tower id.",
            &["PSC::tower_id (TOWER_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
