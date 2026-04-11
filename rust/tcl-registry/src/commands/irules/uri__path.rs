//! `URI::path` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::path",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the path portion of the given URI.",
            &["URI::path URI_STRING (depth | START | (START END))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
