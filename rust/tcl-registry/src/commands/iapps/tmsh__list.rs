//! `tmsh::list` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::list ?component? ?name? ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::list",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Runs the ``list`` command using the specified arguments.",
            &["tmsh::list ?component? ?name? ?options?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
