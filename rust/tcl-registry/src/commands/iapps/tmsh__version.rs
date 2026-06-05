//! `tmsh::version` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::version",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::version",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Returns the version number of the BIG-IP system as a Tcl string.",
            &["tmsh::version"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
