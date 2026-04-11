//! `tcltest::matchDirectories` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::matchDirectories",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set matching directory patterns.  Deprecated: use ``configure -relateddir",
            &["tcltest::matchDirectories ?patternList?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
