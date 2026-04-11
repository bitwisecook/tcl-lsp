//! `release` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "release",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Release a previously forced signal.",
            &["release signal_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
