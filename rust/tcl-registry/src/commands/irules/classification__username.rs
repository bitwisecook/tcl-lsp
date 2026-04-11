//! `CLASSIFICATION::username` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::username",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Provides username associated with classification results.",
            &["CLASSIFICATION::username"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
