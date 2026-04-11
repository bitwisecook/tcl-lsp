//! `FLOWTABLE::limit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOWTABLE::limit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns configured connection limits.",
            &["FLOWTABLE::limit virtual (VIRTUAL_SERVER_OBJ)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
