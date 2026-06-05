//! `drivers` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "drivers signal_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "drivers",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Find drivers of a signal.",
            &["drivers signal_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
