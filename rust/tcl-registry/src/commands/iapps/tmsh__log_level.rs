//! `tmsh::log_level` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::log_level",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies the default severity level.",
            &["tmsh::log_level ?level?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
