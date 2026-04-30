//! `CLASSIFICATION::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Depreated: provides classification for the least explicit application name.",
            &["CLASSIFICATION::protocol"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
