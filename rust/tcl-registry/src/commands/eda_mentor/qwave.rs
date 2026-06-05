//! `qwave` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "qwave ?subcommand? ?args ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "qwave",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Questa waveform viewer command.",
            &["qwave ?subcommand? ?args ...?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
