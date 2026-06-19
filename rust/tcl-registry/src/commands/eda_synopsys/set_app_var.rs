//! `set_app_var` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_app_var variable_name value",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_app_var",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Set a Synopsys application variable.",
            &["set_app_var variable_name value"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
