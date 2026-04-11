//! `link` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "link",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Link the current design to library cells.",
            &["link ?-force?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
