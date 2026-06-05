//! `create_run` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "create_run -flow flow ?-strategy strategy? ?-constrset constrset? ?-parent_run parent? run_name" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_run",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Create a new run.", &["create_run -flow flow ?-strategy strategy? ?-constrset constrset? ?-parent_run parent? run_name"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
