//! `http_header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Evaluates the string following an HTTP header tag that you specify.",
            &["http_header"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
