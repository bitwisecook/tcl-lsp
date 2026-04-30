//! `tcltest::saveState` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::saveState",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Save current interpreter state (procs and vars) for later restoration.",
            &["tcltest::saveState"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
