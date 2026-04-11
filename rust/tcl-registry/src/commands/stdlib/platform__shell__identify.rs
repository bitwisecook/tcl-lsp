//! `platform::shell::identify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::shell::identify",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the platform identifier for a given Tcl shell.",
            &["platform::shell::identify shell"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
