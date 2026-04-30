//! `uniquify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uniquify",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Make each instance of a subdesign unique.",
            &["uniquify ?-force? ?-dont_skip_empty_designs?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
