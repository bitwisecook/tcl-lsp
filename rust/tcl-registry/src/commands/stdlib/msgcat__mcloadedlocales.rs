//! `msgcat::mcloadedlocales` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcloadedlocales",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return or manage the list of locales that have been loaded.",
            &["msgcat::mcloadedlocales subcommand"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
