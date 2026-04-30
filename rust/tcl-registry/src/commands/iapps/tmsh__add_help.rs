//! `tmsh::add_help` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::add_help",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Displays context-sensitive help when the user types ``?``.",
            &["tmsh::add_help <help_data>"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
