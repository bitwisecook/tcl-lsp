//! `pkg::create` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pkg::create",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(4),
        hover: Some(HoverSnippet::brief("Generate a ``package ifneeded`` script for a package.", &["pkg::create -name packageName -version packageVersion ?-load filespec? ?-source filespec?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
