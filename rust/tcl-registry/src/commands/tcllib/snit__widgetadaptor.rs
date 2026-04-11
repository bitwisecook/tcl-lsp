//! `snit::widgetadaptor` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::widgetadaptor",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Define a snit widget adaptor that wraps an existing widget.",
            &["snit::widgetadaptor name definition"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
