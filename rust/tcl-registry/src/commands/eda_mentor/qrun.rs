//! `qrun` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "qrun",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Questa unified compile/optimize/simulate command.",
            &["qrun ?-f file? ?-clean? ?-sv? ?-optimize? ?-top top? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
