//! `generate_target` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "generate_target target_type ?-force? objects",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "generate_target",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Generate IP output products.",
            &["generate_target target_type ?-force? objects"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
