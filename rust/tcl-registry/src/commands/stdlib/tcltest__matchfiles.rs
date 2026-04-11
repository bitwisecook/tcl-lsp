//! `tcltest::matchFiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::matchFiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set matching file patterns.  Deprecated: use ``configure -file``.",
            &["tcltest::matchFiles ?patternList?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
