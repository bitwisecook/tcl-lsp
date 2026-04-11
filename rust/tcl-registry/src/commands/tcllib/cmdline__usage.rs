//! `cmdline::usage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::usage",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Generate a usage string from an option specification.",
            &["cmdline::usage optlist ?usage?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
