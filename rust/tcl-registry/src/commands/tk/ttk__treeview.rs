//! `ttk::treeview` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::treeview",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a themed hierarchical multicolumn data display widget.",
            &["ttk::treeview pathName ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
