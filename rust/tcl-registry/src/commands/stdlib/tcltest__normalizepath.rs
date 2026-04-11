//! `tcltest::normalizePath` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::normalizePath",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Normalise a file path (8.4 compatibility shim for ``file normalize``).",
            &["tcltest::normalizePath pathVar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
