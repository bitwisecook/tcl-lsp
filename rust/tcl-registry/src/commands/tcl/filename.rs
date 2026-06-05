//! `filename` command (name-parity reconcile, GAP-d).
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "filename",
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief("File name conventions", &[], "Tcl")),
        ..CommandSpec::DEFAULT
    }
}
