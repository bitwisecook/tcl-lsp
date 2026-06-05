//! `tmsh::get_config` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::get_config <component> ?name? ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::get_config",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Returns a list of configuration items as Tcl objects.",
            &["tmsh::get_config <component> ?name? ?options?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
