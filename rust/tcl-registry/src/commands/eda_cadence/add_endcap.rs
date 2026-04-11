//! `add_endcap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_endcap",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add endcap cells.",
            &["add_endcap ?-pre_endcap cell? ?-post_endcap cell?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
