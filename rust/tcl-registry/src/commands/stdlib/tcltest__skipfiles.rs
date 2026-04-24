//! `tcltest::skipFiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::skipFiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set file skip patterns.  Deprecated: use ``configure -notfile``.",
            &["tcltest::skipFiles ?patternList?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
