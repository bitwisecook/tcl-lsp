//! `msgcat::mcmset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcmset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Set translations for multiple strings in a given locale.",
            &["msgcat::mcmset locale src-trans-list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
