//! `uri::split` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::split",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Split a URI into its component parts.",
            &["uri::split url ?defaultScheme?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
