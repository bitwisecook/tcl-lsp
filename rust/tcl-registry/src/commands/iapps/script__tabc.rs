//! `script::tabc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "script::tabc",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "A script may provide ``script::tabc`` for tab completion.",
            &["script::tabc"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
