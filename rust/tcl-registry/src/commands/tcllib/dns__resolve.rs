//! `dns::resolve` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::resolve",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Perform a DNS lookup.",
            &["dns::resolve name ?-type type? ?-class class? ?-server server? ?-timeout ms?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
