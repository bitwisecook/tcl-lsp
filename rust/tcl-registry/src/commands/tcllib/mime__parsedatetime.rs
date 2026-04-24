//! `mime::parsedatetime` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::parsedatetime",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Parse an RFC 2822 date/time string.",
            &["mime::parsedatetime datestring property"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
