//! `report_congestion` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_congestion ?-nosplit?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_congestion",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report routing congestion.",
            &["report_congestion ?-nosplit?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
