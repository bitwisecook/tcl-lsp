//! `tmsh::get_name` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::get_name <object>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::get_name",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Returns the object identifier associated with the object.",
            &["tmsh::get_name <object>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
