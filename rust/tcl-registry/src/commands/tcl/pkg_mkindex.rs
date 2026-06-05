//! `pkg_mkindex` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg_mkindex",
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Build pkgIndex.tcl for a directory of packages",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
