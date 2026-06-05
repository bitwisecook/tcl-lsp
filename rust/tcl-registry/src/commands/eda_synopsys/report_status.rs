//! `report_status` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_status ?-verbose?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_status",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report verification status.",
            &["report_status ?-verbose?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
