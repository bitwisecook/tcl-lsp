//! `wait_on_run` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "wait_on_run ?-timeout minutes? run_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "wait_on_run",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Wait for a run to complete.",
            &["wait_on_run ?-timeout minutes? run_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
