//! `vlib` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vlib ?-type type? library_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vlib",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Create a design library directory.",
            &["vlib ?-type type? library_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
