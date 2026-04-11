//! `ip::is` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::is",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        return_type: Some(TclType::Boolean),
        hover: Some(HoverSnippet::brief(
            "Test whether a value is a valid IP address of the given class.",
            &["ip::is class address"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
