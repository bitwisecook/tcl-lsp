//! `route_opt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "route_opt",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform post-route optimization.",
            &["route_opt ?-effort high|medium|low?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
