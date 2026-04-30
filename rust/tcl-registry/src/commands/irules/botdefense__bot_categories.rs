//! `BOTDEFENSE::bot_categories` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::bot_categories",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the list of category names to which the current client belongs.",
            &["BOTDEFENSE::bot_categories"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
