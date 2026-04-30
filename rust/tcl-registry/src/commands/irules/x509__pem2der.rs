//! `X509::pem2der` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::pem2der",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns an X509 certificate in DER format.",
            &["X509::pem2der <cert-in-pem>"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
