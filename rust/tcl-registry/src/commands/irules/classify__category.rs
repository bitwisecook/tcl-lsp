//! `CLASSIFY::category` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::category",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allows to set or add a category name to the classification.",
            &["CLASSIFY::category ('set' | 'add') CLASSIFY_CATEGORY_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
