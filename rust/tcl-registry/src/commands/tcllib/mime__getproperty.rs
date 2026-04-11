//! `mime::getproperty` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getproperty",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Return a property of a MIME part.",
            &["mime::getproperty token ?property?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
