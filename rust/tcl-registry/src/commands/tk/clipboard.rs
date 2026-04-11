//! `clipboard` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clipboard",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Manipulate the Tk clipboard.",
            &["clipboard append ?-displayof window? ?-format format? ?-type type? data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
