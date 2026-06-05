//! `close_hw_manager` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "close_hw_manager",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close_hw_manager",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the hardware manager.",
            &["close_hw_manager"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
