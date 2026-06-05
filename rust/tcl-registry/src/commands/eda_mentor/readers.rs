//! `readers` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "readers signal_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "readers",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Find readers of a signal.",
            &["readers signal_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
