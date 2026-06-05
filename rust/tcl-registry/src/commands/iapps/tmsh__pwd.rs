//! `tmsh::pwd` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::pwd",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::pwd",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current working directory.",
            &["tmsh::pwd"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
