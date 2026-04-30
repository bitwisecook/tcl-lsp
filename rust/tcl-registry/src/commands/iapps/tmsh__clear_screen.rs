//! `tmsh::clear_screen` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::clear_screen",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Clears the screen.",
            &["tmsh::clear_screen"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
