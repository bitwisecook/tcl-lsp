//! `parity` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "parity",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set or query whether parity is retained on spawned process output.",
            &["parity ?-d | -i spawn_id? ?value?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
