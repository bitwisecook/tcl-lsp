//! `uri::setQuirkOption` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::setQuirkOption",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Set a quirk option for URI processing.",
            &["uri::setQuirkOption option ?value...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
