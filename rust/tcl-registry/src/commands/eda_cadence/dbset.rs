//! `dbSet` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dbSet object_spec.attribute value",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dbSet",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set a design database object attribute (legacy).",
            &["dbSet object_spec.attribute value"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
