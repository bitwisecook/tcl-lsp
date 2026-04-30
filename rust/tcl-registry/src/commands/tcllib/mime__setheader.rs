//! `mime::setheader` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::setheader",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet::brief(
            "Set a header on a MIME token.",
            &["mime::setheader token key value ?-mode mode?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
