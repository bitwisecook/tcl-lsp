//! `CLASSIFICATION::category` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::category",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Provides classification category name.",
            &["CLASSIFICATION::category"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
