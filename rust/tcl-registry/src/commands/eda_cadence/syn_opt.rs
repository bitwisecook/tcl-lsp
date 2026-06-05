//! `syn_opt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "syn_opt ?-effort effort? ?-incremental?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "syn_opt",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform post-map optimization.",
            &["syn_opt ?-effort effort? ?-incremental?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
