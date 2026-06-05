//! `tmsh::cd` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::cd <directory>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::cd",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Changes the current working directory.",
            &["tmsh::cd <directory>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
