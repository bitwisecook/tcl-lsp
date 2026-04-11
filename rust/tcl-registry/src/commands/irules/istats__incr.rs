//! `ISTATS::incr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::incr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Increments the specified key by the given value.",
            &["ISTATS::incr KEY VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
