//! `pkg_mkIndex` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg_mkIndex",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Build a ``pkgIndex.tcl`` file for one or more packages.",
            &["pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
