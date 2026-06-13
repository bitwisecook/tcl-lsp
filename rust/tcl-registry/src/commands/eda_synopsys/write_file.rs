//! `write_file` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "write_file ?-format format? ?-hierarchy? ?-output file?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_file",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write design to file in specified format.",
            &["write_file ?-format format? ?-hierarchy? ?-output file?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
