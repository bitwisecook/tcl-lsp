//! `csv::split` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::split line ?sepChar? ?quoteChar?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::split",
        dialects: None,
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet {
            summary: "Split a CSV-formatted line into a list of values.",
            synopsis: &[
                "csv::split line ?sepChar? ?quoteChar?",
                "csv::split -alternate line ?sepChar? ?quoteChar?",
            ],
            snippet: "",
            source: "tcllib csv package",
            examples: "set fields [csv::split $line \",\"]",
            return_value: "A Tcl list of field values.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}
