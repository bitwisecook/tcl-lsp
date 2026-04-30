//! `option` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "option",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Add or retrieve window options to or from the option database.",
            &["option add pattern value ?priority?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
