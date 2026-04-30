//! `xrun` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "xrun",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Run Xcelium compilation and simulation in a single step.", &["xrun ?-sv? ?-access access_type? ?-define define? ?-top top_module? ?-input cmd_file? file_list"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
