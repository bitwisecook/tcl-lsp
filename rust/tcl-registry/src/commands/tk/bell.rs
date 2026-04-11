//! `bell` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bell",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Ring the display's bell.",
            &["bell ?-displayof window? ?-nice?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
