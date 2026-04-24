//! `tcltest::temporaryDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::temporaryDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the temporary directory.  Deprecated: use ``configure -tmpdir``.",
            &["tcltest::temporaryDirectory ?path?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
