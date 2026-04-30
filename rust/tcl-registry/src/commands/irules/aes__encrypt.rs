//! `AES::encrypt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AES::encrypt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Encrypts the data using the previously-created AES key.",
            &["AES::encrypt KEY DATA"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
