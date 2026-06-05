//! `calibre_lvs` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "calibre_lvs ?-hier? ?-turbo? rule_file",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre_lvs",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre LVS (layout vs schematic).",
            &["calibre_lvs ?-hier? ?-turbo? rule_file"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
