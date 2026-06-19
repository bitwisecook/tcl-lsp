//! `vmap` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vmap ?-c? ?-del? library_name ?library_path?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vmap",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Map a logical library name to a directory.",
            &["vmap ?-c? ?-del? library_name ?library_path?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
