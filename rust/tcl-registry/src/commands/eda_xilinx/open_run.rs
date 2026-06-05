//! `open_run` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "open_run ?-name name? run_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_run",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Open a completed run in memory.",
            &["open_run ?-name name? run_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
