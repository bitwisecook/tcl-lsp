//! `trap` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "trap ?command? ?signal ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trap",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Trap signals and execute a command when they occur.",
            &["trap ?command? ?signal ...?"],
            "F5",
        )),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}
