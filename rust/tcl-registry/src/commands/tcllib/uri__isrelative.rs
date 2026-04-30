//! `uri::isrelative` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::isrelative",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::Boolean),
        hover: Some(HoverSnippet::brief(
            "Test whether a URI is relative.",
            &["uri::isrelative uri"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
