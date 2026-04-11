//! `X509::subject_public_key_type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the subjectXs public key type of an X509 certificate.",
            &["X509::subject_public_key_type CERTIFICATE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
