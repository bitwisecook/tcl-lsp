//! `base64::decode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "base64::decode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Decode a base64-encoded string back to binary data.",
            &["base64::decode encodedData"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
