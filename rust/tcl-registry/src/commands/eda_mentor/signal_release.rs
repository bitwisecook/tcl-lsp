//! `signal_release` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "signal_release signal_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "signal_release",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Release a SignalSpy-forced signal.",
            &["signal_release signal_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
