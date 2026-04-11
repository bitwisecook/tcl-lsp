//! `tmsh::log_dest` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::log_dest",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specifies where the system sends events.",
            &["tmsh::log_dest ?destination?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
