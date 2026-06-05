//! `report_clock_gating` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_clock_gating ?-nosplit? ?-verbose?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_clock_gating",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report clock gating statistics.",
            &["report_clock_gating ?-nosplit? ?-verbose?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
