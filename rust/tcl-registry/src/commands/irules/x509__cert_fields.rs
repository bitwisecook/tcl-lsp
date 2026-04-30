//! `X509::cert_fields` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::cert_fields",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL",
            &["X509::cert_fields CERTIFICATE ERROR_CODE ((hash"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
