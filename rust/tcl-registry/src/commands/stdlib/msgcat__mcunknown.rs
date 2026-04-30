//! `msgcat::mcunknown` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcunknown",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Called when a translation is not found; override for custom behaviour.",
            &["msgcat::mcunknown locale src-string ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
