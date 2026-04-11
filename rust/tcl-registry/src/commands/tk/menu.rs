//! `menu` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "menu",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a menu widget.",
            &["menu pathName ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
