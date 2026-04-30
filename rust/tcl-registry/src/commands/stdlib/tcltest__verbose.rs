//! `tcltest::verbose` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::verbose",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set verbosity level.  Deprecated: use ``configure -verbose``.",
            &["tcltest::verbose ?level?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
