//! `vdel` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vdel",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Delete a compiled library or design unit.",
            &["vdel ?-lib library? ?-all? ?design_unit?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
