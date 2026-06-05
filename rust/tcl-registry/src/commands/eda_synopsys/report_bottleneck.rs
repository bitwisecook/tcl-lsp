//! `report_bottleneck` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_bottleneck ?-nosplit? ?-max_cells n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_bottleneck",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report timing bottleneck analysis.",
            &["report_bottleneck ?-nosplit? ?-max_cells n?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
