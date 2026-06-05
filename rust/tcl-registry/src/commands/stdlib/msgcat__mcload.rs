//! `msgcat::mcload` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcload",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
hover: Some(HoverSnippet {
            summary: "Load message catalogue files from a directory.",
            synopsis: &["msgcat::mcload dirname"],
            snippet: "Searches *dirname* for files matching the current locale preferences (e.g. ``en_gb.msg``, ``en.msg``, ``ROOT.msg``) and sources them.",
            source: "Tcl stdlib msgcat package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("msgcat"),
        ..CommandSpec::DEFAULT
    }
}
