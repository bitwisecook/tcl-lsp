//! `label` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "label",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a label widget.",
            &["label pathName ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
