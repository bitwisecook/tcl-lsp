//! `msgcat::mcload` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcload",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Load message catalogue files from a directory.",
            &["msgcat::mcload dirname"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
