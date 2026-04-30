//! `ttk::style` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::style",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Manipulate ttk styles and themes.",
            &["ttk::style subcommand ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
