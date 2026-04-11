//! `CATEGORY::analytics` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::analytics",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls response analytics engine.",
            &["CATEGORY::analytics BOOL_VALUE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
