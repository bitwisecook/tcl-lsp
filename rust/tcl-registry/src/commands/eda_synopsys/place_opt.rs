//! `place_opt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place_opt",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform placement optimization.",
            &["place_opt ?-effort high|medium|low?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
