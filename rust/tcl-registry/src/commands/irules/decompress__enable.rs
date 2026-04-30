//! `DECOMPRESS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DECOMPRESS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable DECOMPRESS feature on current flow.",
            &["DECOMPRESS::enable (request | response)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
