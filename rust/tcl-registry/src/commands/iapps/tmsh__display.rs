//! `tmsh::display` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::display",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Provides access to the tmsh pager.",
            &["tmsh::display <text>"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
