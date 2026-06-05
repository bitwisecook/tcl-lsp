//! `report_timing` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_timing ?-from from? ?-to to? ?-through through? ?-max_paths n? ?-nworst n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_timing",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing paths.",
            &["report_timing ?-from from? ?-to to? ?-through through? ?-max_paths n? ?-nworst n?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
