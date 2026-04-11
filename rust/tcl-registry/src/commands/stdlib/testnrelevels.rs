//! `testnrelevels` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testnrelevels",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test NRE evaluation levels.",
            &["testnrelevels"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
