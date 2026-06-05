//! `calibre_pex` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "calibre_pex ?-hier? ?-turbo? rule_file",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre_pex",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre PEX (parasitic extraction).",
            &["calibre_pex ?-hier? ?-turbo? rule_file"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
