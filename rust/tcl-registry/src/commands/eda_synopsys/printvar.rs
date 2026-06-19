//! `printvar` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "printvar ?variable_name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "printvar",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Print the value of an application variable.",
            &["printvar ?variable_name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
