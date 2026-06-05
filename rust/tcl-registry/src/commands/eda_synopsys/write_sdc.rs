//! `write_sdc` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "write_sdc ?-nosplit? ?-version version? ?file_name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_sdc",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write SDC constraints to a file.",
            &["write_sdc ?-nosplit? ?-version version? ?file_name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
