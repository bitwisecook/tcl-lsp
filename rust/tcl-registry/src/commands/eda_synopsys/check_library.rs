//! `check_library` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "check_library ?library_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_library",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check libraries for consistency issues.",
            &["check_library ?library_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
