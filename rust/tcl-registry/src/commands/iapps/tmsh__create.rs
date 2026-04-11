//! `tmsh::create` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::create",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Mirrors the tmsh ``create`` command.",
            &["tmsh::create <component> <name> ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
