//! `SSL::modssl_sessionid_headers` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::modssl_sessionid_headers",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a list of fields for HTTP headers.",
            &["SSL::modssl_sessionid_headers (initial | current)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
