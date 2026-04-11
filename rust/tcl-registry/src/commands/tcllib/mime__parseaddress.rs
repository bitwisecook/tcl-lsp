//! `mime::parseaddress` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::parseaddress",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Parse an RFC 2822 address string.",
            &["mime::parseaddress string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
