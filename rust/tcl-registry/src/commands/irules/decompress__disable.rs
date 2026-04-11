//! `DECOMPRESS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DECOMPRESS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable DECOMPRESS feature on current flow.",
            &["DECOMPRESS::disable (request | response)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
