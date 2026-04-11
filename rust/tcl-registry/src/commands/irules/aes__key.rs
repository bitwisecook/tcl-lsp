//! `AES::key` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::key",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates an AES key to encrypt/decrypt data.",
            &["AES::key ('128' | '192' | '256')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
