//! `tcltest::restoreState` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::restoreState",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Restore interpreter state saved by ``saveState``.",
            &["tcltest::restoreState"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
