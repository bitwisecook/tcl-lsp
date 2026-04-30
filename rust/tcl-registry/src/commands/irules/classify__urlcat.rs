//! `CLASSIFY::urlcat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::urlcat",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allows to set or add an url category to the classification.",
            &["CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
