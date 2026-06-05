//! `vdel` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vdel ?-lib library? ?-all? ?design_unit?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vdel",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Delete a compiled library or design unit.",
            &["vdel ?-lib library? ?-all? ?design_unit?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
