//! `force` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "force ?-freeze | -drive | -deposit? signal_name value ?time? ?-repeat period? ?-cancel period?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "force",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Force a signal to a value.", &["force ?-freeze | -drive | -deposit? signal_name value ?time? ?-repeat period? ?-cancel period?"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
