//! `execute_flow` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "execute_flow -compile | -analysis_and_synthesis | -fitter | -assembler | -timing_analyzer | -eda_netlist_writer | -signaltap | -export_database",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "execute_flow",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Execute a Quartus compilation flow.",
            &[
                "execute_flow -compile | -analysis_and_synthesis | -fitter | -assembler | -timing_analyzer | -eda_net",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
