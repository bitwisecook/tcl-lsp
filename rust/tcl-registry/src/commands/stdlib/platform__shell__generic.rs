//! `platform::shell::generic` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::shell::generic",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the generic platform identifier for a given Tcl shell.",
            &["platform::shell::generic shell"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
