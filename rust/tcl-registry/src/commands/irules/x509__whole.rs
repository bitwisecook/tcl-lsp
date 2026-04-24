//! `X509::whole` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::whole",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns an X509 certificate in PEM format.",
            &["X509::whole CERTIFICATE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
