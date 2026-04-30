//! `msgcat::mcpackagenamespaceget` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcpackagenamespaceget",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return the package namespace for the calling context.",
            &["msgcat::mcpackagenamespaceget"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
