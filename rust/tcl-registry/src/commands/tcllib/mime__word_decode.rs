//! `mime::word_decode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::word_decode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Decode an RFC 2047 encoded word.",
            &["mime::word_decode encoded"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
