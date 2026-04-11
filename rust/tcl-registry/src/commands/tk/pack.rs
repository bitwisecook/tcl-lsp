//! `pack` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pack",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Geometry manager that packs slaves around the edges of a cavity.",
            &["pack slave ?slave ...? ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
