//! `tcltest::limitConstraints` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::limitConstraints",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set constraint limiting.  Deprecated: use ``configure -limitconstraints``",
            &["tcltest::limitConstraints ?boolean?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
