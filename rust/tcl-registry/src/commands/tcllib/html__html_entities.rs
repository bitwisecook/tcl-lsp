//! `html::html_entities` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html::html_entities",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Replace special characters with HTML entities.",
            &["html::html_entities string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
