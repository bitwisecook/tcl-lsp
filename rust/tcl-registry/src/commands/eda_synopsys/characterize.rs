//! `characterize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "characterize",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Characterize a subdesign for context-dependent optimization.",
            &["characterize ?-constraints? instance_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
