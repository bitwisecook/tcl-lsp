//! `readers` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "readers",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Find readers of a signal.",
            &["readers signal_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
