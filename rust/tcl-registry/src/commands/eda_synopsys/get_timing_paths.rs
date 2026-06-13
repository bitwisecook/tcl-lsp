//! `get_timing_paths` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "get_timing_paths ?-from from_list? ?-through through_list? ?-to to_list? ?-delay_type type? ?-max_paths n? ?-nworst n? ?-slack_lesser_than value?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_timing_paths",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get timing paths as a collection.", &["get_timing_paths ?-from from_list? ?-through through_list? ?-to to_list? ?-delay_type type? ?-max_pa"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
