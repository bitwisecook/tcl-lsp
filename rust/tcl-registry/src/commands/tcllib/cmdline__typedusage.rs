//! `cmdline::typedUsage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedUsage",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Generate a usage string from a typed option specification.",
            &["cmdline::typedUsage optlist ?usage?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
