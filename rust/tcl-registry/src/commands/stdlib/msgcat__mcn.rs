//! `msgcat::mcn` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcn",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Translate a source string in a given namespace.",
            &["msgcat::mcn namespace src-string ?arg arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
