//! `bind` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bind",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Arrange for X event bindings on windows or tags.",
            &["bind tag"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
