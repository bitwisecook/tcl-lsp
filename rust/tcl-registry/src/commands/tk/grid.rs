//! `grid` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "grid",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Geometry manager that arranges widgets in a grid.",
            &["grid slave ?slave ...? ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
