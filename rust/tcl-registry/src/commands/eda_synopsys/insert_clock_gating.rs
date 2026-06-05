//! `insert_clock_gating` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "insert_clock_gating ?-global?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "insert_clock_gating",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Insert clock gating logic.",
            &["insert_clock_gating ?-global?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
