//! `check_timing` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "check_timing ?-file file?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_timing",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check for timing analysis issues.",
            &["check_timing ?-file file?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
