//! `script::run` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::run",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Invoked when the tmsh ``run cli script`` command is issued.",
            &["script::run"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
