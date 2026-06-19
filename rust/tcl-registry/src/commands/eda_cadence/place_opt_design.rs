//! `place_opt_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "place_opt_design ?-effort effort? ?-incremental?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place_opt_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform placement and optimization.",
            &["place_opt_design ?-effort effort? ?-incremental?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
