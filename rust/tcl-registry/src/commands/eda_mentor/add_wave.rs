//! `add_wave` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "add wave ?-position pos? ?-radix radix? ?-format format? ?-label label? ?-divider name? ?-group name? signal_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "add_wave",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add signals to the wave window (add wave).",
            &[
                "add wave ?-position pos? ?-radix radix? ?-format format? ?-label label? ?-divider name? ?-group name",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
