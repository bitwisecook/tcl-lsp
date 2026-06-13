//! `execute_module` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "execute_module -tool tool_name ?-args arg_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "execute_module",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Execute a specific Quartus module.",
            &["execute_module -tool tool_name ?-args arg_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
