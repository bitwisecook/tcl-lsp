//! `msgcat::mcflmset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcflmset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set multiple translations for the locale of the current message file.",
            &["msgcat::mcflmset src-trans-list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
