//! `report_dp` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_dp ?-all?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_dp",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report datapath resources.",
            &["report_dp ?-all?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
