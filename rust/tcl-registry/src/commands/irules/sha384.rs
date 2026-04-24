//! `sha384` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha384",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the Secure Hash Algorithm (SHA2) 384-bit message digest of the specified",
            &["sha384 ANY_CHARS"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
