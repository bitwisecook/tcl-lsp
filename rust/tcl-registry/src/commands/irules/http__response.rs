//! `HTTP::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the raw HTTP response header block as a single string.",
            &["HTTP::response"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
