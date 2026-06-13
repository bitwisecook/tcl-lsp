//! `be` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "be ?breakpoint_id | -all?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "be",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable breakpoints.",
            &["be ?breakpoint_id | -all?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
