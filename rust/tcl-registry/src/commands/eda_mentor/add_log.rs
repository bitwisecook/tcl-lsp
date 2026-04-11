//! `add_log` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_log",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Log signals for waveform viewing (add log).",
            &["add log ?-r? signal_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
