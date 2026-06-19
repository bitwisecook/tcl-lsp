//! `write_do_lec` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "write_do_lec ?-revised_design design? ?-logicEquivalence? > file",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_do_lec",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a Conformal LEC do file.",
            &["write_do_lec ?-revised_design design? ?-logicEquivalence? > file"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
