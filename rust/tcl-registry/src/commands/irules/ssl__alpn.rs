//! `SSL::alpn` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::alpn",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Handle the ALPN TLS extension.",
            &["SSL::alpn set (ARG)+"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
