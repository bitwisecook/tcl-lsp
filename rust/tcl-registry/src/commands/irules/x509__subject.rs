//! `X509::subject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the subject of an X509 certificate.",
            &["X509::subject CERTIFICATE (commonName)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
