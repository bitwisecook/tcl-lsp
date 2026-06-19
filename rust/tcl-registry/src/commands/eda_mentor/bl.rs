//! `bl` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "bl",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bl",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("List all breakpoints.", &["bl"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
