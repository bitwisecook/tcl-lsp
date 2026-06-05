//! `check_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "check_design ?-summary? ?-no_warnings?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check the design for consistency problems.",
            &["check_design ?-summary? ?-no_warnings?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
