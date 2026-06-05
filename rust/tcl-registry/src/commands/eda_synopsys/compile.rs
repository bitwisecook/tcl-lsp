//! `compile` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "compile ?-map_effort effort? ?-area_effort effort? ?-incremental_mapping? ?-exact_map? ?-no_design_rule? ?-scan?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "compile",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Compile (synthesize) the current design.", &["compile ?-map_effort effort? ?-area_effort effort? ?-incremental_mapping? ?-exact_map? ?-no_design_r"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
