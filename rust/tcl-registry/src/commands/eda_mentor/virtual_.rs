//! `virtual` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "virtual ?-install | -env env? ?signal | function? ?-name name?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "virtual",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create virtual signals or regions.",
            &["virtual ?-install | -env env? ?signal | function? ?-name name?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
