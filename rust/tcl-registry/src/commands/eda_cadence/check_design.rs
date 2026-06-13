//! `check_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "check_design ?-all? ?-type type?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check the design for issues.",
            &["check_design ?-all? ?-type type?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
