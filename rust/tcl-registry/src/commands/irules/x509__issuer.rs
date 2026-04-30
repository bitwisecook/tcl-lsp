//! `X509::issuer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::issuer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the issuer of an X509 certificate.",
            &["X509::issuer CERTIFICATE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
