//! `testcpuid` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testcpuid",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test CPUID instruction.",
            &["testcpuid"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
