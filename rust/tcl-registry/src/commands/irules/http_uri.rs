//! `http_uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a URL, but does not include the protocol and the fully qualified domain ",
            &["http_uri"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
