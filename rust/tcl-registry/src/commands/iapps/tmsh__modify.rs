//! `tmsh::modify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::modify",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Runs the ``modify`` command using the specified arguments.",
            &["tmsh::modify <component> <name> ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
