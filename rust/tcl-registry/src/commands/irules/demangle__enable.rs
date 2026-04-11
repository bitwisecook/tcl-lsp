//! `DEMANGLE::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DEMANGLE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `DEMANGLE::enable`.",
            &["DEMANGLE::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
