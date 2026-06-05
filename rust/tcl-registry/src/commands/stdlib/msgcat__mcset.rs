//! `msgcat::mcset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
hover: Some(HoverSnippet {
            summary: "Set the translation for a string in a given locale.",
            synopsis: &["msgcat::mcset locale src-string ?translate-string?"],
            snippet: "Sets the translation for *src-string* in the calling namespace.  If *translate-string* is omitted, *src-string* is used as the translation.",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
