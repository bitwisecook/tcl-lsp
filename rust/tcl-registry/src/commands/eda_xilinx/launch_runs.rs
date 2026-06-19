//! `launch_runs` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "launch_runs ?-jobs n? ?-scripts_only? ?-to_step step? run_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "launch_runs",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Launch synthesis or implementation runs.",
            &["launch_runs ?-jobs n? ?-scripts_only? ?-to_step step? run_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
