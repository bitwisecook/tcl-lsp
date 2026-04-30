//! `place` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Geometry manager for fixed or rubber-sheet placement.",
            &["place window option value ?option value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
