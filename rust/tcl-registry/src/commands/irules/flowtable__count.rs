//! `FLOWTABLE::count` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOWTABLE::count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns flow counts.",
            &["FLOWTABLE::count (virtual (VIRTUAL_SERVER_OBJ)?)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
