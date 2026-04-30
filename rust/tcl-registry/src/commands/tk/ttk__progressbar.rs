//! `ttk::progressbar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::progressbar",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed progress indicator widget.",
            &["ttk::progressbar pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
