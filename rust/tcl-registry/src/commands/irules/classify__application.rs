//! `CLASSIFY::application` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::application",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Allows to set or add an app name to the classification.",
            &["CLASSIFY::application ('set' | 'add') CLASSIFY_APP_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
