//! `findstr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "findstr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Finds a string within another string and returns the string starting at the offs",
            &["findstr STRING SEARCH_STRING ("],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
