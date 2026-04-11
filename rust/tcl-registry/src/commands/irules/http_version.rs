//! `http_version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the HTTP protocol version.",
            &["http_version"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
