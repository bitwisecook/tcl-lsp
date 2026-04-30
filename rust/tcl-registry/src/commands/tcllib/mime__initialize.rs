//! `mime::initialize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::initialize",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Create a MIME part from data, a file, or a channel.", &["mime::initialize ?-canonical type? ?-params dict? ?-encoding enc? ?-header dict? ?-string text | -fi"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
