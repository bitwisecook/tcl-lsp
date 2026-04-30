//! `sha1::sha1` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha1::sha1",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Compute the SHA-1 hash of a string or file.",
            &["sha1::sha1 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
