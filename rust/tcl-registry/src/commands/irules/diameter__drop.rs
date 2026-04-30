//! `DIAMETER::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Drops the current message quietly.",
            &["DIAMETER::drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
