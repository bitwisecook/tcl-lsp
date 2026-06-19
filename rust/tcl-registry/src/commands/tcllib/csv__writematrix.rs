//! `csv::writematrix` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::writematrix m chan ?sepChar? ?quoteChar?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::writematrix",
        dialects: None,
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet {
            summary: "Write a matrix object to a channel in CSV format.",
            synopsis: &["csv::writematrix m chan ?sepChar? ?quoteChar?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}
