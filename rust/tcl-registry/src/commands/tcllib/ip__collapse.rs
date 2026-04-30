//! `ip::collapse` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::collapse",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Collapse a list of IP addresses or subnets into the minimal set.",
            &["ip::collapse addressList"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
