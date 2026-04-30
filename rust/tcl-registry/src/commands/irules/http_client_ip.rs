//! `http_client_ip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_client_ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Return the first IP address from X-Forwarded-For (or a named header), otherwise ",
            &["call http_client_ip"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
