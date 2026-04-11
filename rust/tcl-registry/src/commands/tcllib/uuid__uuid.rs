//! `uuid::uuid` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uuid::uuid",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Generate or manipulate UUIDs.",
            &["uuid::uuid generate"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
