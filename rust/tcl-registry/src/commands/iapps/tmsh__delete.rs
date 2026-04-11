//! `tmsh::delete` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::delete",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Mirrors the tmsh ``delete`` command.",
            &["tmsh::delete <component> <name>"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
