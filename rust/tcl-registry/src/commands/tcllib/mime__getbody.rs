//! `mime::getbody` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getbody",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Return the body of a MIME part.",
            &["mime::getbody token ?-decode? ?-command cmdprefix?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
