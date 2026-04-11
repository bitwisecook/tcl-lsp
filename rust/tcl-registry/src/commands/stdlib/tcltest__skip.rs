//! `tcltest::skip` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::skip",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set test skip patterns.  Deprecated: use ``configure -skip``.",
            &["tcltest::skip ?patternList?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
