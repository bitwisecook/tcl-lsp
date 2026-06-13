//! `tmsh::reset_stats` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::reset_stats ?component? ?name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::reset_stats",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Runs the ``reset-stats`` command using the specified arguments.",
            &["tmsh::reset_stats ?component? ?name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
