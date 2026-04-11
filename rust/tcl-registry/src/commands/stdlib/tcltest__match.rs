//! `tcltest::match` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::match",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set test match patterns.  Deprecated: use ``configure -match``.",
            &["tcltest::match ?patternList?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
