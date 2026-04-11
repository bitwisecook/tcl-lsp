//! `winfo` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "winfo",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Return window-related information.",
            &["winfo atom ?-displayof window? name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
