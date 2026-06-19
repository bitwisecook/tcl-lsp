//! `tmsh::display_threshold` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::display_threshold ?value?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::display_threshold",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Re-enables a display-threshold in the script.",
            &["tmsh::display_threshold ?value?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
