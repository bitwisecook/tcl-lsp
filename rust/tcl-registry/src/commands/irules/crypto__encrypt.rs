//! `CRYPTO::encrypt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::encrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This iRules command encrypts data.",
            &["CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
