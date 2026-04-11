//! `CRYPTO::hash` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CRYPTO::hash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Generates a hash on a piece of data.", &["CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}
