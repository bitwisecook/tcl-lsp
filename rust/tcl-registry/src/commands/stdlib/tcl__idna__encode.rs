//! `tcl::idna::encode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::idna::encode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Encode a hostname to IDNA (Internationalised Domain Names) format.",
            &["tcl::idna::encode hostname"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
