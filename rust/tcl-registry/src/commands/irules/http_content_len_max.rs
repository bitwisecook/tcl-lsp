//! `http_content_len_max` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_content_len_max",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Return the HTTP Content-Length up to a maximum size (default 1024), or reject if",
            &["call http_content_len_max"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
