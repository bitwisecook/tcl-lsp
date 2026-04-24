//! `mime::field_decode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::field_decode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Decode an RFC 2047 header field.",
            &["mime::field_decode field"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
