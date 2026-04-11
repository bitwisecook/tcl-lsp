//! `mime::copymessage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::copymessage",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Copy MIME message to a channel.",
            &["mime::copymessage token channel"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
