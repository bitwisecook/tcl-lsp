//! `script::help` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::help",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Scripts can provide the ``script::help`` procedure for help text.",
            &["script::help"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
