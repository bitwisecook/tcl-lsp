//! `CLASSIFICATION::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Disables classification for the current flow.",
            &["CLASSIFICATION::disable"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
