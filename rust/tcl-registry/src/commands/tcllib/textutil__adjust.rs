//! `textutil::adjust` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::adjust",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Adjust a text block to a given line length.", &["textutil::adjust string ?-length num? ?-justify left|right|center|plain? ?-hyphenate bool? ?-full bo"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
