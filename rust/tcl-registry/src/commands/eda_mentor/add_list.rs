//! `add_list` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_list",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add signals to the list window (add list).",
            &["add list ?-radix radix? signal_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
