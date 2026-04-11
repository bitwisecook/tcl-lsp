//! `msgcat::mcforgetpackage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcforgetpackage",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Remove all translations for the calling package.",
            &["msgcat::mcforgetpackage"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
