//! `qrun` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "qrun ?-f file? ?-clean? ?-sv? ?-optimize? ?-top top? file_list",
}];

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
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
