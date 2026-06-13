//! `close_project` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "close_project ?-quiet?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close_project",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the current project.",
            &["close_project ?-quiet?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
