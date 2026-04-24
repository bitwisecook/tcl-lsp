//! `CRYPTO::keygen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::keygen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Generates keys that can be used to encrypt and sign data.",
            &["CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
