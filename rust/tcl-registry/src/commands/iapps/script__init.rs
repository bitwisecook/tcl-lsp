//! `script::init` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::init",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Called before ``script::run``, ``script::help``, and ``script::tabc``.",
            &["script::init"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
