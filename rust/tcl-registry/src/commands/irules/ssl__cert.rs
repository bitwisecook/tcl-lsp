//! `SSL::cert` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cert",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns data about an X509 SSL certificate, or sets the certificate mode.",
            &["SSL::cert <index>"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
