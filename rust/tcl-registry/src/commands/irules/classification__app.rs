//! `CLASSIFICATION::app` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::app",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Provides classification for the most explicit application name.",
            &["CLASSIFICATION::app"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
