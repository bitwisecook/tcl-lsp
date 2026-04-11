//! `msgcat::mcset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Set the translation for a string in a given locale.",
            &["msgcat::mcset locale src-string ?translate-string?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
