//! `dns::configure` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::configure",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set DNS client configuration options.",
            &["dns::configure ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
