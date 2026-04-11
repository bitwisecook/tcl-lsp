//! `msgcat::mcpreferences` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcpreferences",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the ordered list of locale preferences.",
            &["msgcat::mcpreferences ?locale ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
