//! `msgcat::mcflset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcflset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Set a translation for the locale of the current message file.",
            &["msgcat::mcflset src-string ?translate-string?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
