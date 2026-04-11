//! `testnreunwind` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testnreunwind",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test NRE stack unwinding.",
            &["testnreunwind"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
