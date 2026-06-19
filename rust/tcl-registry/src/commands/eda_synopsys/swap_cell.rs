//! `swap_cell` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "swap_cell cell_list lib_cell",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "swap_cell",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Swap a cell with a different library cell.",
            &["swap_cell cell_list lib_cell"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
