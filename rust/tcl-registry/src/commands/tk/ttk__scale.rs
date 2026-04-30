//! `ttk::scale` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::scale",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed scale (slider) widget.",
            &["ttk::scale pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
