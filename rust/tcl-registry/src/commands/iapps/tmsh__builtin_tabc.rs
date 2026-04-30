//! `tmsh::builtin_tabc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::builtin_tabc",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Displays the same tab completion results as a built-in command.",
            &["tmsh::builtin_tabc ?args?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
