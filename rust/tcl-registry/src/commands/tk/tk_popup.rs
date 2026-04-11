//! `tk_popup` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_popup",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(3, 4),
        hover: Some(HoverSnippet::brief(
            "Post a pop-up menu at the given screen coordinates.",
            &["tk_popup menu x y ?entry?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
