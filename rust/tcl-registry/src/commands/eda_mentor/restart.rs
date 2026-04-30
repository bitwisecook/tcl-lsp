//! `restart` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "restart",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Restart the current simulation.",
            &["restart ?-force? ?-nowave? ?-nolist? ?-nolog? ?-nobreakpoint? ?-nokill?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
