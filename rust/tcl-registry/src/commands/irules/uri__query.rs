//! `URI::query` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::query",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the query string portion of the given URI or the value of a query string",
            &["URI::query URI_STRING (PARAMETER_NAME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
