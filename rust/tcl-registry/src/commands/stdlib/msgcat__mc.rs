//! `msgcat::mc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mc",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Translate a source string according to the current locale.",
            &["msgcat::mc src-string ?arg arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
