//! `textutil::strRepeat` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::strRepeat",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Repeat a string N times.",
            &["textutil::strRepeat char num"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
