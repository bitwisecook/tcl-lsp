//! `tcl::idna::decode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::idna::decode",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Decode a hostname from IDNA format to Unicode.",
            synopsis: &["tcl::idna::decode hostname"],
            snippet: "",
            source: "Tcl stdlib cookiejar package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("cookiejar"),
        ..CommandSpec::DEFAULT
    }
}
