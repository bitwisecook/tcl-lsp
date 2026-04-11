//! `CLASSIFICATION::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Enables classification for the current flow.",
            &["CLASSIFICATION::enable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
