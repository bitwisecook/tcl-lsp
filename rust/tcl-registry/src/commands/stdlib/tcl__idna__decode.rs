//! `tcl::idna::decode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::idna::decode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Decode a hostname from IDNA format to Unicode.",
            &["tcl::idna::decode hostname"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
