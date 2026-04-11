//! `X509::verify_cert_error_string` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::verify_cert_error_string",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns an X509 certificate error string.",
            &["X509::verify_cert_error_string ERROR_CODE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
