//! `qwave` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "qwave",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Questa waveform viewer command.",
            &["qwave ?subcommand? ?args ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
