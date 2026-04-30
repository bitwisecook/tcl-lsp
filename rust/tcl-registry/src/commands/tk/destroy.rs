//! `destroy` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "destroy",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Destroy one or more windows and all their descendants.",
            &["destroy ?window window ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
