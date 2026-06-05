//! `report_gates` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_gates ?-power?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_gates",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report gate-level statistics.",
            &["report_gates ?-power?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
