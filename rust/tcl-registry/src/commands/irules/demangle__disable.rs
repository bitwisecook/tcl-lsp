//! `DEMANGLE::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DEMANGLE::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iRules command `DEMANGLE::disable`.",
            &["DEMANGLE::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
