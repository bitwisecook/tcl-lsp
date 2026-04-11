//! `CRYPTO::verify` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::verify",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Verifies a signed block of data.", &["CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
