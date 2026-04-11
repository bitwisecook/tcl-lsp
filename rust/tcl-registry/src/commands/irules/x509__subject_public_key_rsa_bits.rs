//! `X509::subject_public_key_RSA_bits` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "X509::subject_public_key_RSA_bits",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the size of the subjectXs public RSA key of an X509 certificate.",
            &["X509::subject_public_key_RSA_bits CERTIFICATE"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
