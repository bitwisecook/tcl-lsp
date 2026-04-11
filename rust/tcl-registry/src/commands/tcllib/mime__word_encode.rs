//! `mime::word_encode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::word_encode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet::brief(
            "Encode a string per RFC 2047.",
            &["mime::word_encode charset method string ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
