//! `tcltest::testsDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::testsDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the tests directory.  Deprecated: use ``configure -testdir``.",
            &["tcltest::testsDirectory ?path?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
