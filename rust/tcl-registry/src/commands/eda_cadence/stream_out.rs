//! `stream_out` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "stream_out file_name ?-mapFile map?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "stream_out",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Stream out a GDSII file (legacy).",
            &["stream_out file_name ?-mapFile map?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
