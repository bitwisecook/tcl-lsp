//! `SSL::tls13_secret` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::tls13_secret",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return data about various TLS 1.3 secrets.",
            &["SSL::tls13_secret client (app | hs | early)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
