//! `logger::import` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::import",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Import logger commands into the current namespace.",
            &["logger::import ?-all? ?-force? ?-prefix prefix? ?-namespace namespace? service"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
