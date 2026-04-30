//! `signal_force` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "signal_force",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Force a signal value using SignalSpy.",
            &["signal_force signal_name value ?time? ?-freeze | -drive | -deposit?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
