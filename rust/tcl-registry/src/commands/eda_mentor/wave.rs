//! `wave` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wave",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Waveform window command.",
            &["wave ?subcommand? ?args ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
