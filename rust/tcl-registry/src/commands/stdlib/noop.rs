//! `noop` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "noop",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Do nothing (used for timing baselines).",
            &["noop"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
