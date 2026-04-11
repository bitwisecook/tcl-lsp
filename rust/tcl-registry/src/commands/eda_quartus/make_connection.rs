//! `make_connection` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "make_connection",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Make a connection in an ECO change.",
            &["make_connection -from source -to destination"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
