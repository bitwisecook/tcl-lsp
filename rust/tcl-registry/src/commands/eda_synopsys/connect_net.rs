//! `connect_net` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect_net",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Connect a net to pins/ports.",
            &["connect_net net_name port_pin_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
