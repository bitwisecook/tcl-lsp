//! `add_wave` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_wave",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Add signals to the wave window (add wave).", &["add wave ?-position pos? ?-radix radix? ?-format format? ?-label label? ?-divider name? ?-group name"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
