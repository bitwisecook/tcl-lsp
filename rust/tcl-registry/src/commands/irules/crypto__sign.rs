//! `CRYPTO::sign` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::sign",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Provides a digital signature of a block of data.", &["CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
