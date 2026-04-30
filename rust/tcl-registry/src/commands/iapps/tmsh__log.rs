//! `tmsh::log` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::log",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Logs the specified message.",
            &["tmsh::log <message>"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
