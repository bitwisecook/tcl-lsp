//! `tmsh::stateless` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::stateless ?enabled?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::stateless",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Modifies the behaviour of ``tmsh::create`` and ``tmsh::delete``.",
            &["tmsh::stateless ?enabled?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
