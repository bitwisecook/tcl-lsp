//! `tmsh::show` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::show ?component? ?name? ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::show",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Runs the ``show`` command using the specified arguments.",
            &["tmsh::show ?component? ?name? ?options?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
