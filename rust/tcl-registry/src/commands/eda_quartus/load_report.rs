//! `load_report` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "load_report ?revision_name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load_report",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Load a compilation report into memory.",
            &["load_report ?revision_name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
