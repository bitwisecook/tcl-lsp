//! `ISTATS::get` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::get",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Retrieves the value associated with the given key.",
            &["ISTATS::get KEY"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
