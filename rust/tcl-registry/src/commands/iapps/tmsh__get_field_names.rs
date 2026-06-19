//! `tmsh::get_field_names` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::get_field_names <object>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::get_field_names",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Returns a list of field names present in an object.",
            &["tmsh::get_field_names <object>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
