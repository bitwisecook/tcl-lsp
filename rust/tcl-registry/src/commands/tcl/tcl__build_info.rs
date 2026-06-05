//! `tcl::build-info` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::build-info",
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Return compile-time build metadata for the Tcl runtime.",
            &["tcl::build-info ?key?"],
            "Tcl tcl::build-info (internal)",
        )),
        ..CommandSpec::DEFAULT
    }
}
