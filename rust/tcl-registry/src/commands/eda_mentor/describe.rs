//! `describe` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "describe",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Show type and value information for a signal.",
            &["describe signal_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
