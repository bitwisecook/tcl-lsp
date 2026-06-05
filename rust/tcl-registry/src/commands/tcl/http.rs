//! `http` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http",
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            "HTTP client implementation (package http)",
            &[],
            "Tcl",
        )),
        ..CommandSpec::DEFAULT
    }
}
