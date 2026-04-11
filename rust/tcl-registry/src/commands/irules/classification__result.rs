//! `CLASSIFICATION::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides classification results.",
            &["CLASSIFICATION::result"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
