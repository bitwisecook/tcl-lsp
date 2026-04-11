//! `CLASSIFICATION::urlcat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::urlcat",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: provides classification url category name.",
            &["CLASSIFICATION::urlcat"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
