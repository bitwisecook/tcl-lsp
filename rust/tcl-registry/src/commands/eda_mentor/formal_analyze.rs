//! `formal_analyze` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "formal_analyze ?-property prop_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "formal_analyze",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Analyze formal verification results.",
            &["formal_analyze ?-property prop_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
