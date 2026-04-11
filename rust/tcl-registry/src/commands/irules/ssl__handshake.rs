//! `SSL::handshake` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::handshake",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Halts or resumes SSL activity.",
            &["SSL::handshake (hold | resume)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
