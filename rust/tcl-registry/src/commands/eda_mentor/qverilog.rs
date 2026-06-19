//! `qverilog` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "qverilog ?-sv? ?+define+name=val? ?-R? ?-c? file_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "qverilog",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Questa one-step Verilog compile and simulate.",
            &["qverilog ?-sv? ?+define+name=val? ?-R? ?-c? file_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
