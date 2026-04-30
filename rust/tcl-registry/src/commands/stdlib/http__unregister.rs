//! `http::unregister` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::unregister",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Unregister a protocol handler from the http package.",
            &["http::unregister proto"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
