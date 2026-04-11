//! `ip::subtract` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::subtract",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Subtract address ranges from a list of hosts.",
            &["ip::subtract addressList"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
