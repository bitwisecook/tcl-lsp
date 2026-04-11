//! `sha2::sha256` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha2::sha256",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Compute the SHA-256 hash of a string or file.",
            &["sha2::sha256 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
