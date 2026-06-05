//! `msgcat::mclocale` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mclocale",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
hover: Some(HoverSnippet {
            summary: "Get or set the current locale for message catalogues.",
            synopsis: &["msgcat::mclocale ?newLocale?"],
            snippet: "Without arguments, returns the current locale.  With *newLocale*, sets the locale and returns the new value.",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
