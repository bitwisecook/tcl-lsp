//! `match` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "match",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Match compare points between reference and implementation.",
            &["match"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
