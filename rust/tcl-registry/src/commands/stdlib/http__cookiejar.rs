//! `http::cookiejar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::cookiejar",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create or configure an HTTP cookie jar (TclOO class).",
            &["http::cookiejar create name ?filename?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
