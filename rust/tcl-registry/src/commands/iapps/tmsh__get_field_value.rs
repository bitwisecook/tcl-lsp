//! `tmsh::get_field_value` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::get_field_value <object> <field_name>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::get_field_value",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Retrieves the value of the field name.",
            &["tmsh::get_field_value <object> <field_name>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
