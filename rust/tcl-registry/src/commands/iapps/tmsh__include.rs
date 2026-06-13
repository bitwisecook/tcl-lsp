//! `tmsh::include` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::include <script_name>",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::include",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Runs the Tcl command ``eval`` on the specified script.",
            &["tmsh::include <script_name>"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
