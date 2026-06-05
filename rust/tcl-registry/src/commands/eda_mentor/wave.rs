//! `wave` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "wave ?subcommand? ?args ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wave",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Waveform window command.",
            &["wave ?subcommand? ?args ...?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
