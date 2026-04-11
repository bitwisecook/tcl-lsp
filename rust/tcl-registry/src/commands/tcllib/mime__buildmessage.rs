//! `mime::buildmessage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::buildmessage",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Construct a MIME message from a token.",
            &["mime::buildmessage token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
