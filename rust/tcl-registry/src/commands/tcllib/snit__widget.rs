//! `snit::widget` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::widget",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Define a new snit megawidget type.",
            &["snit::widget name definition"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
