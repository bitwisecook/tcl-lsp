//! `ccopt_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ccopt_design ?-cts? ?-post_cts_opt?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ccopt_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform clock tree synthesis (CTS).",
            &["ccopt_design ?-cts? ?-post_cts_opt?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
