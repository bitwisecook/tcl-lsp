//! `insert_dft` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "insert_dft",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "insert_dft",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Insert DFT structures (scan chains).",
            &["insert_dft"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
