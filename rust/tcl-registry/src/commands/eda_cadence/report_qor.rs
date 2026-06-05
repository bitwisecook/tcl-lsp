//! `report_qor` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_qor ?-summary?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_qor",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report quality-of-results summary.",
            &["report_qor ?-summary?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
