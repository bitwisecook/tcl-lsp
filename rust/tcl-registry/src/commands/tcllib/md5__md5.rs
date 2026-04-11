//! `md5::md5` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "md5::md5",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Compute the MD5 hash of a string or file.",
            &["md5::md5 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
