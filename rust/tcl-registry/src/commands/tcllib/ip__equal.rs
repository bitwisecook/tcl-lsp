//! `ip::equal` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::equal",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Test if two IP addresses or subnets are equal.",
            &["ip::equal address1 address2"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
