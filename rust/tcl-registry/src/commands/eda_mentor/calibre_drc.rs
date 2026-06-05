//! `calibre_drc` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "calibre_drc ?-hier? ?-turbo? rule_file",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre_drc",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre DRC (design rule check).",
            &["calibre_drc ?-hier? ?-turbo? rule_file"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
