//! `when` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "when {condition} {action} ?-label label?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Define conditional actions during simulation.",
            &["when {condition} {action} ?-label label?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
