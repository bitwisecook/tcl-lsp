//! `set_parameter` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_parameter -name name -to instance value",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_parameter",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set a parameter on a design entity.",
            &["set_parameter -name name -to instance value"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
