//! `create_port` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_port",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a new port in the design.",
            &["create_port port_name ?-direction direction?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
