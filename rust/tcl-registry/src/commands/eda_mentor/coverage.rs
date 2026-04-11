//! `coverage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coverage",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Configure or report code coverage.",
            &["coverage ?save | report | reload | clear? ?-file file? ?-directive? ?-comments?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
