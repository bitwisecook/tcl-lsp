//! `force` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "force",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Force a signal to a value.", &["force ?-freeze | -drive | -deposit? signal_name value ?time? ?-repeat period? ?-cancel period?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
