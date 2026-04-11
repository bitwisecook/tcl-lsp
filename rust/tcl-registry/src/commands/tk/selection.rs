//! `selection` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "selection",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Manipulate the X selection.",
            &["selection clear ?-displayof window? ?-selection selection?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
