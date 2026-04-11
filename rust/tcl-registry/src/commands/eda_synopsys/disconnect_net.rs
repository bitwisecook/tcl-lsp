//! `disconnect_net` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "disconnect_net",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Disconnect a net from pins/ports.",
            &["disconnect_net net_name port_pin_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
