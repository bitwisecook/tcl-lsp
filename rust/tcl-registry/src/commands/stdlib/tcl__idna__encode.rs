//! `tcl::idna::encode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::idna::encode",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Encode a hostname to IDNA (Internationalised Domain Names) format.",
            synopsis: &["tcl::idna::encode hostname"],
            snippet: "",
            source: "Tcl stdlib cookiejar package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("cookiejar"),
        ..CommandSpec::DEFAULT
    }
}
