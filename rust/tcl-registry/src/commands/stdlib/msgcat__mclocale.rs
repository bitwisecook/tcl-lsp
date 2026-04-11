//! `msgcat::mclocale` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mclocale",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the current locale for message catalogues.",
            &["msgcat::mclocale ?newLocale?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
