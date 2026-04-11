//! `tk` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Manipulate Tk internal state.",
            &["tk appname ?newName?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
