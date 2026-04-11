//! `AES::decrypt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::decrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Decrypts the data using the previously-created AES key.",
            &["AES::decrypt KEY DATA"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
