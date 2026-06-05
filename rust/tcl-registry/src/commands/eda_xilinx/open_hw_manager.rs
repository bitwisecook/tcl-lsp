//! `open_hw_manager` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "open_hw_manager",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_hw_manager",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Open the hardware manager.",
            &["open_hw_manager"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
