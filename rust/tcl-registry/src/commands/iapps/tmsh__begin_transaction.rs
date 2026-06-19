//! `tmsh::begin_transaction` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tmsh::begin_transaction",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::begin_transaction",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Begins an update transaction.",
            &["tmsh::begin_transaction"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
