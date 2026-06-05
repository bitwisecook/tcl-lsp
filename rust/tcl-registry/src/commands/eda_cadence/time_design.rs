//! `time_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "time_design ?-pre_cts? ?-post_cts? ?-post_route? ?-hold? ?-report_prefix prefix?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "time_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform timing analysis.",
            &["time_design ?-pre_cts? ?-post_cts? ?-post_route? ?-hold? ?-report_prefix prefix?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
