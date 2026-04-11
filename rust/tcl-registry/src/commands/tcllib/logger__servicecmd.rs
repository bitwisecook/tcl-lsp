//! `logger::servicecmd` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::servicecmd",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the command token for a named logger service.",
            &["logger::servicecmd service"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
