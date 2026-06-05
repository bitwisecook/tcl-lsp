//! `describe` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "describe signal_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "describe",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Show type and value information for a signal.",
            &["describe signal_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
