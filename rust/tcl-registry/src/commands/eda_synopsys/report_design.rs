//! `report_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_design ?-nosplit? ?-verbose?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report design summary.",
            &["report_design ?-nosplit? ?-verbose?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
