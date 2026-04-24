//! `get_names` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_names",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get signal names from the design.",
            &["get_names ?-filter filter? ?-node_type type? ?-observable_type type?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
