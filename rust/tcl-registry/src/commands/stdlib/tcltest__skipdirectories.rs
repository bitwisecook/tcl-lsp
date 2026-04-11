//! `tcltest::skipDirectories` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::skipDirectories",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set directory skip patterns.  Deprecated: use ``configure -asidefromdir``",
            &["tcltest::skipDirectories ?patternList?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
