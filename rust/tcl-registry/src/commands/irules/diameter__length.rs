//! `DIAMETER::length` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::length",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets diameter message length.",
            &["DIAMETER::length"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
