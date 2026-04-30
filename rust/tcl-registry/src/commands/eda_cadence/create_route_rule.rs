//! `create_route_rule` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_route_rule",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a non-default routing rule.",
            &["create_route_rule -name name ?-widths list? ?-spacings list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
